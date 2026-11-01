//! Zenoh 1.10.2 spike for Operation 0.1.0 (ROB-60). Run: `cargo run --release -- <scenario>|all`.
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

/// Serves Start, Get and streaming Observe for `core` on `session`.
async fn serve(session: &Session, core: Core, start_delay: Duration) {
    let q = session.declare_queryable(format!("{PREFIX}/start")).await.unwrap();
    let c = core.clone();
    tokio::spawn(async move {
        while let Ok(query) = q.recv_async().await {
            let c = c.clone();
            tokio::spawn(async move {
                let Some(payload) = query.payload() else {
                    query.reply_err("missing payload").await.unwrap();
                    return;
                };
                let req: StartRequest = match serde_json::from_slice(&payload.to_bytes()) {
                    Ok(r) => r,
                    Err(e) => {
                        query.reply_err(format!("malformed: {e}")).await.unwrap();
                        return;
                    }
                };
                // Acceptance is committed before the delayed reply: a lost/late reply leaves an accepted op.
                let resp = c.start(CALLER, &req).await;
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
            let id = op_id_from_key(query.key_expr().as_str());
            let resp = match id {
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
            tokio::spawn(async move {
                let after: Option<u64> = query.parameters().get("after").and_then(|v| v.parse().ok());
                let key = query.key_expr().clone();
                let send = |f: ObserveFrame| {
                    let q = &query;
                    let k = key.clone();
                    async move { q.reply(k, serde_json::to_vec(&f).unwrap()).await }
                };
                let Some((latest, mut rx)) = (match op_id_from_key(key.as_str()) {
                    Some(id) => c.attach(id, 8).await,
                    None => None,
                }) else {
                    let _ = send(ObserveFrame::NotFound).await;
                    return;
                };
                let mut last = after.unwrap_or(0);
                if latest.sequence > last {
                    let gap = latest.sequence > last + 1 && after.is_some();
                    last = latest.sequence;
                    let term = latest.state.is_terminal();
                    if send(ObserveFrame::Snapshot { snapshot: latest, gap_before: gap }).await.is_err() {
                        return;
                    }
                    if term {
                        let _ = send(ObserveFrame::End { reason: "terminal".into() }).await;
                        return;
                    }
                }
                let lease = tokio::time::sleep(Duration::from_secs(5));
                tokio::pin!(lease);
                loop {
                    tokio::select! {
                        s = rx.recv() => match s {
                            Some(s) => {
                                if s.sequence <= last { continue; }
                                let gap = s.sequence > last + 1;
                                last = s.sequence;
                                let term = s.state.is_terminal();
                                if send(ObserveFrame::Snapshot { snapshot: s, gap_before: gap }).await.is_err() { return; }
                                if term { let _ = send(ObserveFrame::End { reason: "terminal".into() }).await; return; }
                            }
                            None => { let _ = send(ObserveFrame::End { reason: "lagged".into() }).await; return; }
                        },
                        _ = &mut lease => { let _ = send(ObserveFrame::End { reason: "lease".into() }).await; return; }
                    }
                }
            });
        }
    });
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
            Err(e) => Err(String::from_utf8_lossy(&e.payload().to_bytes()).into_owned()),
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
    // B. Malformed payload -> reply_err; semantic rejection -> Ok(Rejected).
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
    if all || arg == "advanced" {
        s_advanced().await;
    }
}
