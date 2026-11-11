# Operation `0.1.0-dev`: manual lab runbook (GKO-68)

This page lists the manual scenarios for GKO-68 (OP-A10) and holds the results. It owns no rules: expected results restate the [package table](spec/model.md#31-agreed-first-experiment-010) and the [validation outline](spec/model.md#10-validation), and the automated evidence is on the [validation page](validation-0.1.0.md). The stand is [`tools/operations-lab`](../../tools/operations-lab/README.md).

Status: the stand is built and was smoke-checked by the implementing agent (section 4). Farhat ran all cases L1 to L13 on 2026-11-11 and the results are in section 3 (all Pass; notes on what each run did and did not exercise are in the rows). The Actual and Pass columns are for Farhat only.

## 1. Before the run

Record the environment once:

| Field | Value |
|---|---|
| Commit (`git rev-parse HEAD`) | |
| Host and OS (`uname -a`; WSL distribution) | |
| Rust (`rustc --version`) | |
| Date | |
| Server and client log files | |

Start the stand from the repository root, each in its own terminal:

```sh
make lab-server                          # terminal A
make lab-client                          # terminal B: direct
make lab-client LAB_ARGS=--via-proxy     # terminal C: through the cuttable proxy
```

Keys are in the [stand README](../../tools/operations-lab/README.md). Unless a case says otherwise, the server starts with: acknowledge, 3 steps every 1000 ms, then succeed; no faults. Use `n` in the client for a new `request_id` at the start of each case. "Server table" means the executions table and Core totals on the server screen.

## 2. Cases

Each case names what to inspect: identities, handoffs (`hnd`, the dispatch count), `sequence`, partial results, and errors.

### L1. Accepted and rejected Start (P3)

| Step | Do | Expected |
|---|---|---|
| 1 | B: `s` | `accepted <id>`; server: a new row, `hnd` 1, runs to `succeeded` |
| 2 | B: set `type` to `no.such.type`, `n`, `s` | `rejected(UnknownOperationType)`; server: no new row, totals unchanged |
| 3 | B: type back to `test.echo`, clear `message`, `n`, `s` | `rejected(InvalidParameters)`; no new row |
| 4 | B: `message` back, `repeat` = `three`, `n`, `s` | `rejected(InvalidParameters)` |
| 5 | B: `m` | `invalid_input: malformed Start request` |
| 6 | B: `n`, copy the new `request_id` into `operation id` (any UUIDv4 that is not an operation), `g` | `unavailable` |
| 7 | B: `operation id` = `abc`, `g` | `invalid_input`: the key has no operation id |

### L2. Lost Start reply and retry with the same `request_id` (P4)

| Step | Do | Expected |
|---|---|---|
| 1 | A: `d` until the reply delay is 3000 ms | |
| 2 | B: `n`, `s` | after 2 s: `no answer ... Timeout`; server: the row already exists, `hnd` 1 |
| 3 | A: `d` until 0 ms; B: `s` (same `request_id`) | `accepted` with the same id as the row; totals: operations +1 in total, `hnd` stays 1 |
| 4 | Server log | the first Start's delayed reply is also sent, with the same id |

### L3. Same `request_id`, changed content (P4)

| Step | Do | Expected |
|---|---|---|
| 1 | After L2, B: change `message`, `s` | `rejected(IdentifierConflict)`; no new row; the original row unchanged |

### L4. Concurrent callers (P4)

| Step | Do | Expected |
|---|---|---|
| 1 | B: `n`, `b` (burst size 8) | every caller `accepted` with one id; summary `1 distinct accepted operation(s)`; server: one new row, `hnd` 1 |
| 2 | B: `n`, `B` | summary: one accepted, the other 7 `IdentifierConflict`; one new row |
| 3 | B: `n`, copy the `request_id` into C's form (same `message`); `s` in B and C close together | both answers carry the same id; one row |

Two windows cannot be released at the same instant by hand; step 1 and 2 are the real concurrency.

### L5. Progress, partial results and the terminal result (P2, P3)

| Step | Do | Expected |
|---|---|---|
| 1 | A: steps 5, interval 1000 ms; B: `n`, `s`, `o` | snapshots with strictly increasing `seq` (2, then 3 to 7 with `step i/5` and `partial i/5`), then seq 8 `succeeded` with `done after 5 steps`, stream ended `terminal`; violations 0 |
| 2 | B: `g` | the same final snapshot |
| 3 | A: `o` to "fail"; B: `n`, `s`, `o` | the last progress, then `failed` with the last partial result kept |
| 4 | A: select the finished row, `s` | `rejected` at Core (terminal is immutable); `seq` unchanged |

### L6. `unknown` after a lost acknowledgement (P2, P5)

| Step | Do | Expected |
|---|---|---|
| 1 | A: `a` until "lose acknowledgement"; B: `n`, `s` | `accepted`; server: the row is `unknown`, seq 2, `hnd` 1; no automatic progress |
| 2 | B: `g` after a while | still `unknown`; `hnd` still 1 (never redispatched) |
| 3 | A: select it, `k` (ack) | `running` (late evidence for the same token) |
| 4 | A: `s` | `succeeded` |
| 5 | Repeat 1, then A: `f` straight from `unknown` | `failed` |

### L7. Refused and slow handoff (P2, P5)

| Step | Do | Expected |
|---|---|---|
| 1 | A: `a` until "refuse"; B: `n`, `s`, `g` | `accepted`; Get: `failed`, seq 2, the refusal in the explanation; `hnd` 1 |
| 2 | A: `a` until "slow acknowledge"; B: `n`, `s` | server: the row is `pending` for 3 s, then `running`. The Start reply comes after the handoff, so with the 2 s timeout B sees `no answer`; `s` again returns the same id |

### L8. Late attach (P3)

| Step | Do | Expected |
|---|---|---|
| 1 | Finish an operation in B; paste its id into C's `operation id`; C: `o` | attached with the final snapshot, stream ended `terminal`; server: `hnd` unchanged |
| 2 | A: steps 10, interval 1000 ms; B: `n`, `s`; paste the id into C while running; C: `o` | attached with the current snapshot, then the later ones, then `terminal`; violations 0 |

### L9. Reconnect across a cut link (P3)

| Step | Do | Expected |
|---|---|---|
| 1 | A: steps 10, interval 1000 ms; C (proxy): `n`, `s`, `o` | snapshots arrive |
| 2 | A: `l` (cut) | C: header `NO LINK`; the stream ends with `no answer`; the execution keeps running on the server |
| 3 | Wait 3 s, A: `l` (restore); wait for `link up` in C | |
| 4 | C: `r` | attached at the latest seq with `GAP a..=b disclosed` for the missed snapshots; then the rest to `terminal`; violations 0 |

If `r` gets `no answer`, the session has not reconnected yet: wait for `link up` and press `r` again. The work is not restarted (`hnd` 1).

### L10. Slow observer (P3)

| Step | Do | Expected |
|---|---|---|
| 1 | A: steps 12, interval 200 ms; B: `n`, `s`, `o` | |
| 2 | A: `w` (pause) for about 3 s, then `w` | B: buffered snapshots, then `stream ended: lagged`; other observers and the execution are not affected |
| 3 | B: `r` | attached with the gap disclosed, then `terminal`; violations 0 |

### L11. Attach near the final publication (P3, P5)

| Step | Do | Expected |
|---|---|---|
| 1 | A: steps 0, then "hold"; B: `n`, `s`; paste the id into C | the row stays `running` |
| 2 | Press A `s` and C `o` at about the same moment; repeat with new operations 5 times | every attach ends `terminal` with the final snapshot; none waits for the lease |

A human cannot force the exact interleaving; the forced orders are covered by `observe.rs` and `matrix.rs` ([validation page](validation-0.1.0.md#2-matrix-package-check-evidence)).

### L12. Failed commit (P6)

| Step | Do | Expected |
|---|---|---|
| 1 | A: `c`; B: `n`, `s` | `error: the acceptance could not be committed; nothing was started`; no new row; totals unchanged |
| 2 | B: `s` (same `request_id`) | `accepted`; one new row, `hnd` 1 |

### L13. Duplicate executor event (P5)

| Step | Do | Expected |
|---|---|---|
| 1 | A: select a running or finished row, `r` | `duplicate, no new snapshot`; `seq` unchanged |

## 3. Results

Fill one row per case. A failed case gets a linked corrective Linear issue and a retest row.

| Case | Actual (short) | Pass | Log lines or screenshot | Issue |
|---|---|---|---|---|
| L1 | All 7 steps gave the expected answer. 1: Start returned `accepted` 8801dbc0. 2: `rejected(UnknownOperationType): operation type `no.such.type` is not supported`. 3: with `message` cleared, `rejected(InvalidParameters): ... 1 invalid field(s)`. 4: with `repeat` = "three", `rejected(InvalidParameters): ... 1 invalid field(s)`. 5: `m` returned `invalid_input: malformed Start request: expected ident at line 1 column 2`. 6: Get with a `request_id` as the operation id returned `unavailable`. 7: Get with `abc` returned `invalid_input: the key does not contain an operation id`. Rejected and invalid requests created no new row (observed by Farhat). | Pass | Client 04:50:11.318-04:52:09.346 | |
| L2 | With the reply delay at 3000 ms, Start (`request_id` fc29fa7e) timed out: client `no answer: error reply: Timeout (the operation may exist; retry with the same request_id)` at 04:58:39.624, 2 s after the send. On the server the handoff was acknowledged at 04:58:37.625 and the row b486d720 already existed (`hnd` 1); the delayed reply was sent at 04:58:40.625 with the same id. With the delay back at 0 ms, Start with the same `request_id` returned `accepted b486d720-f8b7-4c32-ab20-dd4101a35f1b` (04:58:49.458), the same id as the row. The operation ran to `succeeded` (seq 6); the table shows one row, `hnd` 1, Core totals 1/1/1, so the retry created no new operation and no new handoff (observed by Farhat). | Pass | Client 04:58:37.623-04:58:49.458; server 04:58:37.625-04:58:49.457 and the executions table | |
| L3 | Same `request_id` 4f33b682 with `message` "hello" changed to "hello1": `rejected(IdentifierConflict): this request_id was already used with different content`. The original operation 4ea9b276 had already run to `succeeded` (snapshot 6) and no further handoff or event followed the rejection. Core totals did not change and no new row appeared (observed by Farhat). | Pass | Client 08:27:47.629-630; server 08:27:47.630 `rejected(identifier_conflict)` (client connected via the proxy, 7448) | |
| L4 | Step 1 (`b`): 8 concurrent Starts with one `request_id`, all `accepted` with the same id b7f6cf77; summary `1 distinct accepted operation(s)`; one handoff. Step 2 (`B`): 8 Starts with different messages, 1 `accepted` (e93caacb, caller 7) and 7 `IdentifierConflict`; one handoff. Step 3 (two windows, same `request_id` and `message`) done by Farhat: one operation. Core totals ended at 3 (one per step). | Pass | Client 08:42:47.087-094 and 08:42:53.072-078; server 08:42:47.092-093 and 08:42:53.075; step 3 reported by Farhat, no log lines pasted | |
| L5 | Step 1 (5 steps, succeed): operation 6ef6fe66, Observe attached at seq 2, then seq 3-7 (`step i/5`, `partial i/5`), seq 8 `succeeded` "done after 5 steps", stream ended `terminal`. Step 2: Get returned seq 8 `succeeded`, the same final snapshot. Step 3 (fail): operation 05cd3c4d, seq 3-7, seq 8 `failed` with `partial 5/5` kept, stream ended `terminal`; Observe panel: held seq 8, violations 0. Step 4: the server table shows the failed row at seq 8 with 8 executor events sent (6 applied in the log), so the extra events did not change `seq` or state. Handoffs 1 per row; Core totals 2/2/2. While `running`, the explanation stays "accepted; waiting for the executor to confirm" (known observation, section 4). | Pass | Client 08:45:04-08:46:03; server 08:45:04-08:46:03 and the executions table; the step 4 log lines were not pasted | |
| L6 | Steps 1-2: with "lose acknowledgement", Start returned `accepted` (eb7495af); the handoff was logged as `ack lost`; two Gets (15:32:25 and 15:32:43) returned `unknown` seq 2 "handoff_unconfirmed" and no progress followed; `hnd` 1. Steps 3-4: `ack` applied (event #1, snapshot 3), then `succeed` applied (event #2, snapshot 4); Get returned `succeeded` seq 4 and Observe attached at seq 4 and ended `terminal`. Step 5: on the new operation 6bc24b7c (`ack lost`), `fail` straight from `unknown` was applied (event #1, snapshot 3); the table shows `failed` seq 3, `hnd` 1. Core totals 2/2/2. The `running` state after the `ack` was not looked at on screen (no Get between `ack` and `succeed`); only the applied event is in the log. | Pass | Client 15:31:54-15:34:29; server 15:31:54-15:35:27 and the executions table | |
| L7 | Step 1 (refuse): Start returned `accepted` (74c9d0c6); the handoff was logged as `refused`; Get returned `failed` seq 2 "the executor refused the handoff before any side effect: the lab executor was set to refuse"; the table shows `fail`, seq 2, `hnd` 1, `ev` 0. Step 2 (slow acknowledge): operation 3ae2065b; the server received the handoff at 03:37:45.391 and held the acknowledgement 3000 ms (`acknowledged` at 03:37:48.391); the client got `no answer: Timeout` at 03:37:47.391 (2 s) before the reply; the retry with the same `request_id` returned `accepted` with the same id; the operation ran 3 steps to `succeeded` (seq 6), `hnd` 1. Core totals 2/2/2. The `pending` state during the 3 s was not looked at on screen; only the log timing shows it (observed by Farhat). | Pass | Client 03:30:14-03:30:19 and 03:37:44-03:38:11; server 03:30:15-03:30:19, 03:37:45-03:37:52 and 03:38:11, and the executions table | |
| L8 | Step 1: operation 178ed462 (3 steps, succeed) ran to `succeeded` (seq 6) from B. Client C (observe by that id, `after=-`) attached at seq 6 with the final snapshot `succeeded` "step 3/3" result "done after 3 steps", the stream ended `terminal`; Observe panel: held seq 6, snapshots 1, violations 0. The server table showed `hnd` 1 and Core totals 1/1/1, so the late attach started no handoff. Step 2 (run with 5 steps every 3000 ms instead of the 10 steps every 1000 ms in the runbook; the check is the same): operation 034813ae, C attached while `running` at seq 4 (`step 2/5`, `partial 2/5`), then received seq 5, 6, 7 (`step 3/5` to `step 5/5`) and seq 8 `succeeded` "done after 5 steps", stream ended `terminal`; Observe panel: held seq 8, snapshots 5, violations 0. Server table: `succeeded` seq 8, `hnd` 1, Core totals 1/1/1 (the second server run, after a restart) (observed by Farhat). | Pass | Step 1: client C 03:44:48.182-184, server 03:44:35-03:44:48. Step 2: client C 04:01:20.417-04:01:31.710, server 04:01:13-04:01:31 and the executions table | |
| L9 | Operation 683e8783 (10 steps every 1000 ms) through the proxy. C observed from seq 2 and received seq 3 and 4. The link was cut at 04:05:44.932; C logged `no answer: the stream closed without an end; resume with the held sequence` at 04:05:44.933. The execution kept running on the server (events #3-#11, snapshots 5-13, `succeeded`), and the link was restored at 04:05:48.795. `r` was pressed at 04:06:03.892, after the operation had already finished, so C attached with `after=4` at seq 13 with `GAP 5..=12 disclosed` and the final snapshot `succeeded` "done after 10 steps", then the stream ended `terminal`; Observe panel: held seq 13, violations 0. Server table: `succeeded` seq 13, `hnd` 1, Core totals 1/1/1, so the work was not restarted. Because `r` came after the end, the "then the rest to `terminal`" part was not exercised in this first run. Rerun with `r` while the operation was still running: operation f8036487 (10 steps every 1000 ms), C observed from seq 2 and received seq 3; the link was cut at 04:07:50.922 (C: `no answer: the stream closed without an end`) and restored at 04:07:53.561; `r` at 04:07:55.475 attached with `after=3` at seq 8 (`step 6/10`) with `GAP 4..=7 disclosed`, then received seq 9-12 and seq 13 `succeeded` "done after 10 steps", stream ended `terminal`; Observe panel: held seq 13, snapshots 8, violations 0. Server table: `succeeded` seq 13, `hnd` 1, Core totals 1/1/1 (a restarted server). The `NO LINK` header and the `link up` message were not captured in the pasted material (observed by Farhat). | Pass | First run: client C 04:05:42.827-04:06:03.894, server 04:05:42-04:06:03. Rerun: client C 04:07:48.937-04:07:59.943, server 04:07:48-04:07:59 (CUT 04:07:50.922, restored 04:07:53.561) and the executions table | |
| L10 | First run, not the case as written: the server stayed at 12 steps every 1000 ms (the runbook asks for 200 ms). Operation 4d0e84d0; observe was paused 04:20:58.387-04:21:00.987 (about 2.6 s), during which only 3 snapshots were produced (seq 5-7). After the resume B received seq 5, 6, 7 at 04:21:01.013-014 and seq 8, so nothing was lost and **no `stream ended: lagged` occurred**. B then pressed `r` at 04:21:02.588 while the stream was healthy ("observe stream dropped by the client"), attached with `after=8` at seq 8 "nothing redelivered" (no gap, nothing to disclose), received seq 9-15, `succeeded` "done after 12 steps", stream ended `terminal`; violations 0, `hnd` 1, Core totals 1/1/1. So the lagged end, the disclosed gap after it, and the unaffected execution under lag were not exercised. Rerun with 50 steps every 200 ms: operation 304123e1, B observed from seq 3 and received snapshots up to seq 13 at 04:26:03.823; during the pause no snapshot reached B and the execution kept running on the server (events #17-#28 at 200 ms spacing, snapshots up to 30). After the resume (04:26:07.390) B received a short burst of buffered snapshots, seq 14-17 at 04:26:07.429-432, and then `stream ended: lagged; resume with the held sequence` (server: `stream ended lagged` at 04:26:07.429). `r` at 04:26:11.607 attached with `after=17` at seq 51 with `GAP 18..=50 disclosed`, then seq 52 and seq 53 `succeeded` "done after 50 steps", stream ended `terminal`; Observe panel: held seq 53, snapshots 18, violations 0. Server table: `succeeded` seq 53, `hnd` 1, Core totals 1/1/1; events kept being applied every 200 ms through the lag, so the execution was not affected. Only one observer was attached, so "other observers are not affected" was not exercised (observed by Farhat). | Pass | First run: client B 04:20:55.705-04:21:08.710, server 04:20:55-04:21:08. Rerun: client B 04:26:01.618-04:26:11.847, server 04:26:05-04:26:11.831 (resumed 04:26:07.390, lagged 04:26:07.429); the pause line was not in the pasted server log | |
| L11 | Server set to 0 steps, then hold. Step 1: operation ff39c3e8 was accepted, logged `script done; holding`, and stayed `running` until finished by hand. Step 2: one first operation plus 5 repeats, 6 operations in all (ff39c3e8, d5cf59e7, 2f894b88, 2c918044, ca15b3b8, 529cc507). In every run C attached and got `succeeded` "finished by hand" at seq 3, the stream ended `terminal` right away (within 3 ms of the attach in the client log), no wait for the 20 s lease; the Observe panel for the last one: held seq 3, snapshots 1, violations 0. Server table: all 6 rows `succeeded` seq 3, `hnd` 1, Core totals 6/6/6. In the runs where the server log shows both lines the manual finish came 0.3-1 s before the attach (for example d5cf59e7: terminal 04:35:16.508, attach 04:35:17.405; 2f894b88: 04:35:42.850 and 04:35:43.153; 2c918044: 04:36:00.319 and 04:36:00.776), so the attach landed after the final publication every time and the near-simultaneous interleaving itself was not hit, as the runbook expects of a manual run. The terminal lines of ca15b3b8 and 529cc507 are not in the pasted server log; the table and C's log show the same result (observed by Farhat). | Pass | Client C 04:34:37.927-04:36:34.093; server 04:29:52-04:36:00 and the executions table | |
| L12 | Step 1: with `commit` set to "inject 1 failure" (server log: "the next commit of a new execution will fail"), Start with `request_id` 3c2c1e75 returned `error: the acceptance could not be committed; nothing was started`; no new row, no handoff, Core totals stayed at 0 (observed by Farhat; the table was empty). Step 2: Start with the same `request_id` returned `accepted` a066254a; one new row, `hnd` 1, the operation ran 3 steps to `succeeded` (seq 6); Core totals ended at 1/1/1. Totals before step 1 were 0 (empty table), not recorded from a separate screen capture. | Pass | Client B 04:46:33.869-880 and 04:46:59.009-012; server 04:46:24.313, 04:46:33.871 and 04:46:59.012-04:47:03.013 and the executions table | |
| L13 | On the finished operation a066254a (`succeeded`, seq 6), `r` on the server resent event #4 (`terminal succeeded`): `resent -> duplicate, no new snapshot (still 6)` at 04:48:38.336. The table after the resend shows `succeeded`, seq 6, `hnd` 1, 4 events in the `ev` column, and Core totals 1/1/1, so the duplicate changed neither `seq` nor the state. Only the terminal event of a finished row was resent; a running row was not tried (observed by Farhat). | Pass | Server 04:48:38.336 and the executions table | |

## 4. Agent smoke check of the stand (not the manual result)

On 2026-11-08 the implementing agent ran both screens in `tmux` on the development host (WSL, Zenoh `1.10.2`, loopback) and drove L2, L5, L6, L7, L9, L10 and L12 by key presses. Every one behaved as expected above. Observations to look at during the manual run:

- **Stale explanation.** While `running`, every snapshot still carries the explanation from acceptance: "accepted; waiting for the executor to confirm". The acknowledgement and progress events do not replace it. Whether that is acceptable is a contract question, not a stand defect.
- **Reconnection delay.** After the proxy link is restored, the Zenoh client session reconnected in about 3 s; a resume before that gets `no answer`.
- **Clock steps.** The WSL wall clock stepped back by up to about 2.5 s during the run, so timestamps in a log can go backwards. Line order within one log file is the real order.
- With the 2 s client timeout, a slow acknowledgement (3 s) makes Start time out, because the reply is sent after the handoff.
