//! The lab caller: one Zenoh client session and the actions the client UI
//! offers. Every request and answer goes to the log; the UI only calls this.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Barrier, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use operations::{
    GetResponse, ObserveResponse, OperationId, OperationType, Parameters, Report, RequestId,
    StartRequest, StartResponse, Value,
};
use operations_zenoh::{Client, ClientConfig, ObserveItem};

use crate::logbook::LogBook;

/// The editable request form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    /// Operation type; `test.echo` is the only one the lab catalog knows.
    pub operation_type: String,
    /// Required text parameter `message`; empty leaves it out.
    pub message: String,
    /// Optional integer parameter `repeat`; empty leaves it out, and text that
    /// is not an integer is sent as text (an invalid parameter).
    pub repeat: String,
    /// The `request_id`; the same value again is a retry.
    pub request_id: String,
    /// Operation to read or observe; filled by an accepted Start.
    pub operation_id: String,
    /// `after_sequence` for Observe; empty attaches without a cursor.
    pub after: String,
    /// Concurrent callers in a burst.
    pub burst: String,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            operation_type: "test.echo".to_string(),
            message: "hello".to_string(),
            repeat: String::new(),
            request_id: RequestId::generate().to_string(),
            operation_id: String::new(),
            after: String::new(),
            burst: "8".to_string(),
        }
    }
}

impl Form {
    /// Builds the Start request the form describes.
    ///
    /// # Errors
    ///
    /// What cannot be sent at all: an invalid operation type name or a
    /// `request_id` that is not a UUIDv4.
    pub fn start_request(&self) -> Result<StartRequest, String> {
        let request_id =
            RequestId::parse(self.request_id.trim()).map_err(|e| format!("request_id: {e}"))?;
        let operation_type = OperationType::new(self.operation_type.trim())
            .map_err(|e| format!("operation type: {e}"))?;
        let mut parameters = Parameters::new();
        if !self.message.is_empty() {
            parameters.insert("message".into(), Value::Text(self.message.clone()));
        }
        let repeat = self.repeat.trim();
        if !repeat.is_empty() {
            let value = repeat
                .parse::<i64>()
                .map_or_else(|_| Value::Text(repeat.to_string()), Value::Integer);
            parameters.insert("repeat".into(), value);
        }
        Ok(StartRequest {
            request_id,
            operation_type,
            target: None,
            parameters,
            trace: Default::default(),
        })
    }

    /// The `after` cursor, if one is given.
    ///
    /// # Errors
    ///
    /// Text that is not a non-negative integer.
    pub fn after_sequence(&self) -> Result<Option<u64>, String> {
        let after = self.after.trim();
        if after.is_empty() {
            return Ok(None);
        }
        after
            .parse()
            .map(Some)
            .map_err(|_| format!("after: \"{after}\" is not a sequence number"))
    }

    /// The burst size, between 2 and 64.
    #[must_use]
    pub fn burst_size(&self) -> usize {
        self.burst.trim().parse().unwrap_or(8).clamp(2, 64)
    }
}

/// Something the UI should apply to its form or status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// A Start was accepted: the operation to read and observe next.
    Accepted(OperationId),
}

/// The state of this client's Observe stream.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObserveStatus {
    /// Operation observed, if a stream was ever opened.
    pub operation_id: Option<OperationId>,
    /// Whether a stream is open now.
    pub open: bool,
    /// How the last stream ended, if it did.
    pub ended: Option<String>,
    /// Latest `sequence` held: the cursor for a resume.
    pub held: Option<u64>,
    /// Snapshots received over all streams for this operation.
    pub received: u64,
    /// Ordering violations seen: duplicates, regressions, undisclosed gaps.
    pub violations: u64,
}

/// Summary of a snapshot on one line.
#[must_use]
pub fn describe_report(report: &Report) -> String {
    let mut text = format!(
        "seq {} {}",
        report.sequence,
        format!("{:?}", report.state).to_lowercase()
    );
    if let Some(progress) = &report.progress {
        text.push_str(&format!(" \"{}\"", progress.stage));
        if let (Some(done), Some(total)) = (progress.completed, progress.total) {
            text.push_str(&format!(" {done}/{total}"));
        }
    }
    if report.result != Value::Null {
        text.push_str(&format!(" result={}", describe_value(&report.result)));
    }
    if !report.explanation.is_empty() {
        text.push_str(&format!(" ({})", report.explanation));
    }
    text
}

fn describe_value(value: &Value) -> String {
    match value {
        Value::Text(text) => format!("\"{text}\""),
        other => format!("{other:?}"),
    }
}

/// One-line summary of a Start answer.
#[must_use]
pub fn describe_start(response: &StartResponse) -> String {
    match response {
        StartResponse::Accepted { operation, .. } => format!("accepted {}", operation.id),
        StartResponse::Rejected(rejection) => {
            format!("rejected({:?}): {}", rejection.code, rejection.message)
        }
        StartResponse::InvalidInput(error) => format!("invalid_input: {}", error.message),
        StartResponse::Error(error) => format!("error: {}", error.message),
    }
}

fn describe_get(response: &GetResponse) -> String {
    match response {
        GetResponse::Report(report) => format!("report {}", describe_report(report)),
        GetResponse::Unavailable(_) => "unavailable".to_string(),
        GetResponse::InvalidInput(error) => format!("invalid_input: {}", error.message),
        GetResponse::Error(error) => format!("error: {}", error.message),
    }
}

/// The lab caller.
pub struct LabClient {
    client: Arc<Client>,
    endpoint: String,
    timeout: Duration,
    log: Arc<LogBook>,
    notices: Sender<Notice>,
    observe: Arc<Mutex<ObserveStatus>>,
    stop_observe: Option<Arc<AtomicBool>>,
}

impl LabClient {
    /// Opens a client session to `endpoint`. Start and Get wait `timeout` for
    /// their reply; returns the client and the receiver of its notices.
    ///
    /// # Errors
    ///
    /// The Zenoh error opening the session.
    pub fn open(
        endpoint: &str,
        timeout: Duration,
        log: Arc<LogBook>,
    ) -> Result<(Self, Receiver<Notice>), String> {
        let mut config = ClientConfig::new(vec![endpoint.to_string()]);
        config.timeout = timeout;
        let client = Client::open(config).map_err(|e| format!("cannot open a session: {e}"))?;
        let (notices, receiver) = mpsc::channel();
        log.push(format!(
            "[lab] client connected to {endpoint}; Start/Get timeout {} ms",
            timeout.as_millis()
        ));
        Ok((
            Self {
                client: Arc::new(client),
                endpoint: endpoint.to_string(),
                timeout,
                log,
                notices,
                observe: Arc::default(),
                stop_observe: None,
            },
            receiver,
        ))
    }

    /// The endpoint this client connects to.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Whether the Zenoh session is connected to the Gecko now.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    /// The Start and Get timeout.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// The shared event log.
    #[must_use]
    pub fn log(&self) -> &LogBook {
        &self.log
    }

    /// The Observe stream state.
    #[must_use]
    pub fn observe_status(&self) -> ObserveStatus {
        self.observe
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Sends Start with the form's request, in the background.
    pub fn start(&self, form: &Form) {
        let request = match form.start_request() {
            Ok(request) => request,
            Err(error) => return self.log.push(format!("[client] not sent: {error}")),
        };
        self.log.push(format!(
            "[client] -> start request_id={} type={} params={:?}",
            request.request_id, request.operation_type, request.parameters
        ));
        let (client, log, notices) = (
            Arc::clone(&self.client),
            Arc::clone(&self.log),
            self.notices.clone(),
        );
        thread::spawn(move || {
            match client.start(&request) {
            Ok(response) => {
                log.push(format!("[client] <- start {}", describe_start(&response)));
                if let StartResponse::Accepted { operation, .. } = response {
                    let _ = notices.send(Notice::Accepted(operation.id));
                }
            }
            Err(no_answer) => log.push(format!(
                "[client] <- start {no_answer} (the operation may exist; retry with the same request_id)"
            )),
        }
        });
    }

    /// Sends a payload that is not JSON as a Start.
    pub fn start_malformed(&self) {
        self.log.push("[client] -> start with payload \"not json\"");
        let (client, log) = (Arc::clone(&self.client), Arc::clone(&self.log));
        thread::spawn(move || {
            let answer = client
                .start_raw(b"not json".to_vec())
                .map_or_else(|e| e.to_string(), |r| describe_start(&r));
            log.push(format!("[client] <- start {answer}"));
        });
    }

    /// Sends the form's Start from `burst_size` threads released together.
    /// With `changed_content`, each thread sends a different `message` under
    /// the same `request_id`.
    pub fn burst(&self, form: &Form, changed_content: bool) {
        let request = match form.start_request() {
            Ok(request) => request,
            Err(error) => return self.log.push(format!("[client] not sent: {error}")),
        };
        let size = form.burst_size();
        self.log.push(format!(
            "[client] -> burst of {size} concurrent starts, request_id={}, {}",
            request.request_id,
            if changed_content {
                "each with a different message"
            } else {
                "identical content"
            }
        ));
        let (client, log, notices) = (
            Arc::clone(&self.client),
            Arc::clone(&self.log),
            self.notices.clone(),
        );
        thread::spawn(move || {
            let barrier = Arc::new(Barrier::new(size));
            let threads: Vec<_> = (0..size)
                .map(|i| {
                    let (client, barrier) = (Arc::clone(&client), Arc::clone(&barrier));
                    let mut request = request.clone();
                    if changed_content {
                        request
                            .parameters
                            .insert("message".into(), Value::Text(format!("caller {i}")));
                    }
                    thread::spawn(move || {
                        barrier.wait();
                        client.start(&request)
                    })
                })
                .collect();
            let mut accepted: BTreeMap<OperationId, usize> = BTreeMap::new();
            let (mut rejected, mut other) = (BTreeMap::<String, usize>::new(), 0);
            for (i, thread) in threads.into_iter().enumerate() {
                let answer = thread
                    .join()
                    .unwrap_or_else(|_| Err(operations_zenoh::NoAnswer("panicked".into())));
                let text = answer
                    .as_ref()
                    .map_or_else(ToString::to_string, describe_start);
                log.push(format!("[client] <- burst caller {i}: {text}"));
                match answer {
                    Ok(StartResponse::Accepted { operation, .. }) => {
                        *accepted.entry(operation.id).or_default() += 1;
                    }
                    Ok(StartResponse::Rejected(r)) => {
                        *rejected.entry(format!("{:?}", r.code)).or_default() += 1;
                    }
                    _ => other += 1,
                }
            }
            log.push(format!(
                "[client] burst summary: {} distinct accepted operation(s) {:?}; rejected {:?}; other or no answer {other}",
                accepted.len(),
                accepted.iter().map(|(id, n)| format!("{id} x{n}")).collect::<Vec<_>>(),
                rejected
            ));
            if let Some(id) = accepted.keys().next() {
                let _ = notices.send(Notice::Accepted(*id));
            }
        });
    }

    /// Reads the latest report of the form's operation. Text that is not an
    /// operation id is sent as the key, to see the server's answer to it.
    pub fn get(&self, form: &Form) {
        let text = form.operation_id.trim().to_string();
        self.log.push(format!("[client] -> get {text}"));
        let (client, log) = (Arc::clone(&self.client), Arc::clone(&self.log));
        thread::spawn(move || {
            let answer = match OperationId::parse(&text) {
                Ok(id) => client.get(id),
                Err(_) => client.get_key(&format!("gecko/g1/op/{text}/report")),
            };
            let text = answer.map_or_else(|e| e.to_string(), |r| describe_get(&r));
            log.push(format!("[client] <- get {text}"));
        });
    }

    /// Opens an Observe stream for the form's operation with the form's
    /// `after`, closing this client's previous stream first.
    pub fn observe(&mut self, form: &Form) {
        let id = match OperationId::parse(form.operation_id.trim()) {
            Ok(id) => id,
            Err(e) => {
                return self
                    .log
                    .push(format!("[client] not sent: operation id: {e}"));
            }
        };
        match form.after_sequence() {
            Ok(after) => self.open_stream(id, after),
            Err(error) => self.log.push(format!("[client] not sent: {error}")),
        }
    }

    /// Opens a new stream for the observed operation, after the latest
    /// `sequence` held: what a client does after a lost link or `lagged`.
    pub fn resume(&mut self) {
        let status = self.observe_status();
        match status.operation_id {
            Some(id) => self.open_stream(id, status.held),
            None => self.log.push("[client] nothing to resume: observe first"),
        }
    }

    /// Closes the Observe stream without waiting for it to end.
    pub fn stop_observe(&mut self) {
        if let Some(stop) = self.stop_observe.take() {
            stop.store(true, Ordering::SeqCst);
            if self.observe_status().open {
                self.log
                    .push("[client] observe stream dropped by the client");
            }
        }
    }

    fn open_stream(&mut self, id: OperationId, after: Option<u64>) {
        self.stop_observe();
        {
            let mut status = self.observe.lock().unwrap_or_else(PoisonError::into_inner);
            if status.operation_id != Some(id) {
                *status = ObserveStatus {
                    operation_id: Some(id),
                    ..ObserveStatus::default()
                };
            }
            status.open = true;
            status.ended = None;
        }
        let after_text = after.map_or_else(|| "-".to_string(), |a| a.to_string());
        self.log
            .push(format!("[client] -> observe {id} after={after_text}"));
        let stop = Arc::new(AtomicBool::new(false));
        self.stop_observe = Some(Arc::clone(&stop));
        let (client, log, status) = (
            Arc::clone(&self.client),
            Arc::clone(&self.log),
            Arc::clone(&self.observe),
        );
        thread::spawn(move || {
            let ended = follow_stream(&client, &log, &status, &stop, id, after);
            let mut status = status.lock().unwrap_or_else(PoisonError::into_inner);
            if !stop.load(Ordering::SeqCst) {
                status.open = false;
                status.ended = Some(ended);
            } else if status.operation_id == Some(id) {
                status.open = false;
                status.ended = Some("dropped by the client".to_string());
            }
        });
    }

    /// Closes the session.
    pub fn close(mut self) {
        self.stop_observe();
        if let Ok(client) = Arc::try_unwrap(self.client) {
            client.close();
        }
    }
}

/// Records snapshot `sequence` (with a disclosed `gap`, if any) as received,
/// checking that it continues what was held. Returns a warning on a duplicate,
/// a regression, or an undisclosed gap.
fn receive(status: &mut ObserveStatus, sequence: u64, gap: Option<(u64, u64)>) -> Option<String> {
    let warning = match status.held {
        Some(held) if sequence <= held => Some(format!(
            "!! VIOLATION: seq {sequence} after holding {held} (duplicate or regression)"
        )),
        Some(held) if sequence > held + 1 => match gap {
            Some((first, last)) if first == held + 1 && last + 1 == sequence => None,
            _ => Some(format!(
                "!! VIOLATION: seq {sequence} after {held} without a matching gap disclosure"
            )),
        },
        _ => None,
    };
    if warning.is_some() {
        status.violations += 1;
    }
    status.held = Some(status.held.map_or(sequence, |h| h.max(sequence)));
    status.received += 1;
    warning
}

/// Reads one Observe stream until it ends or `stop` is set; returns why it ended.
fn follow_stream(
    client: &Client,
    log: &LogBook,
    status: &Mutex<ObserveStatus>,
    stop: &AtomicBool,
    id: OperationId,
    after: Option<u64>,
) -> String {
    let mut stream = match client.observe(id, after) {
        Ok(stream) => stream,
        Err(no_answer) => {
            log.push(format!("[client] <- observe {no_answer}"));
            return no_answer.to_string();
        }
    };
    let short = &id.to_string()[..8];
    loop {
        if stop.load(Ordering::SeqCst) {
            return "dropped by the client".to_string();
        }
        let item = match stream.recv_timeout(Duration::from_millis(100)) {
            Ok(Some(item)) => item,
            Ok(None) if stream.is_finished() => return "finished".to_string(),
            Ok(None) => continue,
            Err(no_answer) => {
                log.push(format!(
                    "[client] <- observe {short}: {no_answer}; resume with the held sequence"
                ));
                return no_answer.to_string();
            }
        };
        let mut status = status.lock().unwrap_or_else(PoisonError::into_inner);
        match item {
            ObserveItem::Response(ObserveResponse::Attached {
                current,
                sequence,
                gap,
            }) => {
                let gap_pair = gap.map(|g| (g.first_missed, g.last_missed));
                log.push(format!(
                    "[client] <- observe {short} attached at seq {sequence}{}{}",
                    gap.map_or_else(String::new, |g| format!(
                        ", GAP {}..={} disclosed",
                        g.first_missed, g.last_missed
                    )),
                    current.as_ref().map_or_else(
                        || ", nothing redelivered".to_string(),
                        |c| format!(", current: {}", describe_report(c))
                    )
                ));
                if let Some(current) = current
                    && let Some(warning) = receive(&mut status, current.sequence, gap_pair)
                {
                    log.push(format!("[client] {warning}"));
                }
            }
            ObserveItem::Response(other) => {
                log.push(format!("[client] <- observe {short} {other:?}"));
                return "not attached".to_string();
            }
            ObserveItem::Snapshot(report) => {
                log.push(format!(
                    "[client] <- observe {short} snapshot {}",
                    describe_report(&report)
                ));
                if let Some(warning) = receive(&mut status, report.sequence, None) {
                    log.push(format!("[client] {warning}"));
                }
            }
            ObserveItem::End(reason) => {
                let reason = format!("{reason:?}").to_lowercase();
                log.push(format!(
                    "[client] <- observe {short} stream ended: {reason}{}",
                    if reason == "terminal" {
                        ""
                    } else {
                        "; resume with the held sequence"
                    }
                ));
                return reason;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_form_is_a_valid_echo_request() {
        let request = Form::default().start_request().unwrap();
        assert_eq!(request.operation_type.to_string(), "test.echo");
        assert_eq!(
            request.parameters.get("message"),
            Some(&Value::Text("hello".into()))
        );
    }

    #[test]
    fn repeat_is_an_integer_or_sent_as_text() {
        let mut form = Form {
            repeat: "3".into(),
            ..Form::default()
        };
        assert_eq!(
            form.start_request().unwrap().parameters.get("repeat"),
            Some(&Value::Integer(3))
        );
        form.repeat = "three".into();
        assert_eq!(
            form.start_request().unwrap().parameters.get("repeat"),
            Some(&Value::Text("three".into()))
        );
    }

    #[test]
    fn an_invalid_request_id_is_not_sent() {
        let form = Form {
            request_id: "abc".into(),
            ..Form::default()
        };
        assert!(form.start_request().is_err());
    }

    #[test]
    fn receiving_flags_duplicates_and_undisclosed_gaps() {
        let mut status = ObserveStatus::default();
        assert_eq!(receive(&mut status, 2, None), None);
        assert_eq!(receive(&mut status, 3, None), None);
        assert!(receive(&mut status, 3, None).is_some());
        assert!(receive(&mut status, 6, None).is_some());
        assert_eq!(receive(&mut status, 9, Some((7, 8))), None);
        assert_eq!(status.violations, 2);
        assert_eq!(status.held, Some(9));
    }
}
