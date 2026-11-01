# ROB-60 spike: Zenoh mapping, wire types and observation races (draft)

Status: research result for ROB-60, November 2, 2026. Measured on one host; nothing here changes accepted rules. Code: this directory (`docs-modules/operation/research/zenoh-spike/`).

## Pinned setup

- `zenoh = "=1.10.2"`, `zenoh-ext = "=1.10.2"` with feature `unstable` (released 2026-10-28; 1.10.1 was 2026-09-07). MSRV 1.75; built with rustc 1.99.1 on Ubuntu-24.04 / WSL2 (kernel 6.6.87.2), x86_64.
- tokio 1 (multi-thread, 4 workers), serde_json, uuid v4.
- Topology: one process. "Gecko" session = `peer`, listening on `tcp/127.0.0.1:<free>`; caller session = `client` connecting to it, directly or via an in-process TCP proxy that can be cut. Multicast and gossip scouting disabled.
- Reproduce from this directory: `cargo run --release -- all` (or `start`, `observe-race`, `slow`, `reconnect`, `advanced`). Raw output of the full run: [results-run1.txt](results-run1.txt). The crate has its own `[workspace]` and is not part of `docs-modules/operation/code`.

Keys used: `gecko/g1/op/start` (queryable), `gecko/g1/op/<op_id>/report` (queryable, Get), `gecko/g1/op/<op_id>/observe?after=<seq>` (queryable, streaming Observe), `gecko/g1/op/<op_id>/snapshot` (pub/sub).

## Measured facts (single host, loopback; repeated 2–3 times, stable)

| # | Case | Observed |
|---|---|---|
| A | Start with no queryable reachable | Zero replies, channel closes in ~0.4 ms. No error value. Same after link loss (Get during outage: zero replies in <1 ms). |
| B | Malformed payload → `reply_err` | Caller gets `Err(ReplyError)` with app payload. Semantic rejection sent as `Ok` reply with typed `Rejected{code}` decodes cleanly. |
| C | Query timeout | Surfaces as `Err(ReplyError)` with payload `"Timeout"`, encoding `zenoh/string` — same type as app `reply_err`. A reply sent by the queryable after the caller's timeout returns `Ok` on the server: the server cannot detect the lost response. Retry with same `request_id` returned the same operation (`retried: true`). |
| D | Normalization | Reordered object keys + different `trace` → same operation. `1` vs `1.0` → `request_id_conflict` (serde_json keeps number spelling). |
| E | 32 concurrent same-content retries | 1 operation id (dedup under Core lock). |
| F | Two queryables on the same Start key | Default target (BestMatching): 1 reply from one arbitrary Gecko, silently. `QueryTarget::All` + `ConsolidationMode::None`: 2 replies. |
| R1 | Attach vs final publication, `subscribe-only` | 99–110 / 200 terminal snapshots missed although `declare_subscriber().await` had returned: declaration is not yet effective at the publisher. |
| R2 | `get-then-subscribe` | 2–3 / 200 missed (window between Get and declaration). |
| R3 | `subscribe-then-get` (dedup by `sequence`) | 0 / 600 missed over three runs. |
| R4 | Streaming query Observe (`ConsolidationMode::None`, Core attaches watcher atomically) | 0 / 600 missed; p50 ≈ 0.6 ms to terminal; replies stream incrementally before query finalization. |
| R5 | Streaming query with default consolidation | 200 / 200 missed: `Auto` resolves to `Latest` (session.rs:2718), so frames on one key collapse to the last one (`End`). |
| S1 | Slow consumer, `RingChannel(32)` | 34–36 of 2000 received, 2–3 gaps detected by `sequence`, last (terminal) seq received; other traffic on session unaffected (Get 0.5 ms). |
| S2 | Slow consumer, `FifoChannel(32)` | All 2000 received, no gaps, but a concurrent Get on the same caller session timed out (2 s): the bounded FIFO callback blocks the session's receive path (head-of-line blocking). Publisher `put` was not slowed (~2–3 ms for 2000). |
| N1 | Link cut 1.5 s (TCP closed), client mode | Subscription re-declared automatically after reconnect; reconnect took ~1.5 s after restore (defaults: retry 1 s init, ×2, max 4 s). Gap 29→171 visible via `sequence`; Get after reconnect returned latest (301). No transport notice of the gap on a plain subscriber. |
| X1 | zenoh-ext Advanced pub/sub, late join | History (cache 16) delivered all 10 earlier samples. |
| X2 | Advanced, 40 puts during outage, cache 16 | After reconnect: 16 recovered (35–50) and one `Miss{nb: 24}` event. |
| W1 | serde_json + internally tagged enum | `u128` field fails to deserialize ("u128 is not supported"); use `u64`. |

## Not measured (assumptions / follow-ups)

- Multi-hop (router) topologies: R3 relies on the subscription declaration and the later query travelling the same ordered link. Not verified through a router.
- Half-open links without TCP close: detection is bounded by session lease (default 10 s); publications in that window would be lost silently. Not exercised.
- Gecko restart (out of scope for 0.1.0; AdvancedPublisher sequence/cache would reset with a new entity id).
- Large payloads, many concurrent observers, memory per streaming query.
- Default `CongestionControl::Drop` under real backpressure; S2 shows buffering, not drops, on loopback.

## Approaches compared for Observe

| Approach | Race on attach | Gaps / slow observer | Cost / limits |
|---|---|---|---|
| Plain pub/sub only | Loses final snapshot (R1) | Silent loss; seq shows gaps only if later samples arrive | Not viable alone |
| Subscribe → Get, dedup by `sequence` | 0 misses (R3) | Ring handler + `sequence` gaps + Get repair (S1, N1) | Stable API; ordering assumption across routers unverified |
| Streaming query (Core-side atomic attach) | 0 misses (R4) | Core decides lag (drop watcher → `End{lagged}`), client resumes with `after` | Matches model §3.1 literally; must force `ConsolidationMode::None`; one in-flight query per observer; query timeout = lease, needs re-issue |
| zenoh-ext Advanced pub/sub | History closes late-join; recovery + Miss events | Per-publisher transport seq, not Operation `sequence` | `unstable` feature; cache in transport layer (model says must not be relied on); per-op publisher entity |

## Recommendation (proposal)

1. Pin `zenoh`/`zenoh-ext` `=1.10.2` (or 1.10.1 if a two-week maturity rule is wanted; API identical for the spike). Do not depend on `unstable` for 0.1.0.
2. Start and Get: Query/Queryable; typed outcomes always as `Ok` replies with JSON body (`accepted` / `rejected{code}`; Get `report` / `unavailable` / `not_found`). Treat zero replies, `Timeout` and any `ReplyError` as "no answer → safe retry with same `request_id`". Reserve `reply_err` for protocol-level errors with an explicit app encoding so they are not confused with Zenoh's `Timeout`.
3. Observe: subscribe-then-Get with `RingChannel` and `sequence` dedup/gap detection as baseline; streaming query as the candidate that proves the model's "atomic attach at Core" end to end. Never use FIFO handlers for observation on a shared session.
4. Namespace keys by Gecko id and query Start with an explicit single-target expectation; detect duplicate responders (F).
5. Canonical content: serde_json `Value` with sorted keys; number spelling significant — document it or normalize numbers.

## Consequential choices for Farhat

1. Observe mapping for 0.1.0: subscribe-then-Get (pub/sub) vs streaming query vs both. Affects the public "observation address" in StartResponse.
2. Whether `unstable` zenoh-ext Advanced pub/sub is acceptable at all before 1.0.0 (recommendation: no for 0.1.0).
3. Rejection channel: typed `Ok` reply (recommended) vs `reply_err`.
4. Number normalization in retry comparison: keep spelling-sensitive (1 ≠ 1.0) or canonicalize numbers.
5. `not_found` vs `unauthorized` on Get/Observe: indistinguishable (anti-enumeration) or distinct as P3 currently says ("unauthorized access and invalid input are distinct errors").
