# operations

Rust crate of the Gecko Operation module. The module contract and decisions are owned by the [Operation documentation center](../../docs-modules/operation/README.md) and its [model](../../docs-modules/operation/spec/model.md). This README covers only code, commands, and verified behavior.

## Status

Version `0.1.0-dev` (matches `docs-modules/operation/VERSION`).

Implemented (GKO-62, OP-A4):

- transport-neutral Start, Get, and Observe inputs and typed outcomes (accepted, rejected, unavailable, invalid input, error) and the full report types (`api`, `report`);
- UUIDv4 `RequestId` and `OperationId`, and a `CallerContext` that adapters build and that cannot be deserialized from a payload (`identity`);
- request validation against a `Catalog`, normalized `RequestFingerprint`, subject-scoped `RetryKey`, and `RetryClass` (`request`, `value`);
- an authorization hook: `Guard` calls the `Authorizer` for every command, records the decision, and returns an `Authorized` proof that `auth::hand_off` consumes and accepts only for Start; `Refusal` maps to typed responses without exposing the authorizer's reason (the adapter must authorize before validating); it fails closed on denial, unknown decision, or log failure, and the default policy is `DenyAll` (`auth`);
- atomic acceptance (GKO-63, OP-A5): `Acceptor::start` authorizes and records the decision, validates, then commits the retry mapping, the execution record, and the initial `pending` snapshot (sequence 1) in one `Storage::accept` call; same-content retries return the earlier operation, changed content is an `identifier_conflict` that leaves the original unchanged, only a request that created the execution gets a `Dispatch`, and the execution stores the index of the decision record that permitted it (`acceptance`). The decision is recorded before the commit, so a failed commit leaves a permit record without an operation; the guard is one mutex, a known limit for slow authorizers;
- executor handoff and Core snapshots (GKO-64, OP-A6): Core issues a random UUIDv4-text `ExecutionToken` at acceptance. `Acceptor::dispatch` consumes the `Dispatch`, hands off once, and records the result: acknowledged is `running`, ambiguous is `unknown` with no redispatch, a refusal proven to precede side effects is `failed`. `Acceptor::notify` arbitrates identified executor events (`token`, `event_seq`) under Z6: duplicates and repeated acks add no snapshot, stale, out-of-order, invalid and conflicting events are typed `NotifyRejection`s that change nothing, terminal outcomes are immutable, partial `result` data is kept, and every new snapshot is a full copy with `sequence` + 1. Arbitration runs under the storage lock per record via `Storage::update` (`lifecycle`, `acceptance`). Stale and conflicting events are typed rejections, not panics or protocol aborts; whether to call them protocol errors stays open in decision Z6;
- authoritative Get and safe Observe (GKO-65, OP-A7): `Acceptor::get` returns the latest retained full snapshot of an execution the caller owns. `Acceptor::observe` reads the latest snapshot and registers the stream under the lock that publishes every snapshot, so a final publication cannot be lost between the two. `after_sequence` resumes: a snapshot the observer holds is not redelivered, skipped numbers are disclosed as a `Gap`, and the latest snapshot is always delivered. Each observer has a bounded buffer; a full one is ended as `Lagged` and Core never waits. A terminal snapshot ends streams with `Terminal`, dropping an `Observation` only removes the observer, and reading never starts, resumes or cancels work. `Follower` is the observer-side rule: the greatest `sequence` wins, equal or lower ones never replace newer state, and conflicting content under one number is refused. `Acceptor::release_report` stands in for retention (`observe`);
- an `Executor` trait and `Handoff` (the trait is still synchronous and provisional);
- behind the `test-support` feature, `TestExecutor`, fixtures, the test catalog (`test.echo`), and `PermitAllForTests`, the only permissive policy.

In-memory only: `MemoryStorage` and the decision log lose everything on restart; there is no restart durability in `0.1.0` (durable accounting is `0.3.0`). The observation address `operations/<id>/observe` is provisional until the Observe mapping is chosen.

Known limits of GKO-64: `Cancel` and `cancelled` do not exist (`cancelled` events are an invalid transition); terminal-detail enrichment is not decided, so a repeated terminal outcome changes nothing; an executor event is trusted by token possession alone (no executor authentication); if recording a handoff result fails after the executor received the work, the execution stays `pending` and the caller must record `Handoff::Ambiguous` (`Acceptor::record_handoff`) instead of dispatching again; simultaneous different outcomes on one `event_seq` resolve by lock order, so the winner is not deterministic for the executor. Snapshots are not delivered anywhere yet.

Known limits of GKO-65: one lock serializes every publication and attach across all executions; only the starting subject may read (choice O1, awaiting confirmation with O2–O4 in status §2.5); a released report cannot come back; there is no full history, only the latest snapshot plus gap disclosure; Observe is an in-process stream, not yet a Zenoh mapping.

Not implemented: size limits for target, parameters, and trace (to be set with the transport adapter); durable storage, Zenoh adapter. The in-memory decision log is not durable audit.

Dependencies: `serde` and `uuid` (`v4`); `serde_json` for tests only. Number comparison is spelling-sensitive (`1` differs from `1.0`), per decision Z4.

Core must stay independent of Zenoh and domain entities. The layout is provisional, not a contract.

## Layout

```text
crates/operations/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── acceptance.rs
│   ├── api.rs
│   ├── auth.rs
│   ├── executor.rs
│   ├── identity.rs
│   ├── lifecycle.rs
│   ├── observe.rs
│   ├── report.rs
│   ├── request.rs
│   ├── value.rs
│   └── testing.rs
├── tests/
│   ├── acceptance.rs
│   ├── contracts.rs
│   ├── harness.rs
│   ├── lifecycle.rs
│   └── observe.rs
└── examples/
    └── smoke.rs
```

The Cargo workspace is at the repository root (`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `Makefile`). Rust crates live under `crates/`.

## Commands

Prerequisites: [rustup](https://rustup.rs). If `cargo` is not on PATH, run `export PATH="$HOME/.cargo/bin:$PATH"` first. `rust-toolchain.toml` pins the toolchain; rustup installs it on first use.

From the repository root:

```sh
make check        # fmt-check, build (default and all features), clippy -D warnings, tests, smoke example, docs link check
make build        # cargo build --locked --workspace, then with --all-targets --all-features
make test         # cargo test --locked --workspace --all-features
make fmt-check    # cargo fmt --all -- --check
make clippy       # cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
make smoke        # cargo run --locked -p operations --features test-support --example smoke
make docs-check   # python3 docs/check_links.py
```

All Cargo commands use `--locked`. Test support requires `--all-features` or `--features test-support`.

## Toolchain and environment

| Item | Observed value |
| --- | --- |
| `rustc -V` | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| `cargo -V` | `cargo 1.99.1 (5f94df478 2026-08-27)` |
| `cargo clippy -V` | `clippy 0.1.99 (58cf8f9c35 2026-10-13)` |
| `cargo fmt --version` | `rustfmt 1.10.0-stable (58cf8f9c35 2026-10-13)` |
| OS (`/etc/os-release` PRETTY_NAME) | `Ubuntu 24.04.1 LTS` |
| `uname -r` | `6.6.87.2-microsoft-standard-WSL2` |
