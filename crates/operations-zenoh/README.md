# operations-zenoh

Minimal Zenoh adapter, demo client, and scripted scenario for the Gecko Operation module (GKO-66, OP-A8). It maps the transport-neutral Core in [`crates/operations`](../operations/README.md) to Zenoh. The module contract and decisions are owned by the [Operation documentation center](../../docs-modules/operation/README.md); this README covers only code, commands, and verified behavior.

Core does not depend on this crate or on Zenoh. The test executor stays in-process with Core.

## Status

Version `0.1.0-dev`. Zenoh SDK: `zenoh = "=1.10.2"` (the version measured in the [GKO-60 spike](../../docs-modules/operation/research/zenoh-spike/REPORT.md)), default features, no `unstable` feature (decision Z2).

## Mapping

Key prefix `gecko/g1/op` (provisional, configurable through `Keys`).

| Action | Zenoh | Reply |
| --- | --- | --- |
| Start | query on `gecko/g1/op/start`, payload is a JSON `StartRequest` | one typed `StartResponse` |
| Get | query on `gecko/g1/op/<operation id>/report` | one typed `GetResponse` |
| Observe | streaming query on `gecko/g1/op/<operation id>/observe`, optional `?after=<sequence>`, consolidation `None` | frames `response`, then `snapshot` frames, then `end` |

The observation address Start returns is the Observe key (give `Keys::address` to `Acceptor::with_observation_address`). Every Operation outcome, including invalid input, is a typed `Ok` reply (Z3). A Zenoh error reply, zero replies, or a stream that closes without an `end` frame is "no answer" (`NoAnswer`). Retry is safe: Start with the same `request_id` creates at most one execution, and Observe resumes with the last `sequence` held. Stream ends are `terminal`, `lagged`, `lease`, `unavailable`, or `closed`; all but the first two and `unavailable` mean "resume".

`Server` runs Core on a peer session and hands every created execution to the executor given to it once. `Client` offers `start`, `start_retrying` (same `request_id`), `get`, `observe`, and `follow`, which follows an operation to its terminal state and reconnects with the last `sequence` after a lost link, `lagged`, or `lease`.

## Run

From the repository root, on Linux or in WSL (see the [development environment](../../docs/development-environment.md)):

```sh
make demo     # the scripted scenario: cargo run --locked -p operations-zenoh --features test-support --example demo
make check    # includes the demo and the scenario test
```

The demo takes about 2 seconds. It starts everything in one process: a Gecko session listening on a free `127.0.0.1` TCP port, a client connected directly, and a client connected through a TCP proxy whose link it cuts. There is nothing to start or clean up separately: ports are free ports chosen at run time, and sessions, the proxy, and threads end when the demo exits (it exits non-zero on the first failed check). A stuck run can be stopped with Ctrl-C; no files or daemons are left.

Endpoints in a real use: set `ServerConfig::listen` (for example `tcp/127.0.0.1:7447`) and `ClientConfig::connect` to it. Multicast and gossip scouting are disabled, so a client must be given the endpoint.

## What the scenario checks (and what it does not)

Verified by `make demo` and `tests/scenario.rs` against Zenoh `1.10.2` on one host over loopback:

1. Start accepted; the observation address is the Observe key; Get over Zenoh equals the same read in-process through Core.
2. Rejections and malformed input are typed replies: unknown operation type, missing parameter, unparsable payload, unknown operation id, a key without an operation id.
3. A Start whose reply is lost (reply sent after the caller's timeout) gives "no answer"; the retry with the same `request_id` returns the same operation, exactly one execution exists, the executor received the work once, and the same `request_id` with other content is `identifier_conflict`.
4. Progress and a result reach a following client; with the link cut and restored while the executor works, the client reconnects, sequences strictly increase, and every number between the first and last is either seen or disclosed as a gap.
5. Resume by `after_sequence` on the wire: `after=2` returns the latest snapshot with the skipped numbers as a gap, `after=<latest>` redelivers nothing, a cursor from the future is invalid input.
6. A late observer of completed work gets the final snapshot, the stream ends as `terminal`, and nothing is handed to the executor.

Deliberately breaking the server to ignore `after` fails step 5, so the scenario detects that defect. Not measured or not done: more than one host, multi-hop routing, half-open links, restart of the Gecko, slow observers through Zenoh (covered at Core level), and load.

## Restricted deployment limits

- **Identity is configuration, not authentication.** `ServerConfig::caller` is applied to every request. Any participant that can reach the endpoint acts as that subject, so use this only on a trusted local host. Production identity and authorization belong to `0.5.0` and the [security gate](../../docs-modules/operation/spec/security.md).
- No TLS, no access control on keys, no limits on request size, no protection against request floods (one thread per Start; a bounded number of Observe streams, default 64).
- Everything is in memory. A restart of the demo loses all executions.
- The `start_reply_delay` hook and the proxy exist only behind the `test-support` feature.
- Observe streams end with `lease` after 20 seconds by default (1 second in the demo) so abandoned streams do not live forever; clients resume.
- The key prefix and frame shapes are provisional until the transport contract is stabilized in `0.4.0`.

## Layout

```text
crates/operations-zenoh/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs       mapping overview and exports
│   ├── keys.rs      key expressions
│   ├── wire.rs      Observe frames and end reasons
│   ├── server.rs    Server: Start, Get, streaming Observe over one Acceptor
│   ├── client.rs    Client, NoAnswer, ObserveStream, follow
│   ├── proxy.rs     cuttable TCP proxy (test-support)
│   └── demo.rs      the scripted scenario (test-support)
├── tests/scenario.rs
└── examples/demo.rs
```
