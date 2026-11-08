//! The Gecko side: serves Start, Get, and streaming Observe from one [`Acceptor`].

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use operations::{
    Acceptor, Authorizer, CallerContext, DecisionLog, Executor, GetRequest, GetResponse,
    InputError, ObserveRequest, ObserveResponse, Poll, ServiceError, StartRequest, StartResponse,
    Storage, Violation,
};
use zenoh::query::Query;
use zenoh::{Session, Wait};

use crate::keys::Keys;
use crate::wire::{EndReason, ObserveFrame};

/// Server settings. The caller identity is the adapter's own configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Zenoh endpoints to listen on, such as `tcp/127.0.0.1:7447`.
    pub listen: Vec<String>,
    /// Key prefix.
    pub keys: Keys,
    /// Trusted caller context applied to every request (restricted deployment).
    pub caller: CallerContext,
    /// Longest an Observe stream stays open before ending with `lease`.
    pub observe_lease: Duration,
    /// Most Observe streams served at once; further ones get a typed error.
    pub max_observers: usize,
    /// Test hook: wait this long after committing a Start before replying, to
    /// make a caller time out on an accepted request.
    pub start_reply_delay: Duration,
}

impl ServerConfig {
    /// Settings for `caller` listening on `listen`, with the default prefix.
    pub fn new(listen: Vec<String>, caller: CallerContext) -> Self {
        Self {
            listen,
            keys: Keys::default(),
            caller,
            observe_lease: Duration::from_secs(20),
            max_observers: 64,
            start_reply_delay: Duration::ZERO,
        }
    }
}

/// How often an idle Observe stream checks its lease and the Core stream.
const POLL: Duration = Duration::from_millis(50);

struct Shared<A, L, S> {
    core: Arc<Acceptor<A, L, S>>,
    executor: Mutex<Box<dyn Executor + Send>>,
    config: ServerConfig,
    stop: AtomicBool,
    observers: AtomicUsize,
    start_delay_ms: AtomicU64,
}

/// A running server. Dropping it stops serving and closes the session.
pub struct Server<A, L, S> {
    shared: Arc<Shared<A, L, S>>,
    session: Option<Session>,
    threads: Vec<JoinHandle<()>>,
}

fn json<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec())
}

fn input_error(field: &str, message: String) -> InputError {
    InputError {
        message: message.clone(),
        violations: vec![Violation::new(field, message)],
    }
}

impl<A, L, S> Server<A, L, S>
where
    A: Authorizer + Send + 'static,
    L: DecisionLog + Send + 'static,
    S: Storage + 'static,
{
    /// Opens a peer session that listens on the configured endpoints and
    /// serves `core`. Created executions are handed to `executor`.
    ///
    /// The `core` must give out `config.keys.address(id)` as observation
    /// address for clients to find the stream.
    ///
    /// # Errors
    ///
    /// Any Zenoh error opening the session or declaring a queryable.
    pub fn open(
        core: Arc<Acceptor<A, L, S>>,
        executor: Box<dyn Executor + Send>,
        config: ServerConfig,
    ) -> Result<Self, zenoh::Error> {
        let mut zconfig = zenoh::Config::default();
        zconfig.insert_json5("mode", "\"peer\"")?;
        zconfig.insert_json5("scouting/multicast/enabled", "false")?;
        zconfig.insert_json5("scouting/gossip/enabled", "false")?;
        let endpoints = serde_json::to_string(&config.listen)?;
        zconfig.insert_json5("listen/endpoints", &endpoints)?;
        let session = zenoh::open(zconfig).wait()?;

        let config_delay = config.start_reply_delay;
        let shared = Arc::new(Shared {
            core,
            executor: Mutex::new(executor),
            config,
            stop: AtomicBool::new(false),
            observers: AtomicUsize::new(0),
            start_delay_ms: AtomicU64::new(
                u64::try_from(config_delay.as_millis()).unwrap_or(u64::MAX),
            ),
        });
        let keys = shared.config.keys.clone();
        let mut threads = Vec::new();

        let start = session.declare_queryable(keys.start()).wait()?;
        let report = session.declare_queryable(keys.report_pattern()).wait()?;
        let observe = session.declare_queryable(keys.observe_pattern()).wait()?;

        let s = Arc::clone(&shared);
        threads.push(thread::spawn(move || {
            while let Ok(query) = start.recv() {
                let s = Arc::clone(&s);
                thread::spawn(move || s.serve_start(&query));
            }
        }));
        let s = Arc::clone(&shared);
        threads.push(thread::spawn(move || {
            while let Ok(query) = report.recv() {
                s.serve_get(&query);
            }
        }));
        let s = Arc::clone(&shared);
        threads.push(thread::spawn(move || {
            while let Ok(query) = observe.recv() {
                if s.observers.fetch_add(1, Ordering::SeqCst) >= s.config.max_observers {
                    s.observers.fetch_sub(1, Ordering::SeqCst);
                    let response = ObserveResponse::Error(ServiceError {
                        message: "too many observers".to_string(),
                    });
                    s.send(&query, &ObserveFrame::Response { response });
                    continue;
                }
                let s = Arc::clone(&s);
                thread::spawn(move || {
                    s.serve_observe(&query);
                    s.observers.fetch_sub(1, Ordering::SeqCst);
                });
            }
        }));
        // The queryables are owned by their threads; they end when the session closes.
        Ok(Self {
            shared,
            session: Some(session),
            threads,
        })
    }

    /// Changes the Start reply delay test hook while running.
    pub fn set_start_reply_delay(&self, delay: Duration) {
        let millis = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX);
        self.shared.start_delay_ms.store(millis, Ordering::SeqCst);
    }

    /// Stops serving and closes the Zenoh session.
    pub fn close(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Some(session) = self.session.take() {
            let _ = session.close().wait();
        }
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

impl<A, L, S> Drop for Server<A, L, S> {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Some(session) = self.session.take() {
            let _ = session.close().wait();
        }
    }
}

impl<A, L, S> Shared<A, L, S>
where
    A: Authorizer + Send + 'static,
    L: DecisionLog + Send + 'static,
    S: Storage + 'static,
{
    fn send<T: serde::Serialize>(&self, query: &Query, frame: &T) -> bool {
        query
            .reply(query.key_expr().clone(), json(frame))
            .wait()
            .is_ok()
    }

    fn serve_start(&self, query: &Query) {
        let parsed = match query.payload() {
            None => Err("the request has no payload".to_string()),
            Some(payload) => serde_json::from_slice::<StartRequest>(&payload.to_bytes())
                .map_err(|e| format!("malformed Start request: {e}")),
        };
        let response = match parsed {
            Err(message) => StartResponse::InvalidInput(input_error("payload", message)),
            Ok(request) => {
                let outcome = self.core.start(&self.config.caller, &request);
                if let Some(dispatch) = outcome.dispatch {
                    // The commit is done; hand the new execution to the executor once.
                    let handoff = match self.executor.lock() {
                        Ok(mut executor) => self.core.dispatch(dispatch, executor.as_mut()),
                        Err(_) => Err(operations::DispatchError::Unavailable),
                    };
                    if handoff.is_err() {
                        // The execution stays recorded; a retry returns it.
                        eprintln!("operations-zenoh: handoff result was not recorded");
                    }
                }
                outcome.response
            }
        };
        let delay = self.start_delay_ms.load(Ordering::SeqCst);
        if delay > 0 {
            thread::sleep(Duration::from_millis(delay));
        }
        self.send(query, &response);
    }

    fn serve_get(&self, query: &Query) {
        let response = match self.config.keys.operation_id(query.key_expr().as_str()) {
            Some(operation_id) => self
                .core
                .get(&self.config.caller, &GetRequest { operation_id }),
            None => GetResponse::InvalidInput(input_error(
                "key",
                "the key does not contain an operation id".to_string(),
            )),
        };
        self.send(query, &response);
    }

    fn serve_observe(&self, query: &Query) {
        let reject = |response: ObserveResponse| {
            self.send(query, &ObserveFrame::Response { response });
        };
        let Some(operation_id) = self.config.keys.operation_id(query.key_expr().as_str()) else {
            return reject(ObserveResponse::InvalidInput(input_error(
                "key",
                "the key does not contain an operation id".to_string(),
            )));
        };
        let after_sequence = match query.parameters().get("after").map(str::parse::<u64>) {
            None => None,
            Some(Ok(after)) => Some(after),
            Some(Err(e)) => {
                return reject(ObserveResponse::InvalidInput(input_error(
                    "after_sequence",
                    format!("not a sequence number: {e}"),
                )));
            }
        };
        let outcome = self.core.observe(
            &self.config.caller,
            &ObserveRequest {
                operation_id,
                after_sequence,
            },
        );
        let attached = matches!(outcome.response, ObserveResponse::Attached { .. });
        if !self.send(
            query,
            &ObserveFrame::Response {
                response: outcome.response,
            },
        ) || !attached
        {
            return;
        }
        let Some(stream) = outcome.stream else {
            // Attached to a terminal execution: nothing more will be published.
            self.send(
                query,
                &ObserveFrame::End {
                    reason: EndReason::Terminal,
                },
            );
            return;
        };
        let lease_ends = Instant::now() + self.config.observe_lease;
        loop {
            let frame = match stream.next_timeout(POLL) {
                Poll::Snapshot(report) => ObserveFrame::Snapshot { report },
                Poll::Ended(end) => {
                    self.send(query, &ObserveFrame::End { reason: end.into() });
                    return;
                }
                Poll::Empty => {
                    if self.stop.load(Ordering::SeqCst) {
                        return;
                    }
                    if Instant::now() >= lease_ends {
                        self.send(
                            query,
                            &ObserveFrame::End {
                                reason: EndReason::Lease,
                            },
                        );
                        return;
                    }
                    continue;
                }
            };
            if !self.send(query, &frame) {
                return; // the observer is gone; dropping the stream detaches it
            }
        }
    }
}
