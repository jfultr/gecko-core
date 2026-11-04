//! Zenoh 1.10.2 spike for Operation 0.1.0 (GKO-60). Run: `cargo run --release -- <scenario>|all`.
//!
//! Topology: one "gecko" session in peer mode listening on tcp/127.0.0.1:<free port>, one caller
//! session in client mode connecting to it (directly or through an in-process TCP proxy that can be
//! cut). Multicast/gossip scouting disabled. Both sessions live in one process and one tokio runtime.

mod core;
mod wire;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;
use uuid::Uuid;
use zenoh::Session;
use zenoh::handlers::{FifoChannel, RingChannel};
use zenoh::query::{ConsolidationMode, QueryTarget};

use crate::core::*;
use crate::wire::*;

const CALLER: &str = "trusted-test-caller";

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

fn base_config(mode: &str) -> zenoh::Config {
    let mut c = zenoh::Config::default();
    c.insert_json5("mode", &format!("\"{mode}\"")).unwrap();
    c.insert_json5("scouting/multicast/enabled", "false").unwrap();
    c.insert_json5("scouting/gossip/enabled", "false").unwrap();
    c
}

async fn open_gecko(port: u16) -> Session {
    let mut c = base_config("peer");
    c.insert_json5("listen/endpoints", &format!("[\"tcp/127.0.0.1:{port}\"]")).unwrap();
    zenoh::open(c).await.unwrap()
}

async fn open_caller(port: u16) -> Session {
    let mut c = base_config("client");
    c.insert_json5("connect/endpoints", &format!("[\"tcp/127.0.0.1:{port}\"]")).unwrap();
    zenoh::open(c).await.unwrap()
}

/// Serves Start, Get and streaming Observe for `core` on `session`, and declares the Gecko
/// liveliness token. All typed outcomes are `Ok` replies; `reply_err` is not used (choice 3).
async fn serve(session: &Session, core: Core, start_delay: Duration) {
    let token = session.liveliness().declare_token(ALIVE_KEY).await.unwrap();
    std::mem::forget(token); // lives as long as the session

    let q = session.declare_queryable(format!("{PREFIX}/start")).await.unwrap();
    let c = core.clone();
    tokio::spawn(async move {
        while let Ok(query) = q.recv_async().await {
            let c = c.clone();
            tokio::spawn(async move {
                let parsed = match query.payload() {
                    None => Err("missing payload".to_string()),
                    Some(p) => serde_json::from_slice::<StartRequest>(&p.to_bytes()).map_err(|e| format!("malformed: {e}")),
                };
                let resp = match parsed {
                    Ok(req) => c.start(CALLER, &req).await,
                    Err(message) => StartResponse::Rejected { code: RejectCode::InvalidInput, message },
                };
                // Acceptance is committed before the delayed reply: a lost/late reply leaves an accepted op.
                tokio::time::sleep(start_delay).await;
                let r = query.reply(query.key_expr().clone(), serde_json::to_vec(&resp).unwrap()).await;
                if let Err(e) = r {
                    println!("    [server] late Start reply failed: {e}");
                }
            });
        }
    });

    let q = session.declare_queryable(format!("{PREFIX}/*/report")).await.unwrap();
    let c = core.clone();
    tokio::spawn(async move {
        while let Ok(query) = q.recv_async().await {
            let resp = match op_id_from_key(query.key_expr().as_str()) {
                Some(id) => c.get(id).await,
                None => GetResponse::NotFound,
            };
            query.reply(query.key_expr().clone(), serde_json::to_vec(&resp).unwrap()).await.unwrap();
        }
    });

    let q = session.declare_queryable(format!("{PREFIX}/*/observe")).await.unwrap();
    let c = core.clone();
    tokio::spawn(async move {
        while let Ok(query) = q.recv_async().await {
            let c = c.clone();
            tokio::spawn(async move { serve_observe(&c, query).await });
        }
    });
}

/// One streaming Observe query. Selector parameters (Zenoh separates them with `;`, not `&`):
/// `after=<seq>` (client cursor), `cap=<n>` (test only: Core watcher capacity, default 8),
/// `lease_ms=<n>` (test only, default 5000). An unparsable cursor is rejected, not ignored.
async fn serve_observe(c: &Core, query: zenoh::query::Query) {
    let p = query.parameters();
    let after: Option<u64> = match p.get("after").map(|v| v.parse::<u64>()) {
        None => None,
        Some(Ok(v)) => Some(v),
        Some(Err(e)) => {
            let f = ObserveFrame::Rejected { code: RejectCode::InvalidInput, message: format!("after: {e}") };
            return drop(query.reply(query.key_expr().clone(), serde_json::to_vec(&f).unwrap()).await);
        }
    };
    let cap: usize = p.get("cap").and_then(|v| v.parse().ok()).unwrap_or(8);
    let lease_ms: u64 = p.get("lease_ms").and_then(|v| v.parse().ok()).unwrap_or(5000);
    let key = query.key_expr().clone();
    let send = |f: ObserveFrame| {
        let (q, k) = (&query, key.clone());
        async move { q.reply(k, serde_json::to_vec(&f).unwrap()).await }
    };
    let attach = match op_id_from_key(key.as_str()) {
        Some(id) => c.attach(id, cap).await,
        None => Attach::NotFound,
    };
    let (latest, rx) = match attach {
        Attach::NotFound => return drop(send(ObserveFrame::NotFound).await),
        Attach::Unavailable => {
            let id = op_id_from_key(key.as_str()).unwrap();
            return drop(send(ObserveFrame::Unavailable { operation_id: id }).await);
        }
        Attach::Attached { latest, rx } => (latest, rx),
    };
    let mut last = after.unwrap_or(0);
    if latest.sequence > last {
        let gap_before = last > 0 && latest.sequence > last + 1;
        last = latest.sequence;
        if send(ObserveFrame::Snapshot { snapshot: latest, gap_before }).await.is_err() {
            return;
        }
    }
    // Already terminal: the cursor may already be at the final snapshot; end as terminal either way.
    let Some(mut rx) = rx else {
        return drop(send(ObserveFrame::End { reason: EndReason::Terminal }).await);
    };
    let lease = tokio::time::sleep(Duration::from_millis(lease_ms));
    tokio::pin!(lease);
    loop {
        tokio::select! {
            s = rx.recv() => match s {
                Some(s) => {
                    if s.sequence <= last { continue; }
                    let gap_before = s.sequence > last + 1;
                    last = s.sequence;
                    let term = s.state.is_terminal();
                    if send(ObserveFrame::Snapshot { snapshot: s, gap_before }).await.is_err() { return; }
                    if term { return drop(send(ObserveFrame::End { reason: EndReason::Terminal }).await); }
                }
                None => return drop(send(ObserveFrame::End { reason: EndReason::Lagged }).await),
            },
            _ = &mut lease => return drop(send(ObserveFrame::End { reason: EndReason::Lease }).await),
        }
    }
}

fn op_id_from_key(k: &str) -> Option<Uuid> {
    k.strip_prefix(&format!("{PREFIX}/"))?.split('/').next()?.parse().ok()
}

async fn setup(start_delay: Duration) -> (Session, Session, Core, u16) {
    let port = free_port();
    let g = open_gecko(port).await;
    let core = Core::new(g.clone());
    serve(&g, core.clone(), start_delay).await;
    let c = open_caller(port).await;
    (g, c, core, port)
}

fn req(id: Uuid, params: serde_json::Value) -> StartRequest {
    StartRequest { request_id: id, operation_type: "test.sleep".into(), target: Some("local".into()), params, trace: None }
}

/// Raw Start: returns each reply as Ok(json)/Err(error payload), plus elapsed time.
async fn raw_start(c: &Session, payload: Vec<u8>, timeout: Duration) -> (Vec<Result<String, String>>, Duration) {
    let t = Instant::now();
    let replies = c.get(format!("{PREFIX}/start")).payload(payload).timeout(timeout).await.unwrap();
    let mut out = vec![];
    while let Ok(r) = replies.recv_async().await {
        out.push(match r.result() {
            Ok(s) => Ok(String::from_utf8_lossy(&s.payload().to_bytes()).into_owned()),
            Err(e) => Err(format!("{} [encoding {}]", String::from_utf8_lossy(&e.payload().to_bytes()), e.encoding())),
        });
    }
    (out, t.elapsed())
}

async fn start(c: &Session, r: &StartRequest) -> Option<StartResponse> {
    let (out, _) = raw_start(c, serde_json::to_vec(r).unwrap(), Duration::from_secs(2)).await;
    out.into_iter().next().and_then(|x| x.ok()).map(|s| serde_json::from_str(&s).unwrap())
}

fn op_of(r: &Option<StartResponse>) -> Uuid {
    match r {
        Some(StartResponse::Accepted { operation_id, .. }) => *operation_id,
        other => panic!("not accepted: {other:?}"),
    }
}

async fn get(c: &Session, op: Uuid) -> Option<GetResponse> {
    let replies = c.get(report_key(op)).timeout(Duration::from_secs(2)).await.unwrap();
    let r = replies.recv_async().await.ok()?;
    Some(serde_json::from_slice(&r.result().ok()?.payload().to_bytes()).unwrap())
}

// ---------------------------------------------------------------- scenarios

async fn s_start() {
    println!("== start: query outcomes, rejection, timeout, retry, concurrency, normalization");
    // A. No queryable at all.
    let port = free_port();
    let g = open_gecko(port).await;
    let c = open_caller(port).await;
    let (out, el) = raw_start(&c, b"{}".to_vec(), Duration::from_millis(500)).await;
    println!("  A no-queryable: replies={out:?} elapsed={el:?}");
    g.close().await.unwrap();
    c.close().await.unwrap();

    let (g, c, _core, _) = setup(Duration::ZERO).await;
    // B. Malformed payload and semantic rejection -> both Ok(Rejected{code}).
    let (out, _) = raw_start(&c, b"not json".to_vec(), Duration::from_secs(1)).await;
    println!("  B malformed: {out:?}");
    let mut bad = req(Uuid::new_v4(), serde_json::json!({}));
    bad.operation_type = "nope".into();
    println!("  B unknown type: {:?}", start(&c, &bad).await);

    // D. Same-content retry with reordered keys + different trace -> same op; changed content -> conflict.
    let id = Uuid::new_v4();
    let r1 = req(id, serde_json::from_str(r#"{"b":1,"a":{"y":2,"x":3}}"#).unwrap());
    let mut r2 = req(id, serde_json::from_str(r#"{"a":{"x":3,"y":2},"b":1}"#).unwrap());
    r2.trace = Some("trace-2".into());
    let a1 = start(&c, &r1).await;
    let a2 = start(&c, &r2).await;
    println!("  D first={a1:?}\n    retry(reordered keys, new trace)={a2:?}");
    let r3 = req(id, serde_json::from_str(r#"{"b":1.0,"a":{"x":3,"y":2}}"#).unwrap());
    println!("    retry with b=1.0 instead of 1: {:?}", start(&c, &r3).await);

    // E. Concurrent same-content retries -> one execution.
    let id = Uuid::new_v4();
    let mut hs = vec![];
    for _ in 0..32 {
        let c = c.clone();
        let r = req(id, serde_json::json!({"n": 1}));
        hs.push(tokio::spawn(async move { op_of(&start(&c, &r).await) }));
    }
    let mut ops = std::collections::HashSet::new();
    for h in hs {
        ops.insert(h.await.unwrap());
    }
    println!("  E 32 concurrent retries -> distinct operation ids: {}", ops.len());
    g.close().await.unwrap();
    c.close().await.unwrap();

    // G. Observe/Get for a terminal op and for a dropped report.
    let (g, c, core, _) = setup(Duration::ZERO).await;
    let op = op_of(&start(&c, &req(Uuid::new_v4(), serde_json::json!({}))).await);
    core.advance(op, State::Running, "work").await;
    let fin = core.advance(op, State::Succeeded, "done").await;
    println!("  G terminal op, observe after=0: {:?}", observe_frames(&c, op, 0).await);
    println!("    terminal op, observe after={fin} (cursor at final): {:?}", observe_frames(&c, op, fin).await);
    core.drop_report(op).await;
    println!("    dropped report: get={:?} observe={:?}", get(&c, op).await, observe_frames(&c, op, 0).await);
    let r = c.get(format!("{}?after=0&cap=1", observe_key(op))).consolidation(ConsolidationMode::None).await.unwrap();
    let f = r.recv_async().await.ok().and_then(|r| r.result().ok().map(|s| String::from_utf8_lossy(&s.payload().to_bytes()).into_owned()));
    println!("    observe with `&`-separated parameters: {f:?}");
    println!("    unknown op: get={:?} observe={:?}", get(&c, Uuid::new_v4()).await, observe_frames(&c, Uuid::new_v4(), 0).await);
    g.close().await.unwrap();
    c.close().await.unwrap();

    // C. Server commits then replies after caller timeout -> caller sees nothing; retry recovers op.
    let (g, c, core, _) = setup(Duration::from_millis(800)).await;
    let id = Uuid::new_v4();
    let r = req(id, serde_json::json!({}));
    let (out, el) = raw_start(&c, serde_json::to_vec(&r).unwrap(), Duration::from_millis(300)).await;
    println!("  C late reply: caller replies={out:?} elapsed={el:?}");
    tokio::time::sleep(Duration::from_millis(700)).await;
    let (out, _) = raw_start(&c, serde_json::to_vec(&r).unwrap(), Duration::from_secs(2)).await;
    println!("    retry after timeout: {out:?}");
    let _ = core;
    g.close().await.unwrap();
    c.close().await.unwrap();

    // F. Two queryables answer the same Start key (two Geckos on one key space).
    let (g, c, _core, port) = setup(Duration::ZERO).await;
    let p2 = free_port();
    let mut cfg = base_config("peer");
    cfg.insert_json5("listen/endpoints", &format!("[\"tcp/127.0.0.1:{p2}\"]")).unwrap();
    cfg.insert_json5("connect/endpoints", &format!("[\"tcp/127.0.0.1:{port}\"]")).unwrap();
    let g2 = zenoh::open(cfg).await.unwrap();
    serve(&g2, Core::new(g2.clone()), Duration::ZERO).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let r = serde_json::to_vec(&req(Uuid::new_v4(), serde_json::json!({}))).unwrap();
    for (name, tgt, cons) in [
        ("default", None, None),
        ("target=All,consolidation=None", Some(QueryTarget::All), Some(ConsolidationMode::None)),
    ] {
        let mut b = c.get(format!("{PREFIX}/start")).payload(r.clone()).timeout(Duration::from_secs(1));
        if let Some(t) = tgt {
            b = b.target(t);
        }
        if let Some(m) = cons {
            b = b.consolidation(m);
        }
        let replies = b.await.unwrap();
        let mut n = 0;
        while replies.recv_async().await.is_ok() {
            n += 1;
        }
        println!("  F two Gecko queryables, {name}: replies={n}");
    }
    for s in [g, g2, c] {
        s.close().await.unwrap();
    }
}

/// Attach patterns raced against final publication. Final publish is triggered in-process (faster
/// than the wire), at a random 0/50/200/1000 us delay after the caller starts attaching.
async fn s_observe_race() {
    println!("== observe-race: attach vs final publication, 200 iterations per pattern");
    let (g, c, core, _) = setup(Duration::ZERO).await;
    for pattern in ["subscribe-only", "get-then-subscribe", "subscribe-then-get", "streaming-query", "streaming-query-auto"] {
        let mut missed = 0;
        let mut lat = vec![];
        for _ in 0..200 {
            let op = op_of(&start(&c, &req(Uuid::new_v4(), serde_json::json!({}))).await);
            core.advance(op, State::Running, "work").await;
            let go = Arc::new(Notify::new());
            let (core2, go2) = (core.clone(), go.clone());
            // Delay buckets 0, 50, 200, 1000 us after the caller begins attaching (busy-wait, not timer).
            let delay = [0u64, 50, 200, 1000][(Uuid::new_v4().as_u128() % 4) as usize];
            let fin = tokio::spawn(async move {
                go2.notified().await;
                let t = Instant::now();
                while t.elapsed() < Duration::from_micros(delay) {
                    std::hint::spin_loop();
                }
                core2.advance(op, State::Succeeded, "done").await
            });
            let t = Instant::now();
            go.notify_one();
            let got_terminal = observe_once(&c, op, pattern).await;
            fin.await.unwrap();
            if got_terminal {
                lat.push(t.elapsed());
            } else {
                missed += 1;
            }
        }
        lat.sort();
        let p50 = lat.get(lat.len() / 2).copied().unwrap_or_default();
        println!("  {pattern:>20}: missed_terminal={missed}/200 p50_to_terminal={p50:?}");
    }
    g.close().await.unwrap();
    c.close().await.unwrap();
}

/// Returns whether the caller learned the terminal snapshot within 500 ms.
async fn observe_once(c: &Session, op: Uuid, pattern: &str) -> bool {
    let deadline = Duration::from_millis(500);
    let terminal_get = |r: Option<GetResponse>| {
        matches!(r, Some(GetResponse::Report { snapshot }) if snapshot.state.is_terminal())
    };
    match pattern {
        "streaming-query" | "streaming-query-auto" => {
            let mut b = c.get(format!("{}?after=0", observe_key(op))).timeout(deadline);
            if pattern == "streaming-query" {
                b = b.consolidation(ConsolidationMode::None);
            }
            let replies = b.await.unwrap();
            while let Ok(r) = replies.recv_async().await {
                if let Ok(s) = r.result() {
                    if let ObserveFrame::Snapshot { snapshot, .. } = serde_json::from_slice(&s.payload().to_bytes()).unwrap() {
                        if snapshot.state.is_terminal() {
                            return true;
                        }
                    }
                }
            }
            false
        }
        _ => {
            if pattern == "get-then-subscribe" && terminal_get(get(c, op).await) {
                return true;
            }
            let sub = c.declare_subscriber(snapshot_key(op)).await.unwrap();
            if pattern == "subscribe-then-get" && terminal_get(get(c, op).await) {
                return true;
            }
            let r = tokio::time::timeout(deadline, async {
                while let Ok(s) = sub.recv_async().await {
                    let snap: Snapshot = serde_json::from_slice(&s.payload().to_bytes()).unwrap();
                    if snap.state.is_terminal() {
                        return true;
                    }
                }
                false
            })
            .await
            .unwrap_or(false);
            sub.undeclare().await.unwrap();
            r
        }
    }
}

/// Fast publisher, slow subscriber with Ring vs Fifo handlers; probes Get latency meanwhile.
async fn s_slow() {
    println!("== slow-subscriber: 2000 snapshots published back-to-back, consumer sleeps 1 ms/sample");
    for handler in ["ring(32)", "fifo(32)"] {
        let (g, c, core, _) = setup(Duration::ZERO).await;
        let op = op_of(&start(&c, &req(Uuid::new_v4(), serde_json::json!({}))).await);
        let done = Arc::new(AtomicBool::new(false));
        let consumer = {
            let c = c.clone();
            let done = done.clone();
            let key = snapshot_key(op);
            let ring = handler.starts_with("ring");
            tokio::spawn(async move {
                let mut seqs = vec![];
                macro_rules! drain {
                    ($sub:expr) => {
                        loop {
                            let s = match tokio::time::timeout(Duration::from_secs(3), $sub.recv_async()).await {
                                Ok(Ok(s)) => s,
                                _ => break,
                            };
                            let snap: Snapshot = serde_json::from_slice(&s.payload().to_bytes()).unwrap();
                            seqs.push(snap.sequence);
                            std::thread::sleep(Duration::from_millis(1)); // deliberately blocks the worker
                            if snap.state.is_terminal() || done.load(Ordering::SeqCst) && seqs.len() > 3000 {
                                break;
                            }
                        }
                    };
                }
                if ring {
                    let sub = c.declare_subscriber(key).with(RingChannel::new(32)).await.unwrap();
                    drain!(sub);
                } else {
                    let sub = c.declare_subscriber(key).with(FifoChannel::new(32)).await.unwrap();
                    drain!(sub);
                }
                seqs
            })
        };
        tokio::time::sleep(Duration::from_millis(300)).await;
        // Probe: Get latency on the same caller session while the subscriber is backlogged.
        let probe = {
            let c = c.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                let t = Instant::now();
                let r = get(&c, op).await.is_some();
                (r, t.elapsed())
            })
        };
        let t = Instant::now();
        for i in 0..2000 {
            let st = if i == 1999 { State::Succeeded } else { State::Running };
            core.advance(op, st, "step").await;
        }
        let pub_time = t.elapsed();
        done.store(true, Ordering::SeqCst);
        let seqs = consumer.await.unwrap();
        let (probe_ok, probe_lat) = probe.await.unwrap();
        let gaps = seqs.windows(2).filter(|w| w[1] != w[0] + 1).count();
        let last = seqs.last().copied().unwrap_or(0);
        let latest = match get(&c, op).await {
            Some(GetResponse::Report { snapshot }) => snapshot.sequence,
            _ => 0,
        };
        println!(
            "  {handler}: publish_2000={pub_time:?} received={} gaps={gaps} last_seq_received={last} \
             get_latest_seq={latest} get_probe_ok={probe_ok} get_probe_latency={probe_lat:?}",
            seqs.len()
        );
        g.close().await.unwrap();
        c.close().await.unwrap();
    }
}

/// In-process TCP proxy that can be cut and restored to emulate link loss without restarting Gecko.
struct Proxy {
    port: u16,
    task: Option<tokio::task::JoinHandle<()>>,
    upstream: u16,
}

impl Proxy {
    async fn new(upstream: u16) -> Self {
        let mut p = Proxy { port: free_port(), task: None, upstream };
        p.up().await;
        p
    }
    async fn up(&mut self) {
        let l = TcpListener::bind(("127.0.0.1", self.port)).await.unwrap();
        let up = self.upstream;
        self.task = Some(tokio::spawn(async move {
            let mut conns = tokio::task::JoinSet::new();
            loop {
                let Ok((mut a, _)) = l.accept().await else { break };
                conns.spawn(async move {
                    if let Ok(mut b) = TcpStream::connect(("127.0.0.1", up)).await {
                        let _ = tokio::io::copy_bidirectional(&mut a, &mut b).await;
                    }
                });
            }
        }));
    }
    fn down(&mut self) {
        // Aborting the accept task drops the listener and the JoinSet, which aborts every connection.
        if let Some(t) = self.task.take() {
            t.abort();
        }
    }
}

async fn s_reconnect() {
    println!("== reconnect: caller link cut for 1.5 s; 300 snapshots published every 20 ms");
    let port = free_port();
    let g = open_gecko(port).await;
    let core = Core::new(g.clone());
    serve(&g, core.clone(), Duration::ZERO).await;
    let mut proxy = Proxy::new(port).await;
    let c = open_caller(proxy.port).await;
    let op = op_of(&start(&c, &req(Uuid::new_v4(), serde_json::json!({}))).await);
    let sub = c.declare_subscriber(snapshot_key(op)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let pubber = {
        let core = core.clone();
        tokio::spawn(async move {
            for i in 0..300 {
                let st = if i == 299 { State::Succeeded } else { State::Running };
                core.advance(op, st, "tick").await;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
    };
    let t0 = Instant::now();
    let rx = tokio::spawn(async move {
        let mut v = vec![];
        while let Ok(Ok(s)) = tokio::time::timeout(Duration::from_secs(10), sub.recv_async()).await {
            let snap: Snapshot = serde_json::from_slice(&s.payload().to_bytes()).unwrap();
            v.push((t0.elapsed(), snap.sequence));
            if snap.state.is_terminal() {
                break;
            }
        }
        v
    });
    tokio::time::sleep(Duration::from_millis(600)).await;
    proxy.down();
    let cut = t0.elapsed();
    let t = Instant::now();
    let during = get(&c, op).await;
    println!("  Get during outage: {:?} after {:?}", during.map(|_| "reply"), t.elapsed());
    tokio::time::sleep(Duration::from_millis(1500)).await;
    proxy.up().await;
    let restored = t0.elapsed();
    let mut first_get_ok = None;
    while first_get_ok.is_none() && t0.elapsed() < Duration::from_secs(6) {
        let r = c.get(report_key(op)).timeout(Duration::from_millis(100)).await.unwrap();
        if let Ok(rep) = r.recv_async().await {
            if rep.result().is_ok() {
                first_get_ok = Some(t0.elapsed());
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    println!("  first successful Get after restore at {first_get_ok:?}");
    pubber.await.unwrap();
    let v = rx.await.unwrap();
    let mut gaps = vec![];
    for w in v.windows(2) {
        if w[1].1 != w[0].1 + 1 {
            gaps.push((w[0].1, w[1].1, w[1].0));
        }
    }
    println!(
        "  cut_at={cut:?} restored_at={restored:?} received={} last={:?}",
        v.len(),
        v.last().map(|x| x.1)
    );
    for (a, b, at) in gaps {
        println!("  gap: seq {a} -> {b} (resumed at {at:?}, {:?} after restore)", at.saturating_sub(restored));
    }
    println!("  Get after restore: {:?}", get(&c, op).await.map(|r| match r {
        GetResponse::Report { snapshot } => snapshot.sequence,
        _ => 0,
    }));
    g.close().await.unwrap();
    c.close().await.unwrap();
}

/// zenoh-ext AdvancedPublisher/AdvancedSubscriber (unstable API): late-join history and recovery.
async fn s_advanced() {
    use zenoh_ext::{
        AdvancedPublisherBuilderExt, AdvancedSubscriberBuilderExt, CacheConfig, HistoryConfig, MissDetectionConfig,
        RecoveryConfig,
    };
    println!("== advanced: zenoh-ext AdvancedPublisher(cache=16, heartbeat) / AdvancedSubscriber(history, recovery)");
    let port = free_port();
    let g = open_gecko(port).await;
    let mut proxy = Proxy::new(port).await;
    let c = open_caller(proxy.port).await;
    let key = "gecko/g1/op/advanced-test/snapshot";
    let publ = g
        .declare_publisher(key)
        .cache(CacheConfig::default().max_samples(16))
        .sample_miss_detection(MissDetectionConfig::default().heartbeat(Duration::from_millis(200)))
        .publisher_detection()
        .await
        .unwrap();
    for i in 1..=10u64 {
        publ.put(i.to_string()).await.unwrap();
    }
    let sub = c
        .declare_subscriber(key)
        .history(HistoryConfig::default().detect_late_publishers())
        .recovery(RecoveryConfig::default().heartbeat())
        .subscriber_detection()
        .await
        .unwrap();
    let misses = sub.sample_miss_listener().await.unwrap();
    let t0 = Instant::now();
    let mut got = vec![];
    let drain = |sub: &zenoh_ext::AdvancedSubscriber<zenoh::handlers::FifoChannelHandler<zenoh::sample::Sample>>,
                 got: &mut Vec<u64>| {
        while let Ok(Some(s)) = sub.try_recv() {
            got.push(String::from_utf8_lossy(&s.payload().to_bytes()).parse().unwrap());
        }
    };
    tokio::time::sleep(Duration::from_millis(500)).await;
    drain(&sub, &mut got);
    println!("  late join after 10 puts: history delivered {:?}", got);
    got.clear();
    proxy.down();
    for i in 11..=50u64 {
        publ.put(i.to_string()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    proxy.up().await;
    let restored = t0.elapsed();
    tokio::time::sleep(Duration::from_secs(4)).await;
    drain(&sub, &mut got);
    let mut missed = vec![];
    while let Ok(Some(m)) = misses.try_recv() {
        missed.push(m.nb());
    }
    println!(
        "  40 puts during outage, cache=16: after restore received {} samples, first={:?} last={:?}; \
         miss events nb={:?} (restore at {restored:?})",
        got.len(),
        got.first(),
        got.last(),
        missed
    );
    g.close().await.unwrap();
    c.close().await.unwrap();
}

/// One streaming Observe query; returns compact frame descriptions.
async fn observe_frames(c: &Session, op: Uuid, after: u64) -> Vec<String> {
    let replies = c
        .get(format!("{}?after={after}", observe_key(op)))
        .consolidation(ConsolidationMode::None)
        .timeout(Duration::from_secs(2))
        .await
        .unwrap();
    let mut out = vec![];
    while let Ok(r) = replies.recv_async().await {
        out.push(match r.result() {
            Ok(s) => match serde_json::from_slice::<ObserveFrame>(&s.payload().to_bytes()).unwrap() {
                ObserveFrame::Snapshot { snapshot, gap_before } => {
                    format!("snapshot(seq={},{:?},gap_before={gap_before})", snapshot.sequence, snapshot.state)
                }
                ObserveFrame::End { reason } => format!("end({reason:?})"),
                ObserveFrame::Unavailable { .. } => "unavailable".into(),
                ObserveFrame::NotFound => "not_found".into(),
                ObserveFrame::Rejected { code, .. } => format!("rejected({code:?})"),
            },
            Err(e) => format!("err({})", String::from_utf8_lossy(&e.payload().to_bytes())),
        });
    }
    out
}

/// Executor notification identity and arbitration (P5 / ST3), directly against Core.
async fn s_executor() {
    println!("== executor: event identity (token, event_seq), duplicates, stale, ordering, conflicts");
    let (g, c, core, _) = setup(Duration::ZERO).await;
    let op = op_of(&start(&c, &req(Uuid::new_v4(), serde_json::json!({}))).await);
    let tok = core.token(op).await;
    let ev = |n: u64, event: EventKind| ExecutorEvent { token: tok, event_seq: n, event };
    let prog = |s: &str| EventKind::Progress { progress: Progress::stage(s) };
    let term = |st: State| EventKind::Terminal { state: st, progress: Progress::stage("end") };
    let steps: Vec<(&str, ExecutorEvent)> = vec![
        ("progress before ack", ev(1, prog("early"))),
        ("ack", ev(1, EventKind::Ack)),
        ("ack redelivered", ev(1, EventKind::Ack)),
        ("event_seq 1 reused with other content", ev(1, prog("x"))),
        ("event_seq skips 2", ev(3, prog("a"))),
        ("progress a", ev(2, prog("a"))),
        ("progress a redelivered", ev(2, prog("a"))),
        ("repeated ack as new event", ev(3, EventKind::Ack)),
        ("terminal cancelled (no Cancel in 0.1.0)", ev(4, term(State::Cancelled))),
        ("terminal succeeded", ev(4, term(State::Succeeded))),
        ("terminal succeeded redelivered", ev(4, term(State::Succeeded))),
        ("late progress after terminal", ev(5, prog("late"))),
        ("conflicting terminal failed", ev(5, term(State::Failed))),
        ("same terminal as new event", ev(5, term(State::Succeeded))),
        ("unknown token", ExecutorEvent { token: Uuid::new_v4(), event_seq: 1, event: EventKind::Ack }),
    ];
    for (name, e) in steps {
        println!("  {name:>40}: {:?}", core.notify(&e).await);
    }
    if let Some(GetResponse::Report { snapshot }) = get(&c, op).await {
        println!("  final snapshot via Get: seq={} state={:?}", snapshot.sequence, snapshot.state);
    }

    // Concurrent redelivery of the same event: one snapshot.
    let op = op_of(&start(&c, &req(Uuid::new_v4(), serde_json::json!({}))).await);
    let tok = core.token(op).await;
    let mut hs = vec![];
    for _ in 0..32 {
        let core = core.clone();
        hs.push(tokio::spawn(async move {
            core.notify(&ExecutorEvent { token: tok, event_seq: 1, event: EventKind::Ack }).await
        }));
    }
    let mut applied = 0;
    let mut dup = 0;
    for h in hs {
        match h.await.unwrap() {
            NotifyOutcome::Applied { .. } => applied += 1,
            NotifyOutcome::Duplicate { .. } => dup += 1,
            o => println!("  unexpected {o:?}"),
        }
    }
    println!("  32 concurrent copies of one ack: applied={applied} duplicate={dup}");

    // Two different events racing for the same event_seq: one wins, the other is stale.
    let mut hs = vec![];
    for st in [State::Succeeded, State::Failed] {
        let core = core.clone();
        hs.push(tokio::spawn(async move {
            core.notify(&ExecutorEvent { token: tok, event_seq: 2, event: EventKind::Terminal { state: st, progress: Progress::stage("end") } })
                .await
        }));
    }
    let r: Vec<_> = futures_join(hs).await;
    println!("  succeeded vs failed racing on event_seq 2: {r:?}");

    // Ambiguous handoff: pending -> unknown (Core), later ack with the same token -> running.
    let op = op_of(&start(&c, &req(Uuid::new_v4(), serde_json::json!({}))).await);
    let tok = core.token(op).await;
    println!("  handoff lost: {:?}", core.handoff_lost(op).await);
    println!("  late ack with same token: {:?}", core.notify(&ExecutorEvent { token: tok, event_seq: 1, event: EventKind::Ack }).await);
    if let Some(GetResponse::Report { snapshot }) = get(&c, op).await {
        println!("  after late ack: seq={} state={:?}", snapshot.sequence, snapshot.state);
    }
    g.close().await.unwrap();
    c.close().await.unwrap();
}

/// Collapses runs of the same end kind: `closed x30 [0.60s..3.55s]`.
fn summarize_ends(ends: &[String]) -> Vec<String> {
    let mut out: Vec<(String, usize, String, String)> = vec![];
    for e in ends {
        let (k, at) = e.split_once('@').unwrap_or((e, ""));
        match out.last_mut() {
            Some(l) if l.0 == k => {
                l.1 += 1;
                l.3 = at.to_string();
            }
            _ => out.push((k.to_string(), 1, at.to_string(), at.to_string())),
        }
    }
    out.into_iter()
        .map(|(k, n, a, b)| if n == 1 { format!("{k}@{a}") } else { format!("{k} x{n} [{a}..{b}]") })
        .collect()
}

async fn futures_join<T>(hs: Vec<tokio::task::JoinHandle<T>>) -> Vec<T> {
    let mut v = vec![];
    for h in hs {
        v.push(h.await.unwrap());
    }
    v
}

#[derive(Clone, Copy, Debug)]
enum H {
    Default,
    Fifo(usize),
    Ring(usize),
}

/// What a streaming-query observer saw, over all re-issued queries.
#[derive(Default, Debug)]
struct StreamStats {
    frames: usize,
    seq_gaps: usize,
    gap_before_flags: usize,
    ends: Vec<String>,
    queries: usize,
    terminal_at: Option<Duration>,
    last: u64,
}

/// Streaming-query observer: re-issues `observe?after=<last>` until terminal or `deadline`.
async fn stream_observer(c: Session, op: Uuid, h: H, extra: String, per_frame: Duration, t0: Instant, deadline: Duration) -> StreamStats {
    let mut st = StreamStats::default();
    while st.terminal_at.is_none() && t0.elapsed() < deadline {
        st.queries += 1;
        let b = c
            .get(format!("{}?after={}{extra}", observe_key(op), st.last))
            .consolidation(ConsolidationMode::None)
            .timeout(Duration::from_secs(10));
        let mut ended = false;
        macro_rules! drain {
            ($replies:expr) => {{
                let replies = $replies;
                while let Ok(r) = replies.recv_async().await {
                    match r.result() {
                        Ok(s) => match serde_json::from_slice::<ObserveFrame>(&s.payload().to_bytes()).unwrap() {
                            ObserveFrame::Snapshot { snapshot, gap_before } => {
                                st.frames += 1;
                                if gap_before {
                                    st.gap_before_flags += 1;
                                }
                                if snapshot.sequence > st.last + 1 && st.last > 0 {
                                    st.seq_gaps += 1;
                                }
                                if snapshot.sequence > st.last {
                                    st.last = snapshot.sequence;
                                }
                                if snapshot.state.is_terminal() && st.terminal_at.is_none() {
                                    st.terminal_at = Some(t0.elapsed());
                                }
                                if !per_frame.is_zero() {
                                    tokio::time::sleep(per_frame).await;
                                }
                            }
                            ObserveFrame::End { reason } => {
                                ended = true;
                                st.ends.push(format!("{reason:?}@{:?}", t0.elapsed()));
                            }
                            other => {
                                ended = true;
                                st.ends.push(format!("{other:?}"));
                            }
                        },
                        Err(e) => {
                            ended = true;
                            st.ends.push(format!("err({})@{:?}", String::from_utf8_lossy(&e.payload().to_bytes()), t0.elapsed()));
                        }
                    }
                }
            }};
        }
        match h {
            H::Default => drain!(b.await.unwrap()),
            H::Fifo(n) => drain!(b.with(FifoChannel::new(n)).await.unwrap()),
            H::Ring(n) => drain!(b.with(RingChannel::new(n)).await.unwrap()),
        }
        if !ended {
            st.ends.push(format!("closed@{:?}", t0.elapsed()));
        }
        if st.terminal_at.is_none() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    st
}

#[derive(Default, Debug)]
struct SubStats {
    samples: usize,
    seq_gaps: usize,
    repairs: usize,
    liveliness: Vec<String>,
    terminal_at: Option<Duration>,
    terminal_via: &'static str,
    last: u64,
}

/// subscribe-then-Get observer with RingChannel(64). With `repair`, it also watches the Gecko
/// liveliness token and issues a Get when the token reappears after a Delete.
async fn sub_observer(c: Session, op: Uuid, repair: bool, t0: Instant, deadline: Duration) -> SubStats {
    let mut st = SubStats::default();
    let sub = c.declare_subscriber(snapshot_key(op)).with(RingChannel::new(64)).await.unwrap();
    let liv = c.liveliness().declare_subscriber(ALIVE_KEY).history(true).await.unwrap();
    let apply = |st: &mut SubStats, s: Snapshot, via: &'static str| {
        if s.sequence <= st.last {
            return;
        }
        if st.last > 0 && s.sequence > st.last + 1 {
            st.seq_gaps += 1;
        }
        st.last = s.sequence;
        if s.state.is_terminal() && st.terminal_at.is_none() {
            st.terminal_at = Some(t0.elapsed());
            st.terminal_via = via;
        }
    };
    if let Some(GetResponse::Report { snapshot }) = get(&c, op).await {
        apply(&mut st, snapshot, "get");
    }
    let mut saw_delete = false;
    while st.terminal_at.is_none() && t0.elapsed() < deadline {
        tokio::select! {
            s = sub.recv_async() => {
                let Ok(s) = s else { break };
                st.samples += 1;
                apply(&mut st, serde_json::from_slice(&s.payload().to_bytes()).unwrap(), "sub");
            }
            l = liv.recv_async() => {
                let Ok(l) = l else { break };
                st.liveliness.push(format!("{}@{:?}", l.kind(), t0.elapsed()));
                if l.kind() == zenoh::sample::SampleKind::Delete {
                    saw_delete = true;
                } else if saw_delete && repair {
                    saw_delete = false;
                    st.repairs += 1;
                    if let Some(GetResponse::Report { snapshot }) = get(&c, op).await {
                        apply(&mut st, snapshot, "get-repair");
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {}
        }
    }
    st
}

/// Observe paths across a 1.5–2 s link cut: plain subscribe-then-Get, the same with
/// liveliness-triggered Get repair, and the streaming query re-issued with `after=<last>`.
async fn s_reconnect_observe() {
    println!("== reconnect-observe: observers behind a cuttable proxy; Gecko publishes every 20 ms");
    for (case, ticks, cut_at, outage) in [
        ("finish-after-restore", 300u32, 600u64, 1500u64),
        ("finish-during-outage", 60, 400, 2000),
    ] {
        let port = free_port();
        let g = open_gecko(port).await;
        let core = Core::new(g.clone());
        serve(&g, core.clone(), Duration::ZERO).await;
        let mut proxy = Proxy::new(port).await;
        let direct = open_caller(port).await;
        let op = op_of(&start(&direct, &req(Uuid::new_v4(), serde_json::json!({}))).await);
        core.advance(op, State::Running, "work").await;
        let (c1, c2, c3) = (open_caller(proxy.port).await, open_caller(proxy.port).await, open_caller(proxy.port).await);
        tokio::time::sleep(Duration::from_millis(200)).await;
        let t0 = Instant::now();
        let deadline = Duration::from_secs(10);
        let o1 = tokio::spawn(sub_observer(c1.clone(), op, false, t0, deadline));
        let o2 = tokio::spawn(sub_observer(c2.clone(), op, true, t0, deadline));
        let o3 = tokio::spawn(stream_observer(c3.clone(), op, H::Default, String::new(), Duration::ZERO, t0, deadline));
        let pubber = {
            let core = core.clone();
            tokio::spawn(async move {
                let mut fin = Duration::ZERO;
                for i in 0..ticks {
                    let st = if i == ticks - 1 { State::Succeeded } else { State::Running };
                    core.advance(op, st, "tick").await;
                    if i == ticks - 1 {
                        fin = t0.elapsed();
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                fin
            })
        };
        tokio::time::sleep(Duration::from_millis(cut_at)).await;
        proxy.down();
        let cut = t0.elapsed();
        tokio::time::sleep(Duration::from_millis(outage)).await;
        proxy.up().await;
        let restored = t0.elapsed();
        let fin = pubber.await.unwrap();
        let final_seq = ticks as u64 + 2;
        println!("  [{case}] cut={cut:.2?} restored={restored:.2?} terminal_published={fin:.2?} final_seq={final_seq}");
        let s1 = o1.await.unwrap();
        println!(
            "    subscribe-then-get: terminal={:?} via={} last={} samples={} seq_gaps={} liveliness={:?}",
            s1.terminal_at, s1.terminal_via, s1.last, s1.samples, s1.seq_gaps, s1.liveliness
        );
        let s2 = o2.await.unwrap();
        println!(
            "    + liveliness repair: terminal={:?} via={} last={} samples={} seq_gaps={} repairs={} liveliness={:?}",
            s2.terminal_at, s2.terminal_via, s2.last, s2.samples, s2.seq_gaps, s2.repairs, s2.liveliness
        );
        let s3 = o3.await.unwrap();
        println!(
            "    streaming query: terminal={:?} last={} frames={} seq_gaps={} gap_before={} queries={} ends={:?}",
            s3.terminal_at, s3.last, s3.frames, s3.seq_gaps, s3.gap_before_flags, s3.queries, summarize_ends(&s3.ends)
        );
        for s in [g, direct, c1, c2, c3] {
            s.close().await.unwrap();
        }
    }
}

/// Slow streaming-query observer: Core publishes 2000 snapshots back-to-back while the observer
/// takes ~1 ms per frame. Probes a Get on the same caller session meanwhile.
async fn s_slow_observe() {
    println!("== slow-observe: streaming query, 2000 snapshots back-to-back, observer ~1 ms/frame");
    for (name, h, cap) in [
        ("reply=default-fifo(256) watcher=8", H::Default, 8usize),
        ("reply=fifo(32) watcher=8", H::Fifo(32), 8),
        ("reply=ring(32) watcher=8", H::Ring(32), 8),
        ("reply=default-fifo(256) watcher=4096", H::Default, 4096),
    ] {
        let (g, c, core, _) = setup(Duration::ZERO).await;
        let op = op_of(&start(&c, &req(Uuid::new_v4(), serde_json::json!({}))).await);
        core.advance(op, State::Running, "work").await;
        let t0 = Instant::now();
        let obs = tokio::spawn(stream_observer(
            c.clone(),
            op,
            h,
            format!(";cap={cap};lease_ms=30000"),
            Duration::from_millis(1),
            t0,
            Duration::from_secs(30),
        ));
        tokio::time::sleep(Duration::from_millis(200)).await;
        let probe = {
            let c = c.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                let t = Instant::now();
                (get(&c, op).await.is_some(), t.elapsed())
            })
        };
        let t = Instant::now();
        for i in 0..2000 {
            let st = if i == 1999 { State::Succeeded } else { State::Running };
            core.advance(op, st, "step").await;
        }
        let pub_time = t.elapsed();
        let s = obs.await.unwrap();
        let (probe_ok, probe_lat) = probe.await.unwrap();
        let ends = summarize_ends(&s.ends);
        println!(
            "  {name}: publish_2000={pub_time:.2?} terminal={:?} last={} frames={} seq_gaps={} gap_before={} queries={} ends={ends:?} get_probe_ok={probe_ok} get_probe_latency={probe_lat:.2?}",
            s.terminal_at, s.last, s.frames, s.seq_gaps, s.gap_before_flags, s.queries
        );
        g.close().await.unwrap();
        c.close().await.unwrap();
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| "all".into());
    println!("zenoh=1.10.2 (pinned in Cargo.toml) arch={} os={}", std::env::consts::ARCH, std::env::consts::OS);
    let all = arg == "all";
    if all || arg == "start" {
        s_start().await;
    }
    if all || arg == "observe-race" {
        s_observe_race().await;
    }
    if all || arg == "slow" {
        s_slow().await;
    }
    if all || arg == "reconnect" {
        s_reconnect().await;
    }
    if all || arg == "executor" {
        s_executor().await;
    }
    if all || arg == "reconnect-observe" {
        s_reconnect_observe().await;
    }
    if all || arg == "slow-observe" {
        s_slow_observe().await;
    }
    if all || arg == "advanced" {
        s_advanced().await;
    }
}
