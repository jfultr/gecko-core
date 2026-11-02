//! Toy in-memory Core for the spike: request dedup, executor event identity and arbitration (P5),
//! Core-numbered snapshots, atomic watcher attach. Publication to Zenoh happens while holding the
//! Core lock so wire order equals `sequence` order.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;
use zenoh::Session;

use crate::wire::*;

pub const PREFIX: &str = "gecko/g1/op";
/// Liveliness token the Gecko session declares; observers watch it to detect reconnection.
pub const ALIVE_KEY: &str = "gecko/g1/alive";

pub fn report_key(id: Uuid) -> String {
    format!("{PREFIX}/{id}/report")
}
pub fn observe_key(id: Uuid) -> String {
    format!("{PREFIX}/{id}/observe")
}
pub fn snapshot_key(id: Uuid) -> String {
    format!("{PREFIX}/{id}/snapshot")
}

struct OpRecord {
    operation: OperationRef,
    target: Option<String>,
    token: Uuid,
    state: State,
    progress: Progress,
    sequence: u64,
    last_event_seq: u64,
    /// Accepted events by `event_seq`: content and the snapshot sequence current after applying it.
    events: HashMap<u64, (EventKind, u64)>,
    /// Retained report: only the latest full snapshot. `None` after `drop_report`.
    latest: Option<Snapshot>,
    watchers: Vec<mpsc::Sender<Snapshot>>,
}

impl OpRecord {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            operation: self.operation.clone(),
            sequence: self.sequence,
            reported_at_unix_ms: now_ms(),
            state: self.state,
            progress: self.progress.clone(),
            target: self.target.clone(),
        }
    }
}

#[derive(Default)]
struct Inner {
    requests: HashMap<(String, Uuid), (Uuid, String)>,
    ops: HashMap<Uuid, OpRecord>,
    tokens: HashMap<Uuid, Uuid>,
}

pub enum Attach {
    NotFound,
    Unavailable,
    /// `rx` is `None` when `latest` is already terminal.
    Attached { latest: Snapshot, rx: Option<mpsc::Receiver<Snapshot>> },
}

#[derive(Clone)]
pub struct Core {
    inner: Arc<Mutex<Inner>>,
    session: Session,
    /// When false, snapshots are recorded but not put on the wire (used to emulate lost publications).
    pub publish: Arc<std::sync::atomic::AtomicBool>,
}

impl Core {
    pub fn new(session: Session) -> Self {
        Self { inner: Default::default(), session, publish: Arc::new(true.into()) }
    }

    pub async fn start(&self, caller: &str, req: &StartRequest) -> StartResponse {
        if req.operation_type.is_empty() {
            return StartResponse::Rejected { code: RejectCode::InvalidInput, message: "empty type".into() };
        }
        if req.operation_type != "test.sleep" {
            return StartResponse::Rejected {
                code: RejectCode::UnknownOperationType,
                message: format!("unknown type {}", req.operation_type),
            };
        }
        let norm = normalized_content(req);
        let mut g = self.inner.lock().await;
        if let Some((op, prev)) = g.requests.get(&(caller.to_string(), req.request_id)) {
            if *prev == norm {
                let op = *op;
                return accepted(op, true);
            }
            return StartResponse::Rejected {
                code: RejectCode::RequestIdConflict,
                message: "request_id reused with different content".into(),
            };
        }
        // Request mapping, execution record and first snapshot commit together (P6).
        let op = Uuid::new_v4();
        let token = Uuid::new_v4();
        let mut rec = OpRecord {
            operation: OperationRef { id: op, kind: req.operation_type.clone(), parent_id: None },
            target: req.target.clone(),
            token,
            state: State::Pending,
            progress: Progress::stage("accepted"),
            sequence: 1,
            last_event_seq: 0,
            events: HashMap::new(),
            latest: None,
            watchers: vec![],
        };
        rec.latest = Some(rec.snapshot());
        g.requests.insert((caller.to_string(), req.request_id), (op, norm));
        g.ops.insert(op, rec);
        g.tokens.insert(token, op);
        drop(g);
        accepted(op, false)
    }

    /// Execution token handed to the (test) executor with the work.
    pub async fn token(&self, op: Uuid) -> Uuid {
        self.inner.lock().await.ops[&op].token
    }

    /// Validates and applies one executor notification (P5 / ST3).
    pub async fn notify(&self, ev: &ExecutorEvent) -> NotifyOutcome {
        let mut g = self.inner.lock().await;
        let Some(op) = g.tokens.get(&ev.token).copied() else { return NotifyOutcome::UnknownToken };
        let rec = g.ops.get_mut(&op).unwrap();

        if ev.event_seq <= rec.last_event_seq {
            return match rec.events.get(&ev.event_seq) {
                Some((k, seq)) if *k == ev.event => NotifyOutcome::Duplicate { sequence: *seq },
                _ => NotifyOutcome::Stale,
            };
        }
        if ev.event_seq != rec.last_event_seq + 1 {
            return NotifyOutcome::OutOfOrder { expected: rec.last_event_seq + 1 };
        }

        use State::*;
        let next: Option<(State, Progress)> = match (&ev.event, rec.state) {
            (EventKind::Ack, Pending | Unknown) => Some((Running, Progress::stage("started"))),
            (EventKind::Ack, _) => None, // repeated or late ack: accepted, nothing changes
            (EventKind::Progress { progress }, Running | Unknown) => Some((Running, progress.clone())),
            (EventKind::Progress { .. }, from) => return NotifyOutcome::InvalidTransition { from },
            (EventKind::Terminal { state, .. }, from) if from.is_terminal() => {
                if *state == from {
                    None // enrichment of a terminal report is open (ST3)
                } else {
                    return NotifyOutcome::ConflictingTerminal { recorded: from };
                }
            }
            // No Cancel in 0.1.0, so `cancelled` cannot be confirmed.
            (EventKind::Terminal { state: Cancelled, .. }, from) => return NotifyOutcome::InvalidTransition { from },
            (EventKind::Terminal { state, progress }, _) if state.is_terminal() => Some((*state, progress.clone())),
            (EventKind::Terminal { .. }, from) => return NotifyOutcome::InvalidTransition { from },
        };

        rec.last_event_seq = ev.event_seq;
        let Some((state, progress)) = next else {
            rec.events.insert(ev.event_seq, (ev.event.clone(), rec.sequence));
            return NotifyOutcome::NoChange { sequence: rec.sequence };
        };
        rec.state = state;
        rec.progress = progress;
        rec.sequence += 1;
        rec.events.insert(ev.event_seq, (ev.event.clone(), rec.sequence));
        let snap = rec.snapshot();
        self.record_and_publish(op, rec, snap).await
    }

    /// Core-originated transition: handoff may have reached the executor but no ack arrived.
    pub async fn handoff_lost(&self, op: Uuid) -> NotifyOutcome {
        let mut g = self.inner.lock().await;
        let rec = g.ops.get_mut(&op).unwrap();
        if rec.state != State::Pending {
            return NotifyOutcome::InvalidTransition { from: rec.state };
        }
        rec.state = State::Unknown;
        rec.progress = Progress::stage("handoff_unconfirmed");
        rec.sequence += 1;
        let snap = rec.snapshot();
        self.record_and_publish(op, rec, snap).await
    }

    async fn record_and_publish(&self, op: Uuid, rec: &mut OpRecord, snap: Snapshot) -> NotifyOutcome {
        if rec.latest.is_some() {
            rec.latest = Some(snap.clone());
        }
        // A full or closed watcher is dropped: its stream ends with `lagged` and the client resumes.
        rec.watchers.retain(|w| w.try_send(snap.clone()).is_ok());
        if self.publish.load(std::sync::atomic::Ordering::SeqCst) {
            let bytes = serde_json::to_vec(&snap).unwrap();
            self.session.put(snapshot_key(op), bytes).await.unwrap();
        }
        NotifyOutcome::Applied { sequence: snap.sequence }
    }

    /// Test-executor helper: sends the next contiguous event that moves `op` towards `state`
    /// (ack from pending, progress while running, terminal). Each call yields one snapshot.
    pub async fn advance(&self, op: Uuid, state: State, stage: &str) -> u64 {
        let (token, event_seq, cur) = {
            let g = self.inner.lock().await;
            let r = &g.ops[&op];
            (r.token, r.last_event_seq + 1, r.state)
        };
        let event = match (state, cur) {
            (State::Running, State::Pending) => EventKind::Ack,
            (State::Running, _) => EventKind::Progress { progress: Progress::stage(stage) },
            (s, _) => EventKind::Terminal { state: s, progress: Progress::stage(stage) },
        };
        match self.notify(&ExecutorEvent { token, event_seq, event }).await {
            NotifyOutcome::Applied { sequence } => sequence,
            other => panic!("advance: {other:?}"),
        }
    }

    pub async fn get(&self, op: Uuid) -> GetResponse {
        let g = self.inner.lock().await;
        match g.ops.get(&op) {
            Some(r) => match &r.latest {
                Some(s) => GetResponse::Report { snapshot: s.clone() },
                None => GetResponse::Unavailable { operation_id: op },
            },
            None => GetResponse::NotFound,
        }
    }

    /// Emulates report retention expiry: the request mapping and execution record stay.
    pub async fn drop_report(&self, op: Uuid) {
        self.inner.lock().await.ops.get_mut(&op).unwrap().latest = None;
    }

    /// Atomic attach: read the latest snapshot and register for later ones under one lock.
    pub async fn attach(&self, op: Uuid, cap: usize) -> Attach {
        let mut g = self.inner.lock().await;
        let Some(rec) = g.ops.get_mut(&op) else { return Attach::NotFound };
        let Some(latest) = rec.latest.clone() else { return Attach::Unavailable };
        if latest.state.is_terminal() {
            return Attach::Attached { latest, rx: None };
        }
        let (tx, rx) = mpsc::channel(cap);
        rec.watchers.push(tx);
        Attach::Attached { latest, rx: Some(rx) }
    }
}

fn accepted(op: Uuid, retried: bool) -> StartResponse {
    StartResponse::Accepted { operation_id: op, report_key: report_key(op), observe_key: observe_key(op), retried }
}
