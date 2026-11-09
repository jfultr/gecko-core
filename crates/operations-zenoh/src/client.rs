//! Demo client: Start, Get, and Observe over Zenoh, with explicit retry.

use std::fmt;
use std::time::Duration;

use operations::{
    Applied, Follower, Gap, GetResponse, ObserveResponse, OperationId, Report, StartRequest,
    StartResponse,
};
use zenoh::query::{ConsolidationMode, Reply};
use zenoh::{Session, Wait};

use crate::keys::Keys;
use crate::wire::{EndReason, ObserveFrame};

/// The transport gave no usable answer: a Zenoh error reply, no reply, a
/// reply that cannot be read, or a stream that closed without an `end` frame.
///
/// It says nothing about whether the operation exists or ran. Retrying is safe:
/// Start with the same `request_id` creates at most one execution, and Observe
/// resumes from the last `sequence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoAnswer(pub String);

impl fmt::Display for NoAnswer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no answer: {}", self.0)
    }
}

impl std::error::Error for NoAnswer {}

/// Client settings.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Zenoh endpoints of the Gecko, such as `tcp/127.0.0.1:7447`.
    pub connect: Vec<String>,
    /// Key prefix; must match the server's.
    pub keys: Keys,
    /// How long Start and Get wait for their reply.
    pub timeout: Duration,
    /// Longest an Observe stream may stay open on the client side. Keep it
    /// above the server's lease, so a lease ends the stream cleanly.
    pub observe_timeout: Duration,
}

impl ClientConfig {
    /// Settings for a Gecko at `connect` with the default prefix.
    pub fn new(connect: Vec<String>) -> Self {
        Self {
            connect,
            keys: Keys::default(),
            timeout: Duration::from_secs(2),
            observe_timeout: Duration::from_secs(30),
        }
    }
}

/// One thing received on an Observe stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObserveItem {
    /// The typed attach outcome; always first.
    Response(ObserveResponse),
    /// A snapshot newer than the attach outcome's.
    Snapshot(Box<Report>),
    /// The stream ended on purpose.
    End(EndReason),
}

/// An open Observe stream.
pub struct ObserveStream {
    replies: zenoh::handlers::FifoChannelHandler<Reply>,
    finished: bool,
}

impl ObserveStream {
    /// Waits for the next frame.
    ///
    /// # Errors
    ///
    /// [`NoAnswer`] when the stream closed without an `end` frame, or a frame
    /// is an error reply or unreadable.
    pub fn recv(&mut self) -> Result<Option<ObserveItem>, NoAnswer> {
        if self.finished {
            return Ok(None);
        }
        let Ok(reply) = self.replies.recv() else {
            self.finished = true;
            return Err(NoAnswer("the stream closed without an end".to_string()));
        };
        self.accept(&reply).map(Some)
    }

    fn accept(&mut self, reply: &Reply) -> Result<ObserveItem, NoAnswer> {
        let frame = decode::<ObserveFrame>(reply).inspect_err(|_| self.finished = true)?;
        Ok(match frame {
            ObserveFrame::Response { response } => {
                self.finished = !matches!(response, ObserveResponse::Attached { .. });
                ObserveItem::Response(response)
            }
            ObserveFrame::Snapshot { report } => ObserveItem::Snapshot(report),
            ObserveFrame::End { reason } => {
                self.finished = true;
                ObserveItem::End(reason)
            }
        })
    }
}

impl ObserveStream {
    /// Waits up to `wait` for the next frame. `Ok(None)` means nothing arrived
    /// in time, or the stream is finished (see [`ObserveStream::is_finished`]).
    ///
    /// # Errors
    ///
    /// As [`ObserveStream::recv`].
    pub fn recv_timeout(&mut self, wait: Duration) -> Result<Option<ObserveItem>, NoAnswer> {
        if self.finished {
            return Ok(None);
        }
        match self.replies.recv_timeout(wait) {
            Ok(None) => Ok(None),
            Ok(Some(reply)) => self.accept(&reply).map(Some),
            Err(_) => {
                self.finished = true;
                Err(NoAnswer("the stream closed without an end".to_string()))
            }
        }
    }

    /// Whether the stream has ended: an `end` frame, a final attach outcome,
    /// or no answer.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

fn decode<T: serde::de::DeserializeOwned>(reply: &Reply) -> Result<T, NoAnswer> {
    match reply.result() {
        Ok(sample) => serde_json::from_slice(&sample.payload().to_bytes())
            .map_err(|e| NoAnswer(format!("unreadable reply: {e}"))),
        Err(error) => Err(NoAnswer(format!(
            "error reply: {}",
            String::from_utf8_lossy(&error.payload().to_bytes())
        ))),
    }
}

/// How [`Client::follow`] ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Followed {
    /// The execution reached a terminal state; this is the last snapshot held.
    Terminal(Box<Report>),
    /// The report is not available to this caller.
    Unavailable,
}

/// [`Client::follow`] gave up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowError {
    /// The server answered with invalid input or an error.
    Rejected(ObserveResponse),
    /// Every reconnection attempt got no answer or ended early.
    GaveUp(NoAnswer),
}

/// A Zenoh client session of the Operation mapping.
pub struct Client {
    session: Session,
    config: ClientConfig,
}

impl Client {
    /// Opens a client session connecting to the configured endpoints.
    ///
    /// # Errors
    ///
    /// Any Zenoh error opening the session.
    pub fn open(config: ClientConfig) -> Result<Self, zenoh::Error> {
        let mut zconfig = zenoh::Config::default();
        zconfig.insert_json5("mode", "\"client\"")?;
        zconfig.insert_json5("scouting/multicast/enabled", "false")?;
        zconfig.insert_json5("scouting/gossip/enabled", "false")?;
        let endpoints = serde_json::to_string(&config.connect)?;
        zconfig.insert_json5("connect/endpoints", &endpoints)?;
        let session = zenoh::open(zconfig).wait()?;
        Ok(Self { session, config })
    }

    /// Closes the session.
    pub fn close(self) {
        let _ = self.session.close().wait();
    }

    /// Whether the session is connected to the Gecko now (to any router or peer).
    ///
    /// After a lost link, Zenoh reconnects on its own after a while; requests
    /// sent before that get no answer.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        let info = self.session.info();
        info.routers_zid().wait().next().is_some() || info.peers_zid().wait().next().is_some()
    }

    /// The keys this client uses.
    pub fn keys(&self) -> &Keys {
        &self.config.keys
    }

    /// Sends one Start. The request carries its own `request_id`, so sending the
    /// same value again is a retry.
    ///
    /// # Errors
    ///
    /// [`NoAnswer`] if no reply arrived; the execution may still exist.
    pub fn start(&self, request: &StartRequest) -> Result<StartResponse, NoAnswer> {
        let payload = serde_json::to_vec(request).map_err(|e| NoAnswer(e.to_string()))?;
        self.start_raw(payload)
    }

    /// Sends a raw Start payload (for malformed-input checks).
    ///
    /// # Errors
    ///
    /// [`NoAnswer`] if no reply arrived.
    pub fn start_raw(&self, payload: Vec<u8>) -> Result<StartResponse, NoAnswer> {
        self.query_one(&self.config.keys.start(), Some(payload))
    }

    /// Sends Start up to `attempts` times with the same `request_id`, stopping
    /// at the first typed answer. Safe because the server returns the same
    /// operation for the same `request_id` and content.
    ///
    /// # Errors
    ///
    /// The last [`NoAnswer`] when every attempt got no answer.
    pub fn start_retrying(
        &self,
        request: &StartRequest,
        attempts: u32,
    ) -> Result<StartResponse, NoAnswer> {
        let mut last = NoAnswer("no attempt was made".to_string());
        for _ in 0..attempts.max(1) {
            match self.start(request) {
                Ok(response) => return Ok(response),
                Err(no_answer) => last = no_answer,
            }
        }
        Err(last)
    }

    /// Reads the latest report of `id`.
    ///
    /// # Errors
    ///
    /// [`NoAnswer`] if no reply arrived.
    pub fn get(&self, id: OperationId) -> Result<GetResponse, NoAnswer> {
        self.query_one(&self.config.keys.report(id), None)
    }

    /// Reads the report at an arbitrary key (for malformed-key checks).
    ///
    /// # Errors
    ///
    /// [`NoAnswer`] if no reply arrived.
    pub fn get_key(&self, key: &str) -> Result<GetResponse, NoAnswer> {
        self.query_one(key, None)
    }

    fn query_one<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
        payload: Option<Vec<u8>>,
    ) -> Result<T, NoAnswer> {
        let mut builder = self.session.get(key).timeout(self.config.timeout);
        if let Some(payload) = payload {
            builder = builder.payload(payload);
        }
        let replies = builder.wait().map_err(|e| NoAnswer(e.to_string()))?;
        let reply = replies
            .recv()
            .map_err(|_| NoAnswer("no reply before the query closed".to_string()))?;
        decode(&reply)
    }

    /// Opens an Observe stream, resuming after `after_sequence` if given.
    ///
    /// # Errors
    ///
    /// [`NoAnswer`] if the query could not be sent.
    pub fn observe(
        &self,
        id: OperationId,
        after_sequence: Option<u64>,
    ) -> Result<ObserveStream, NoAnswer> {
        let selector = match after_sequence {
            Some(after) => format!("{}?after={after}", self.config.keys.observe(id)),
            None => self.config.keys.observe(id),
        };
        let replies = self
            .session
            .get(selector)
            // Frames must reach the caller one by one, not be merged into the latest.
            .consolidation(ConsolidationMode::None)
            .timeout(self.config.observe_timeout)
            .wait()
            .map_err(|e| NoAnswer(e.to_string()))?;
        Ok(ObserveStream {
            replies,
            finished: false,
        })
    }

    /// Follows `id` to its terminal state, reconnecting after every no-answer,
    /// `lagged`, or `lease` end by resuming with the last `sequence` held.
    ///
    /// `on_snapshot` sees each snapshot that replaces the held one, with the
    /// gap disclosed when intermediate snapshots were skipped. Reconnecting
    /// never starts or resumes the operation.
    ///
    /// # Errors
    ///
    /// [`FollowError`] when the server rejects the request or `reconnects`
    /// attempts are used up. `GaveUp` carries the last reason: the no-answer
    /// error of the last attempt, or `stream ended` with the end reason
    /// (`Lagged`, `Lease`, `Closed`) when the last stream ended with an `end` frame.
    pub fn follow(
        &self,
        id: OperationId,
        follower: &mut Follower,
        reconnects: u32,
        mut on_snapshot: impl FnMut(&Report, Option<Gap>),
    ) -> Result<Followed, FollowError> {
        let mut last = NoAnswer("no attempt was made".to_string());
        for attempt in 0..=reconnects {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(100));
            }
            let mut stream = match self.observe(id, follower.sequence()) {
                Ok(stream) => stream,
                Err(no_answer) => {
                    last = no_answer;
                    continue;
                }
            };
            loop {
                match stream.recv() {
                    Err(no_answer) => {
                        last = no_answer;
                        break;
                    }
                    Ok(None) => {
                        last = NoAnswer("the stream closed without an end frame".to_string());
                        break;
                    }
                    Ok(Some(ObserveItem::Response(ObserveResponse::Attached {
                        current,
                        gap,
                        ..
                    }))) => {
                        if let Some(current) = current {
                            apply(follower, *current, gap, &mut on_snapshot);
                        }
                    }
                    Ok(Some(ObserveItem::Response(ObserveResponse::Unavailable(_)))) => {
                        return Ok(Followed::Unavailable);
                    }
                    Ok(Some(ObserveItem::Response(other))) => {
                        return Err(FollowError::Rejected(other));
                    }
                    Ok(Some(ObserveItem::Snapshot(report))) => {
                        apply(follower, *report, None, &mut on_snapshot);
                    }
                    Ok(Some(ObserveItem::End(EndReason::Terminal))) => {
                        return match follower.latest() {
                            Some(report) if report.state.is_terminal() => {
                                Ok(Followed::Terminal(Box::new(report.clone())))
                            }
                            _ => Err(FollowError::GaveUp(NoAnswer(
                                "terminal end without a terminal snapshot".to_string(),
                            ))),
                        };
                    }
                    Ok(Some(ObserveItem::End(EndReason::Unavailable))) => {
                        return Ok(Followed::Unavailable);
                    }
                    Ok(Some(ObserveItem::End(reason))) => {
                        // lagged, lease, closed: resume; `last` keeps the reason if no attempt succeeds.
                        last = NoAnswer(format!("stream ended: {reason:?}"));
                        break;
                    }
                }
            }
        }
        Err(FollowError::GaveUp(last))
    }
}

fn apply(
    follower: &mut Follower,
    report: Report,
    gap: Option<Gap>,
    on_snapshot: &mut impl FnMut(&Report, Option<Gap>),
) {
    let snapshot = report.clone();
    if let Applied::Replaced { missed } = follower.apply(report) {
        on_snapshot(&snapshot, gap.or(missed));
    }
}
