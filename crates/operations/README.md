# operations

Rust crate of the Gecko Operation module. The module contract and decisions are owned by the [Operation documentation center](../../docs-modules/operation/README.md) and its [model](../../docs-modules/operation/spec/model.md). This README covers only code, commands, and verified behavior.

## Status

Version `0.1.0-dev` (matches `docs-modules/operation/VERSION`).

Implemented (GKO-62, OP-A4):

- transport-neutral Start, Get, and Observe inputs and typed outcomes (accepted, rejected, unavailable, invalid input, error) and the full report types (`api`, `report`);
- UUIDv4 `RequestId` and `OperationId`, and a `CallerContext` that adapters build and that cannot be deserialized from a payload (`identity`);
- request validation against a `Catalog`, normalized `RequestFingerprint`, subject-scoped `RetryKey`, and `RetryClass` (`request`, `value`);
- an authorization hook: `Guard` calls the `Authorizer` for every command, records the decision, and returns an `Authorized` proof that `auth::hand_off` consumes and accepts only for Start; `Refusal` maps to typed responses without exposing the authorizer's reason (the adapter must authorize before validating); it fails closed on denial, unknown decision, or log failure, and the default policy is `DenyAll` (`auth`);
- a provisional `Executor` trait, `ExecutionToken`, and `Handoff`;
- behind the `test-support` feature, `TestExecutor`, fixtures, the test catalog (`test.echo`), and `PermitAllForTests`, the only permissive policy.

Not implemented: size limits for target, parameters, and trace (to be set with the transport adapter); atomic acceptance and retry index (GKO-63), state machine, snapshots, storage, Zenoh adapter. The in-memory decision log is not durable audit.

Dependencies: `serde` and `uuid` (`v4`); `serde_json` for tests only. Number comparison is spelling-sensitive (`1` differs from `1.0`), per decision Z4.

Core must stay independent of Zenoh and domain entities. The layout is provisional, not a contract.

## Layout

```text
crates/operations/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── api.rs
│   ├── auth.rs
│   ├── executor.rs
│   ├── identity.rs
│   ├── report.rs
│   ├── request.rs
│   ├── value.rs
│   └── testing.rs
├── tests/
│   ├── contracts.rs
│   └── harness.rs
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
