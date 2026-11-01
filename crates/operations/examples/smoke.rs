//! Smoke run of the harness: queues outcomes on a `TestExecutor`, hands off
//! fixture tokens, and checks the recorded results.

use operations::testing::{TestExecutor, fixtures};
use operations::{Executor, Handoff};

fn main() {
    let mut executor = TestExecutor::new();
    executor
        .queue_response(Handoff::Refused {
            reason: "smoke: refused".to_string(),
        })
        .queue_response(Handoff::Ambiguous);

    let outcomes: Vec<Handoff> = (1..=3)
        .map(|n| executor.hand_off(&fixtures::execution_token(n)))
        .collect();

    println!(
        "operations smoke: {} handoffs recorded",
        executor.dispatched().len()
    );
    for (token, outcome) in executor.dispatched().iter().zip(&outcomes) {
        println!("  {} -> {outcome:?}", token.as_str());
    }

    assert_eq!(
        outcomes,
        vec![
            Handoff::Refused {
                reason: "smoke: refused".to_string(),
            },
            Handoff::Ambiguous,
            Handoff::Acknowledged,
        ]
    );
    assert_eq!(executor.dispatched().len(), 3);

    println!("operations smoke: ok");
}
