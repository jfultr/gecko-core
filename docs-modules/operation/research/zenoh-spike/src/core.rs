//! Toy in-memory Core for the spike: request dedup, Core-numbered snapshots, atomic watcher attach.
//! Publication to Zenoh happens while holding the Core lock so wire order equals `sequence` order.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;
use zenoh::Session;

use crate::wire::*;

pub const PREFIX: &str = "gecko/g1/op";

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
    snapshots: Vec<Snapshot>,
    watchers: Vec<mpsc::Sender<Snapshot>>,
}

#[derive(Default)]
struct Inner {
    requests: HashMap<(String, Uuid), (Uuid, String)>,
    ops: HashMap<Uuid, OpRecord>,
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
        Self {
            inner: Default::default(),
            session,
            publish: Arc::new(true.into()),
        }
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
        let op = Uuid::new_v4();
        g.requests.insert((caller.to_string(), req.request_id), (op, norm));
        let first = Snapshot {
            operation_id: op,
            operation_type: req.operation_type.clone(),
            sequence: 1,
            reported_at_unix_ms: now_ms(),
            state: State::Pending,
            stage: "accepted".into(),
        };
        g.ops.insert(op, OpRecord { snapshots: vec![first.clone()], watchers: vec![] });
        drop(g);
        accepted(op, false)
    }

    /// Emulates an accepted executor notification: Core numbers and publishes a new full snapshot.
    pub async fn advance(&self, op: Uuid, state: State, stage: &str) -> u64 {
        let mut g = self.inner.lock().await;
        let rec = g.ops.get_mut(&op).expect("op");
        let last = rec.snapshots.last().unwrap().clone();
        let snap = Snapshot {
            sequence: last.sequence + 1,
            reported_at_unix_ms: now_ms(),
            state,
            stage: stage.into(),
            ..last
        };
        rec.snapshots.push(snap.clone());
        // A full or closed watcher is dropped: its stream ends and the client resumes with its cursor.
        rec.watchers.retain(|w| w.try_send(snap.clone()).is_ok());
        if self.publish.load(std::sync::atomic::Ordering::SeqCst) {
            let bytes = serde_json::to_vec(&snap).unwrap();
            self.session.put(snapshot_key(op), bytes).await.unwrap();
        }
        snap.sequence
    }

    pub async fn get(&self, op: Uuid) -> GetResponse {
        let g = self.inner.lock().await;
        match g.ops.get(&op) {
            Some(r) => GetResponse::Report { snapshot: r.snapshots.last().unwrap().clone() },
            None => GetResponse::NotFound,
        }
    }

    /// Atomic attach: read the latest snapshot and register for later ones under one lock.
    pub async fn attach(&self, op: Uuid, cap: usize) -> Option<(Snapshot, mpsc::Receiver<Snapshot>)> {
        let mut g = self.inner.lock().await;
        let rec = g.ops.get_mut(&op)?;
        let latest = rec.snapshots.last().unwrap().clone();
        let (tx, rx) = mpsc::channel(cap);
        if !latest.state.is_terminal() {
            rec.watchers.push(tx);
        }
        Some((latest, rx))
    }
}

fn accepted(op: Uuid, retried: bool) -> StartResponse {
    StartResponse::Accepted { operation_id: op, report_key: report_key(op), observe_key: observe_key(op), retried }
}
