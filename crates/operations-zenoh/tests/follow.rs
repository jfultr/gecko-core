//! `Client::follow` reports the end reason of its last stream when it gives up.

use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use operations::testing::contract_fixtures::{caller, catalog, echo_request};
use operations::testing::{PermitAllForTests, TestExecutor};
use operations::{Acceptor, Follower, Guard, MemoryDecisionLog, MemoryStorage, StartResponse};
use operations_zenoh::{Client, ClientConfig, FollowError, NoAnswer, Server, ServerConfig};

fn free_endpoint() -> String {
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port();
    format!("tcp/127.0.0.1:{port}")
}

#[test]
fn follow_that_gives_up_reports_the_lease_end_reason() {
    let core = Arc::new(Acceptor::new(
        catalog(),
        Guard::new(PermitAllForTests::new(), MemoryDecisionLog::new()),
        MemoryStorage::new(),
    ));
    let gecko = free_endpoint();
    let mut server_config = ServerConfig::new(vec![gecko.clone()], caller("follow-test"));
    // A short lease: the Observe stream ends with `lease` while the execution runs.
    server_config.observe_lease = Duration::from_millis(300);
    let server = Server::open(
        Arc::clone(&core),
        Box::new(TestExecutor::new()),
        server_config,
    )
    .expect("server");
    let client = Client::open(ClientConfig::new(vec![gecko])).expect("client");

    let request = echo_request("hi");
    let id = match client.start(&request).expect("start answered") {
        StartResponse::Accepted { operation, .. } => operation.id,
        other => panic!("expected acceptance, got {other:?}"),
    };

    // Zero reconnects: the single stream ends with `lease`, and that is the reason given.
    let result = client.follow(id, &mut Follower::new(), 0, |_, _| {});
    match result {
        Err(FollowError::GaveUp(NoAnswer(reason))) => {
            assert!(reason.contains("Lease"), "reason was: {reason}");
        }
        other => panic!("expected GaveUp with the lease reason, got {other:?}"),
    }

    client.close();
    server.close();
}
