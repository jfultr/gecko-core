//! The lab controllers over a real Zenoh session, without a terminal: checks
//! that the stand itself works. It does not replace the manual run (GKO-68).

use std::net::TcpListener;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use operations::{OperationId, Outcome, RequestId, State};
use operations_lab::client::{Form, LabClient, Notice};
use operations_lab::logbook::LogBook;
use operations_lab::server::{AckMode, Finish, Lab, LabServerConfig, Script};

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port()
}

fn wait_until(what: &str, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if ready() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for {what}");
}

fn accepted(notices: &Receiver<Notice>) -> OperationId {
    match notices.recv_timeout(Duration::from_secs(10)) {
        Ok(Notice::Accepted(id)) => id,
        Err(e) => panic!("no acceptance: {e}"),
    }
}

fn state(lab: &Lab, id: OperationId) -> Option<State> {
    lab.rows()
        .into_iter()
        .find(|r| r.operation_id == id)
        .and_then(|r| r.report)
        .map(|r| r.state)
}

struct Stand {
    lab: Lab,
    client: LabClient,
    notices: Receiver<Notice>,
    client_log: Arc<LogBook>,
}

fn stand(timeout: Duration) -> Stand {
    let config = LabServerConfig {
        port: free_port(),
        proxy_port: free_port(),
        observe_lease: Duration::from_secs(5),
        ..LabServerConfig::default()
    };
    let lab = Lab::start(config, Arc::new(LogBook::in_memory())).expect("lab");
    let client_log = Arc::new(LogBook::in_memory());
    let (client, notices) =
        LabClient::open(&lab.endpoint(), timeout, Arc::clone(&client_log)).expect("client");
    Stand {
        lab,
        client,
        notices,
        client_log,
    }
}

fn logged(log: &LogBook, needle: &str) -> bool {
    log.all().iter().any(|l| l.contains(needle))
}

#[test]
fn lost_acknowledgement_is_unknown_then_refined_by_evidence() {
    let s = stand(Duration::from_secs(2));
    s.lab.set_script(Script {
        ack: AckMode::Lose,
        ..Script::default()
    });
    s.client.start(&Form::default());
    let id = accepted(&s.notices);
    wait_until("unknown", || state(&s.lab, id) == Some(State::Unknown));
    let row = s
        .lab
        .rows()
        .into_iter()
        .find(|r| r.operation_id == id)
        .unwrap();
    assert_eq!(row.handoffs, 1);

    s.lab.send_ack(id);
    wait_until("running", || state(&s.lab, id) == Some(State::Running));
    s.lab.send_terminal(id, Outcome::Succeeded);
    wait_until("succeeded", || state(&s.lab, id) == Some(State::Succeeded));
    // A resent terminal is a duplicate: no new snapshot.
    s.lab.resend_last(id);
    assert!(logged(s.lab.log(), "duplicate, no new snapshot"));
    s.client.close();
    s.lab.close();
}

#[test]
fn lost_start_reply_and_retry_create_one_execution() {
    let mut s = stand(Duration::from_millis(500));
    s.lab.set_reply_delay(Duration::from_millis(1500));
    let form = Form::default();
    s.client.start(&form);
    wait_until("no answer", || logged(&s.client_log, "<- start no answer"));
    s.lab.set_reply_delay(Duration::ZERO);
    s.client.start(&form);
    accepted(&s.notices);
    let counters = s.lab.counters();
    assert_eq!(counters.operations, 1);
    assert_eq!(counters.handoffs, 1);

    // Same request_id, changed content: a conflict, nothing new.
    let changed = Form {
        message: "other".into(),
        ..form
    };
    s.client.start(&changed);
    wait_until("conflict", || logged(&s.client_log, "IdentifierConflict"));
    assert_eq!(s.lab.counters().operations, 1);
    s.client.close();
    s.lab.close();
}

#[test]
fn bursts_with_one_request_id_create_one_execution() {
    let s = stand(Duration::from_secs(3));
    let same = Form::default();
    s.client.burst(&same, false);
    accepted(&s.notices);
    wait_until("summary", || logged(&s.client_log, "burst summary"));
    assert!(logged(
        &s.client_log,
        "burst summary: 1 distinct accepted operation(s)"
    ));
    assert_eq!(s.lab.counters().operations, 1);

    let changed = Form {
        request_id: RequestId::generate().to_string(),
        ..Form::default()
    };
    s.client.burst(&changed, true);
    accepted(&s.notices);
    wait_until("second summary", || {
        s.client_log
            .all()
            .iter()
            .filter(|l| l.contains("burst summary"))
            .count()
            == 2
    });
    let summary = s
        .client_log
        .all()
        .into_iter()
        .rfind(|l| l.contains("burst summary"))
        .unwrap();
    assert!(summary.contains("1 distinct accepted"), "{summary}");
    assert!(summary.contains("IdentifierConflict"), "{summary}");
    assert_eq!(s.lab.counters().operations, 2);
    s.client.close();
    s.lab.close();
}

#[test]
fn observing_follows_progress_to_the_terminal_and_resumes_without_duplicates() {
    let mut s = stand(Duration::from_secs(2));
    s.lab.set_script(Script {
        steps: 3,
        interval: Duration::from_millis(100),
        finish: Finish::Succeed,
        ..Script::default()
    });
    let form = Form::default();
    s.client.start(&form);
    let id = accepted(&s.notices);
    let form = Form {
        operation_id: id.to_string(),
        ..form
    };
    s.client.observe(&form);
    wait_until("terminal end", || {
        logged(&s.client_log, "stream ended: terminal")
    });
    let status = s.client.observe_status();
    assert_eq!(status.violations, 0);
    assert_eq!(status.held, Some(6)); // pending 1, running 2, 3 progress, terminal

    // Resuming after the held sequence redelivers nothing.
    s.client.resume();
    wait_until("second terminal end", || {
        s.client_log
            .all()
            .iter()
            .filter(|l| l.contains("stream ended: terminal"))
            .count()
            == 2
    });
    assert!(logged(&s.client_log, "nothing redelivered"));
    assert_eq!(s.client.observe_status().violations, 0);
    s.client.close();
    s.lab.close();
}
