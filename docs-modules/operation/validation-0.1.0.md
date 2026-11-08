# Operation `0.1.0-dev`: P1–P6 validation evidence and runbook

This is evidence and an operator runbook for GKO-67 (OP-A9). It owns no rules: the accepted rules are the [package table in the model](spec/model.md#31-agreed-first-experiment-010) and the [validation outline](spec/model.md#10-validation). It records what the code checks today, where, and what it does not check.

Status of this page: automated checks were run by the implementing agent on one host (Ubuntu 24.04 in WSL, Zenoh `1.10.2`, loopback). **Nobody has run the manual scenarios or signed the acceptance checklist below; that is for Farhat at GKO-68 and GKO-69.**

## 1. Run the checks

From the repository root, on Linux or in WSL ([environment](../../docs/development-environment.md)):

| Command | What it runs | Time |
|---|---|---|
| `make matrix` | Only the P1–P6 checks: Core directly and through Zenoh | under 10 s after the first build |
| `make demo` | The scripted scenario with a narrative (section 4) | about 2 s |
| `make check` | Everything: format, build, Clippy, all tests, smoke, demo, link and article checks | about 1 min |

Observed on 2026-11-08: `cargo test --workspace --all-features` passed 93 tests (Core: acceptance 13, contracts 19, dispatch 9, harness 3, lifecycle 23, observe 15; Zenoh: matrix 9, scenario 1, follow 1). The Zenoh matrix passed 8 repeated runs in a row.

## 2. Matrix: package, check, evidence

"Core" means the check calls the Core API directly (`crates/operations/tests`). "Zenoh" means it goes through the adapter and a real Zenoh session (`crates/operations-zenoh/tests/matrix.rs`, abbreviated `matrix.rs`; the scripted scenario is `demo.rs`).

### P1: boundary (Start, Get, Observe, minimal Zenoh, in-memory)

| Check | Core | Zenoh |
|---|---|---|
| Core works without the transport | all of `crates/operations/tests` | `demo.rs` step 1: Get over Zenoh equals the in-process read |
| Start, Get, Observe over Zenoh | n/a | `demo.rs` steps 1–6, `tests/scenario.rs` |
| No Cancel in `0.1.0` | `lifecycle.rs::cancelled_is_not_accepted_before_cancel_exists` | n/a |
| No restart durability (limit, not a check) | stated in the crate READMEs | [Gaps](#5-gaps-and-limits) |

### P2: states

| Check | Core | Zenoh |
|---|---|---|
| `pending` visible first, acknowledged handoff gives `running` | `acceptance.rs::acceptance_commits_mapping_record_and_pending_snapshot`, `lifecycle.rs::acknowledged_handoff_makes_the_execution_running_with_the_next_sequence` | `demo.rs` step 1 (`running`, sequence 2) |
| Lost acknowledgement is `unknown`, never redispatched, refined by evidence for the same token | `lifecycle.rs::lost_acknowledgement_is_unknown_and_never_redispatched`, `late_evidence_for_the_same_token_refines_unknown` | `matrix.rs::p2_p5_lost_ack_is_unknown_never_redispatched_and_refined_by_evidence` |
| A proven refusal is `failed` and stays failed | `lifecycle.rs::refused_handoff_is_a_proven_failure_and_stays_failed`, `dispatch.rs::a_refusal_that_cannot_be_recorded_is_never_downgraded_to_unknown` | n/a |
| Terminal outcome is immutable | `lifecycle.rs::terminal_outcome_is_immutable_and_arrival_order_does_not_decide` | `matrix.rs::p2_p5_terminal_state_is_immutable_over_zenoh` |
| Terminal state needs an authoritative outcome | `lifecycle.rs::terminal_outcome_confirms_a_pending_execution_whose_ack_was_lost` | n/a |

### P3: public API

| Check | Core | Zenoh |
|---|---|---|
| Typed requests and responses round-trip | `contracts.rs::requests_and_responses_round_trip` | `demo.rs` step 2 |
| Rejected and invalid Start create no operation | `acceptance.rs::denied_and_invalid_requests_create_no_operation` | `matrix.rs::p3_p4_rejected_starts_create_nothing_over_zenoh` |
| Typed outcomes travel as successful replies; no answer is retryable | n/a | `demo.rs` steps 2–3 |
| Get returns the latest full snapshot or unavailability; Z5 (unowned = nonexistent) | `observe.rs::get_returns_the_latest_full_snapshot_at_each_stage`, `unowned_nonexistent_and_denied_reads_are_indistinguishable` | `demo.rs` step 2 |
| Late observer gets current then later snapshots; completed work gives the final snapshot | `observe.rs::late_observer_of_running_work_gets_current_then_later_snapshots`, `late_observer_of_completed_work_gets_the_final_snapshot_and_no_stream` | `demo.rs` step 6 |
| Resume by `sequence`: no duplicates, gap disclosed, future cursor is invalid | `observe.rs::resume_suppresses_duplicates_and_discloses_gaps`, `reconnect_resumes_without_duplicates_or_restarting_work` | `demo.rs` steps 4–5 |
| Slow observer is detectable and never blocks Core | `observe.rs::slow_observer_is_ended_as_lagged_and_never_blocks_core`, `one_slow_observer_does_not_affect_others` | `matrix.rs::p3_slow_observer_is_ended_as_lagged_and_resumes_with_a_gap`, `p3_follow_survives_a_lagged_stream_and_ends_on_the_core_snapshot` |
| Attach racing the final publication never loses it | `observe.rs::attaching_while_the_final_snapshot_is_published_never_loses_it` (300 rounds) | `matrix.rs::p3_p5_attach_racing_the_final_publication_never_loses_it` (40 rounds) |
| Observing never starts, resumes or cancels work | `observe.rs::reading_and_observing_never_start_resume_or_cancel_execution` | `demo.rs` step 6 (nothing handed to the executor) |

### P4: identity and retry

| Check | Core | Zenoh |
|---|---|---|
| Lost response, same-content retry: one execution, same identity | `acceptance.rs::lost_response_retry_returns_the_same_operation_without_a_second_dispatch` | `demo.rs` step 3 |
| Changed content conflicts; the original is unchanged | `acceptance.rs::changed_content_conflicts_and_leaves_the_original_unchanged` | `demo.rs` step 3 |
| Concurrent same-content retries: one execution | `acceptance.rs::concurrent_same_content_retries_create_one_execution` (and `…_after_a_commit_failure_…`) | `matrix.rs::p4_concurrent_same_content_retries_create_one_execution` (8 threads) |
| Concurrent changed content: one winner | `acceptance.rs::concurrent_requests_with_different_content_have_one_winner` | `matrix.rs::p4_concurrent_requests_with_changed_content_have_one_winner` (6 threads) |
| Scope is the receiving Gecko and caller; tracing metadata ignored; normalization | `acceptance.rs::the_retry_key_is_scoped_to_the_caller`, `…_to_the_receiving_gecko`, `retry_ignores_tracing_metadata`, `equality_is_normalized_…`; `contracts.rs::retry_comparison_*` | n/a |
| UUIDv4 identifiers | `contracts.rs::identifiers_must_be_uuid_v4` | n/a |

### P5: executor

| Check | Core | Zenoh |
|---|---|---|
| Handoff once, with the issued token; ordered, identified events | `lifecycle.rs::a_dispatch_hands_off_once_with_the_issued_token`, `snapshots_are_full_and_numbered_without_gaps` | `demo.rs` step 4 |
| Duplicate, stale, out-of-order, unknown-token events | `lifecycle.rs::duplicates_and_repeated_acks_do_not_increment_the_sequence`, `stale_out_of_order_and_unknown_events_are_rejected_without_change` | n/a |
| A rejected event consumes its number (Z6) | `lifecycle.rs::rejected_event_consumes_its_number_so_later_events_proceed`, `resending_a_rejected_event_returns_the_same_rejection`, `conflicting_terminal_consumes_its_number` | n/a |
| Concurrent copies and competing outcomes | `lifecycle.rs::concurrent_copies_of_one_event_apply_once`, `competing_outcomes_on_one_event_number_leave_exactly_one_terminal` | n/a |
| Ambiguous handoff is not retried | see P2 | see P2 |

### P6: accounting and trust

| Check | Core | Zenoh |
|---|---|---|
| Mapping, record and first snapshot commit together before handoff | `acceptance.rs::acceptance_commits_mapping_record_and_pending_snapshot` | n/a |
| A failed commit accepts and dispatches nothing; the retry creates one execution | `acceptance.rs::commit_failure_accepts_nothing_dispatches_nothing_and_stores_no_mapping`, `concurrent_retries_after_a_commit_failure_create_exactly_one_execution` | `matrix.rs::p6_failed_commit_accepts_and_dispatches_nothing_then_retry_succeeds` |
| Dispatch failures are recorded, not left `pending` (O8, O9) | `dispatch.rs` (9 tests) | n/a (the server logs `Unrecorded` to stderr) |
| Identity comes from the adapter, not the body | `contracts.rs::payload_cannot_carry_a_caller_identity` | `ServerConfig::caller` |
| Authorization is called and its decision recorded before handoff | `contracts.rs::every_public_command_is_authorized_and_recorded`, `handoff_requires_a_recorded_permit`, `unrecorded_decision_yields_no_proof_and_no_handoff` | n/a |
| Denial maps to a typed response without leaking the reason | `contracts.rs::refusals_map_to_typed_responses_without_leaking_the_reason` | n/a |

## 3. Deterministic controls

| Failure to inject | Control | Where |
|---|---|---|
| Lost Start response | `Server::set_start_reply_delay` longer than the client timeout | `demo.rs` step 3 |
| Cut link | `Proxy::cut` and `Proxy::restore` | `demo.rs` step 4 |
| Lost executor acknowledgement | `Handoff::Ambiguous` from the executor | `matrix.rs` (`Shared::answer_next`), `lifecycle.rs` |
| Failed commit or storage update | `MemoryStorage::inject_commit_failures`, `FlakyStorage` | `acceptance.rs`, `dispatch.rs`, `matrix.rs` |
| Changed-content retry | same `request_id`, other parameters | `acceptance.rs`, `matrix.rs`, `demo.rs` |
| Concurrent retries | threads released by a `Barrier` | `acceptance.rs`, `matrix.rs` |
| Observation lag | `Acceptor::with_observer_capacity` plus `Server::set_observe_paused` (new in GKO-67) | `matrix.rs` |
| Final-publication race | a terminal event and an attach released together, repeated | `observe.rs`, `matrix.rs` |

A check that cannot fail proves nothing. The lag check was verified by turning `set_observe_paused` into a no-op: `p3_slow_observer_is_ended_as_lagged_and_resumes_with_a_gap` then fails. The Core attach order was verified earlier (GKO-65).

## 4. Operator runbook: manual scenario

There is no separate server binary in `0.1.0`: the demo starts the Gecko, the executor, and two clients in one process, so there is nothing to start or clean up. Ports are chosen free at run time. Stop a stuck run with Ctrl-C; it leaves no files or daemons.

```sh
make demo
```

Compare each line with the expected value. Identifiers (operation ids, ports) differ per run; counters below are deterministic unless marked.

| Step | Field | Expected | Actual (run on 2026-11-08) |
|---|---|---|---|
| 1 | Get over Zenoh, state and `sequence` | `Running`, 2 | `Running`, 2 |
| 1 | Observe address | `gecko/g1/op/<operation id>/observe` | matched |
| 2 | unknown type / missing parameter / malformed payload | `rejected(unknown_operation_type)` / `rejected(invalid_parameters)` / `invalid_input` | matched |
| 2 | unknown id / key without id | `unavailable` / `invalid_input` | matched |
| 3 | first attempt (reply after the timeout) | `no answer` | `no answer: error reply: Timeout` |
| 3 | executions created by the lost reply plus retry | 1 | 1 |
| 3 | same `request_id`, other content | `rejected(identifier_conflict)` | matched |
| 4 | last snapshot | `Succeeded`, sequence 8 (2 + five progress + terminal) | `Succeeded`, 8, result `echo: hello` |
| 4 | snapshots seen across the cut link | strictly increasing; every number from first to last seen or in a disclosed gap | 2, 3, 4, then 8 with gap 5..=7 (the gap itself varies with timing) |
| 5 | `after=2` | latest (8) and gap `3..=7` | matched |
| 5 | `after=8` / `after=13` | nothing redelivered / `invalid_input` | matched |
| 6 | late observer | final snapshot, stream ends `terminal`, executor handed nothing | matched |

Exit code 0 and the final line `scenario complete` mean every check passed; a failed check prints `check failed: <what>` and exits non-zero.

To watch the counters on a single execution, follow the sequence numbers: `pending` is 1, handoff acknowledged is 2, each executor event that creates a snapshot adds 1, and a rejected or duplicate event adds nothing. The execution count is `MemoryStorage::operation_count`, asserted in `matrix.rs` and `demo.rs`.

## 5. Gaps and limits

- Not covered because it is not built: Cancel cases of the [validation outline](spec/model.md#10-validation) (`0.2.0`), restart and republish of a retained snapshot, and recovery after a crash (`0.3.0`). The outline row "inject failures around handoff and recording" is covered at Core for storage and handoff failures (`dispatch.rs`) but not as a crash.
- Everything is one host, loopback, in memory, threads in one process. Not measured: more than one host, multi-hop routing, half-open links, Gecko restart, load, and separate-process clients.
- The Zenoh attach race runs 40 rounds and has not been checked against a deliberately broken server; the Core race runs 300 rounds and has.
- `p3_follow_survives_a_lagged_stream_and_ends_on_the_core_snapshot` also passes without the pause hook, because `follow` often discloses a gap without a lag. It proves `follow` ends on the Core snapshot with gaps disclosed; the lag itself is proven by the other lag test.
- Lag is produced by a server hook that stops reading Core's stream. It does not show how Zenoh congestion would behave with a real slow client.
- Identity is adapter configuration, not authentication; the permissive test policy is allowed only in this restricted deployment (P6, [security](spec/security.md)).
- Open, unchanged by this work: whether stale and conflicting notifications are protocol errors or ordinary rejection outcomes (Z6 in [decision status](spec/status.md)).
- No defect was found while writing these checks, so none is linked.

## 6. Human acceptance (not yet done)

For GKO-68 and GKO-69. Unchecked until Farhat runs it.

- [ ] `make check` passes on the reviewer's checkout.
- [ ] `make demo`: the table in section 4 matches, including the cut-link step.
- [ ] Each package P1–P6 has a row in section 2 the reviewer accepts as evidence.
- [ ] The gaps in section 5 are acceptable for `0.1.0`.
- [ ] Choices O1–O7 in [decision status §2.5](spec/status.md#25-implementation-choices-from-gko-64-gko-65-and-gko-66) are confirmed or corrected.
