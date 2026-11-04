# GKO-60 spike: Zenoh mapping, wire types and observation races

Status: research result for GKO-60, November 2, 2026, revised after the [first review](https://linear.app/hakalem-echo-team/issue/GKO-60/research010-spike-zenoh-mapping-wire-types-and-observation). Measured on one host. Nothing here changes accepted rules; the consequential choices are registered as Z1–Z6 in [decision status §2.3](../../spec/status.md#23-choices-raised-by-the-zenoh-spike-gko-60). Code: this directory (`docs-modules/operation/research/zenoh-spike/`).

## Pinned setup

- `zenoh = "=1.10.2"` and `zenoh-ext = "=1.10.2"` with feature `unstable`, used only by the `advanced` scenario. 1.10.2 was released on 2026-10-28; the repository has no version-maturity rule. The exact dependency tree is fixed by the checked-in `Cargo.lock`.
- Crate edition 2024, so it needs rustc ≥ 1.85. Zenoh's own MSRV is 1.75. Built with the repository toolchain, rustc 1.99.1 from the root `rust-toolchain.toml`, on Ubuntu-24.04 / WSL2 (kernel 6.6.87.2), x86_64. Other dependencies: tokio 1 (multi-thread, 4 workers), serde_json, uuid v4.
- Topology: one process. The "Gecko" session runs in `peer` mode and listens on `tcp/127.0.0.1:<free>`. Caller sessions run in `client` mode and connect either directly or through an in-process TCP proxy that can be cut (closing all TCP connections) and restored. Multicast and gossip scouting are disabled.
- Reproduce from this directory:
  - `cargo test --release` runs the canonicalization unit tests.
  - `cargo run --release -- all` runs every scenario. Single scenarios: `start`, `observe-race`, `slow`, `reconnect`, `executor`, `reconnect-observe`, `slow-observe`, `advanced`.
- Raw output of three consecutive full runs of this revision: [run 1](results-run1.txt), [run 2](results-run2.txt), [run 3](results-run3.txt). The single run of the first draft is in git history at `b9c95ed`. The crate has its own `[workspace]` and is not part of the root workspace.

Keys:

| Key | Mechanism | Used for |
|---|---|---|
| `gecko/g1/op/start` | queryable | Start |
| `gecko/g1/op/<op_id>/report` | queryable | Get |
| `gecko/g1/op/<op_id>/observe?after=<seq>` | queryable | streaming Observe |
| `gecko/g1/op/<op_id>/snapshot` | pub/sub | pub/sub Observe |
| `gecko/g1/alive` | liveliness token | reconnect signal |

Zenoh selector parameters are separated by `;`, not `&` ([`LIST_SEPARATOR`](https://docs.rs/crate/zenoh-protocol/1.10.2/source/src/core/parameters.rs#32)). The server rejects an unparsable cursor as `invalid_input` instead of ignoring it (scenario G).

## Measured facts

All runs were on a single host over loopback. Ranges cover the three checked-in runs; "3/3" means the same outcome in every run.

### Start and Get

| # | Case | Observed |
|---|---|---|
| A | Start with no queryable reachable | Zero replies; the reply channel closes in 0.4–0.5 ms, with no error value (3/3). A Get during a link outage also gets zero replies, in about 1 ms. |
| B | Malformed payload; unknown type | Both come back as `Ok` replies carrying typed `Rejected{invalid_input}` or `Rejected{unknown_operation_type}` (3/3). `reply_err` is no longer used. |
| C | Query timeout | `Err(ReplyError)` with payload `"Timeout"` and encoding `zenoh/string` after 300 ms (3/3). A reply the queryable sends after the caller's timeout still returns `Ok` on the server. A retry with the same `request_id` returned the same operation (`retried: true`). |
| D | Normalization | Reordered object keys plus a different `trace` gave the same operation. Changing `1` to `1.0` gave `request_id_conflict` (3/3). Keys are now sorted explicitly ([`canonical_json`](src/wire.rs)); unit tests cover nested reordering, an insertion-ordered map, arrays, escaping and `trace` exclusion. |
| E | 32 concurrent same-content retries | One operation id (3/3). |
| F | Two queryables on the Start key | With default target (`BestMatching`): one reply, silently, from one of them. With `QueryTarget::All` and `ConsolidationMode::None`: two replies (3/3). |
| G | Get and Observe edge cases | A terminal operation observed with `after=0` gives the terminal snapshot then `End{terminal}`. With the cursor already at the final `sequence`, it gives only `End{terminal}`; the first draft wrongly ended with `lagged`. A dropped report gives Get `Unavailable` and Observe `unavailable`. An unknown id gives `NotFound` for both (3/3). |

### Executor notifications (P5 / ST3), directly against Core

Event identity is (`token`, `event_seq`). Core hands out the execution token. The executor numbers its events from 1, contiguously; this number is separate from the Core-assigned snapshot `sequence`. Outcomes were identical in 3/3 runs:

| Notification | Core outcome | Snapshot |
|---|---|---|
| progress before ack | `InvalidTransition{from: pending}` | none |
| ack (`event_seq` 1) | `Applied` → `running` | seq 2 |
| same ack redelivered | `Duplicate{sequence: 2}` | none |
| `event_seq` 1 again with other content | `Stale` | none |
| `event_seq` 3 while 2 is expected | `OutOfOrder{expected: 2}` | none |
| progress (2), then the same redelivered | `Applied`, then `Duplicate{3}` | seq 3 |
| ack again as a new event (3) | `NoChange` | none |
| terminal `cancelled` (no Cancel in 0.1.0) | `InvalidTransition{from: running}` | none |
| terminal `succeeded` (4), then redelivered | `Applied`, then `Duplicate{4}` | seq 4 |
| progress after terminal | `InvalidTransition{from: succeeded}` | none |
| terminal `failed` after `succeeded` | `ConflictingTerminal{recorded: succeeded}` | none |
| terminal `succeeded` again as a new event | `NoChange` (enrichment is open, ST3) | none |
| unknown token | `UnknownToken` | none |
| 32 concurrent copies of one ack | 1 `Applied`, 31 `Duplicate` | one |
| `succeeded` and `failed` racing on one `event_seq` | One `Applied`; the other `Stale`. The winner is whichever takes the Core lock first. | one |
| lost handoff, then a late ack with the same token | `pending` → `unknown` (seq 2) → `running` (seq 3) | two |

Snapshot identity on the wire is (`operation.id`, `sequence`). Redelivery keeps both values; observers deduplicate by `sequence`.

### Observe attach races (200 iterations per pattern per run)

| # | Pattern | Missed terminal snapshot |
|---|---|---|
| R1 | subscribe only | 113–120 / 200 |
| R2 | Get, then subscribe | 2–6 / 200 |
| R3 | subscribe, then Get, dedup by `sequence` | 0 / 600 |
| R4 | streaming query, `ConsolidationMode::None`, Core attaches the watcher atomically | 0 / 600; p50 to terminal 0.73–0.75 ms |
| R5 | streaming query with default consolidation | 600 / 600 |

### Slow observers (2000 snapshots back-to-back, consumer ~1 ms per item)

| # | Path and handler | Observed (3/3 unless a range is given) |
|---|---|---|
| S1 | subscriber, `RingChannel(32)` | 35–67 received, 3–4 `sequence` gaps, terminal received. A Get on the same session answered in 0.6–0.7 ms. |
| S2 | subscriber, `FifoChannel(32)` | All 2000 received with no gaps, but a concurrent Get on the same caller session timed out after 2 s. |
| O1 | streaming query, default reply handler (FIFO 256), Core watcher cap 8 | First stream sent 10–32 frames, then `End{lagged}` at 0.22–0.27 s. The re-issue with `after=<last>` returned the terminal snapshot (seq 2002) with `gap_before`. A Get answered in 0.6–0.8 ms. |
| O2 | streaming query, `FifoChannel(32)`, watcher cap 8 | Same as O1: `lagged`, one re-issue, terminal. Get answered in 0.7–0.85 ms. |
| O3 | streaming query, `RingChannel(32)`, watcher cap 8 | 3 frames. The `End` frames were dropped on the caller side, so both streams closed without `End`. The terminal was still recognized from the snapshot `state`. |
| O4 | streaming query, default reply handler, watcher cap 4096 | All 2001 frames, no lag, terminal at about 4.8 s. A concurrent Get on the same session timed out after 2 s, as in S2. |

### Reconnection (TCP link cut and restored; Gecko publishes every 20 ms)

| # | Case | Observed (3/3) |
|---|---|---|
| N1 | plain subscriber, 1.5 s cut | The subscription was re-declared automatically. Delivery resumed 1.52–1.54 s after restore, with gap 28 → 163/164 visible through `sequence`. A Get after reconnect returned the latest snapshot. No gap notice is delivered on a plain subscriber. |
| N2 | three observers, 1.5 s cut, operation finishes after restore | subscribe-then-Get, subscribe-then-Get with liveliness repair, and the streaming query all received the terminal snapshot (seq 302) when it was published, each with one disclosed gap. The in-flight streaming query closed about 1 ms after the cut with no reply and no error. The 30 queries re-issued during the outage closed immediately with zero replies. The first query after reconnect resumed with `gap_before`. |
| N3 | same three observers, 2 s cut, operation finishes **during** the outage | **Plain subscribe-then-Get never learned the terminal** (last seq 20, no later sample, 10 s deadline). With liveliness repair: the Gecko token's `DELETE` arrived about 1 ms after the cut, its `PUT` about 1.0 s after restore, and the Get repair returned the terminal at once. The streaming query got the terminal 0.06–0.08 s after the liveliness `PUT`. |
| N4 | reconnect timing | The liveliness `PUT` came about 3.0 s after the cut in both cases: 1.5 s after restore for a 1.5 s cut, 1.0 s for a 2 s cut. |

### zenoh-ext Advanced pub/sub (`unstable`)

| # | Case | Observed (3/3) |
|---|---|---|
| X1 | late join | History (cache 16) delivered all 10 earlier samples. |
| X2 | 40 puts during an outage, cache 16 | After reconnect, 16 samples were recovered (35–50) and one `Miss{nb: 24}` event was reported. |
| W1 | serde_json internally tagged enum | A `u128` field fails to deserialize ("u128 is not supported"); the spike uses `u64` instead. |

## Interpretations and source readings (not measured)

- **R1.** One explanation is that `declare_subscriber().await` returns before the declaration has propagated to the publisher. The spike only shows the outcome.
- **S2/O4.** One explanation is that a full bounded FIFO handler blocks the session's receive path (head-of-line blocking), which delays other replies on the same session. Measured: the Get timeout in S2 and O4, and Get latency below 1 ms with ring handlers or with Core-side lagging.
- **N4.** The ~3 s reconnect fits the default connect retry schedule: 1 s initial period, ×2 backoff, 4 s maximum.
- **R5.** `Auto` consolidation resolves to `Latest` unless the selector has a time range ([session.rs#2718](https://docs.rs/crate/zenoh/1.10.2/source/src/api/session.rs#2718)), so frames on one key collapse to the last one.
- **Timeout encoding.** The session's own timeout uses `zenoh/string` ([session.rs#2760](https://docs.rs/crate/zenoh/1.10.2/source/src/api/session.rs#2760)). A router-side timeout uses the default encoding ([queries.rs#495](https://docs.rs/crate/zenoh/1.10.2/source/src/net/routing/dispatcher/queries.rs#495)), so encoding cannot reliably tell a Zenoh timeout from an application `reply_err` across topologies. The spike therefore sends no application `reply_err`.
- **Reply handler capacity.** The default reply and sample handler is a FIFO with capacity 256 ([session.rs#129](https://docs.rs/crate/zenoh/1.10.2/source/src/api/session.rs#129)).

## Not measured (assumptions and follow-ups)

- **Multi-hop (router) topologies.** R3 assumes that the subscription declaration and the later query travel the same ordered link. N3 assumes the observer sees the token of the Gecko it actually observes. Neither was checked through a router.
- **Half-open links without a TCP close.** Detection is bounded by the session lease (10 s by default). Publications in that window would be lost silently, and the liveliness `DELETE` would come late. Not exercised.
- **Gecko restart.** Out of scope for 0.1.0. The token and the in-memory records do not survive it.
- **Scale.** Large payloads, many concurrent observers, and memory per streaming query.
- **Congestion.** `CongestionControl::Drop` under real backpressure. On loopback, S2 shows buffering rather than drops.
- **Authenticated transport.** Access control and namespace ACLs are not configured; see the security column below.

## Approaches compared for Observe

| Approach | Race on attach | Reconnect | Slow observer | Security (security.md "Core bypass", integrity) | Cost and limits |
|---|---|---|---|---|---|
| Plain pub/sub only | Loses the final snapshot (R1) | Silent gap; terminal lost if published during the outage (N3) | Ring: gaps visible by `sequence`; FIFO: session HOL (S2) | Anyone allowed to `put` on `.../snapshot` can inject or forge snapshots, so the observer cannot tell Core output from a forgery without transport ACLs or signatures | Not viable alone |
| Subscribe → Get, dedup by `sequence` | 0 misses (R3) | Needs an extra signal; with liveliness repair the terminal is recovered (N3) | Ring handler and `sequence` gaps, repaired by Get (S1) | Same snapshot-key injection risk as pub/sub. Get answers come from a queryable, and a spoofed queryable can also answer (F); key ACLs are needed for both | Stable API. Ordering assumption across routers unverified. Two mechanisms plus liveliness per observer |
| Streaming query (Core-side atomic attach) | 0 misses (R4) | Stream closes on link loss; re-issue with `after` resumes with `gap_before`, with no extra signal (N2, N3) | Core bounds the watcher and ends with `lagged`; no session HOL at cap 8 (O1–O2). A large cap with a FIFO handler still blocks (O4). A ring handler loses `End` (O3) | No snapshot key to publish on, so only a queryable can answer. A spoofed queryable on `.../observe` remains possible (F); queryable ACLs and responder identity checks are needed | Matches model §3.1 literally. Must force `ConsolidationMode::None`. One in-flight query per observer, with a re-issue loop. The lease bounds stream length |
| zenoh-ext Advanced pub/sub | History covers late join (X1) | Recovery plus `Miss` events (X2) | Transport-level miss detection | Same publication injection risk as pub/sub | `unstable` feature. The cache keeps per-publisher samples rather than Operation `sequence`, which the model says must not be relied on. One publisher entity per operation |

## Mapping to the model (spike wire types against model §4–§5)

The spike types are a subset of the model so that GKO-61 does not inherit an accidental shape:

| Model | Spike | Gap |
|---|---|---|
| `operation{id, type, parent_id}` | `Snapshot.operation{id, type, parent_id?}` | `parent_id` is never set |
| `sequence`, `reported_at` | `sequence`, `reported_at_unix_ms` (u64) | Time format is still open (model §6) |
| `state` with six values | `pending/running/succeeded/failed/cancelled/unknown` | `cancelled` cannot be reached in 0.1.0 |
| `progress{stage, message, completed?, total?, unit?}` | same | none |
| `subject`, `context` | `target` only | `subject`, full `context`, `result`, `blockers`, `limitations`, `explanation`, `next_actions`, `evidence`, `children`, `details` are missing |
| Start: accepted with `operation.id` and observation address / rejected with reason | `Accepted{operation_id, report_key, observe_key, retried}` / `Rejected{code, message}` | The address form depends on Z1 |
| Get: latest snapshot or explicit unavailability | `Report` / `Unavailable` / `NotFound` | Z5 |
| Observe: current plus later snapshots, cursor, gap | `Snapshot{gap_before}` / `End{terminal, lagged, lease}` / `Unavailable` / `NotFound` / `Rejected` | none |
| Executor events (P5) | `ExecutorEvent{token, event_seq, event: ack, progress, terminal}` → `NotifyOutcome` | No handoff transport; the executor is in-process |

## Recommendation (proposal)

1. **Pin.** Pin `zenoh` `=1.10.2`. Do not depend on `unstable` for 0.1.0 (Z2).
2. **Start and Get.** Use Query/Queryable. Every Operation outcome is an `Ok` reply with a typed JSON body, including invalid input (Z3). The client treats every Zenoh `Err` (including `Timeout`), zero replies, or a closed stream as "no answer" and retries safely with the same `request_id`. Since no application error travels as `Err`, a deterministic rejection is never retried blindly.
3. **Observe (Z1).** Use the streaming query: Core-side atomic attach, `ConsolidationMode::None`, `after=<sequence>` resume, and Core-bounded watchers that end with `lagged`. Use a FIFO reply handler on the caller and a small Core watcher cap, so lag is decided at Core rather than by session HOL. Treat a terminal snapshot as the end even if `End` is lost. Subscribe-then-Get with liveliness repair is a measured fallback. Never use plain pub/sub alone, and never use large FIFO handlers on a shared session.
4. **Keys.** Namespace keys by Gecko id. Detect duplicate responders on Start (F) and treat more than one reply as a configuration error. Production namespace ACLs belong to the security work.
5. **Normalization.** Sort keys explicitly (done) and keep number spelling significant for 0.1.0 (Z4).
6. **Executor events (Z6).** Use (`token`, contiguous `event_seq`) with the outcome table above.

## Consequential choices for Farhat

These are registered as [Z1–Z6](../../spec/status.md#23-choices-raised-by-the-zenoh-spike-gko-60). Everything else here is implementation detail under accepted rules.

1. **Z1:** Observe mapping. Streaming query (recommended), subscribe-then-Get with liveliness repair, or both. This fixes the observation address returned by Start.
2. **Z2:** `unstable` zenoh-ext Advanced pub/sub before 1.0.0. Recommendation: not for 0.1.0.
3. **Z3:** Rejection channel. Typed `Ok` replies (recommended) or `reply_err`.
4. **Z4:** Number spelling in retry comparison. Spelling-sensitive (recommended for 0.1.0) or canonicalize numbers.
5. **Z5:** `not_found` versus `unauthorized` on Get and Observe. Indistinguishable (anti-enumeration) or distinct, which refines P3.
6. **Z6:** Executor event identity and arbitration outcomes for P5/ST3.
