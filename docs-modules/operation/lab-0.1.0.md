# Operation `0.1.0-dev`: manual lab runbook (GKO-68)

This page lists the manual scenarios for GKO-68 (OP-A10) and holds the results. It owns no rules: expected results restate the [package table](spec/model.md#31-agreed-first-experiment-010) and the [validation outline](spec/model.md#10-validation), and the automated evidence is on the [validation page](validation-0.1.0.md). The stand is [`tools/operations-lab`](../../tools/operations-lab/README.md).

Status: the stand is built and was smoke-checked by the implementing agent (section 4). **No case below has been run by Farhat. The Actual and Pass columns are for Farhat only.**

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
| L1 | | | | |
| L2 | | | | |
| L3 | | | | |
| L4 | | | | |
| L5 | | | | |
| L6 | | | | |
| L7 | | | | |
| L8 | | | | |
| L9 | | | | |
| L10 | | | | |
| L11 | | | | |
| L12 | | | | |
| L13 | | | | |

## 4. Agent smoke check of the stand (not the manual result)

On 2026-11-08 the implementing agent ran both screens in `tmux` on the development host (WSL, Zenoh `1.10.2`, loopback) and drove L2, L5, L6, L7, L9, L10 and L12 by key presses. Every one behaved as expected above. Observations to look at during the manual run:

- **Stale explanation.** While `running`, every snapshot still carries the explanation from acceptance: "accepted; waiting for the executor to confirm". The acknowledgement and progress events do not replace it. Whether that is acceptable is a contract question, not a stand defect.
- **Reconnection delay.** After the proxy link is restored, the Zenoh client session reconnected in about 3 s; a resume before that gets `no answer`.
- **Clock steps.** The WSL wall clock stepped back by up to about 2.5 s during the run, so timestamps in a log can go backwards. Line order within one log file is the real order.
- With the 2 s client timeout, a slow acknowledgement (3 s) makes Start time out, because the reply is sent after the handoff.
