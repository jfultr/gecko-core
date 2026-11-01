# operations

Rust crate of the Gecko Operation module. The module contract and decisions are owned by the [Operation documentation center](../../docs-modules/operation/README.md) and its [model](../../docs-modules/operation/spec/model.md). This README covers only code, commands, and verified behavior.

## Status

Version `0.1.0-dev` (matches `docs-modules/operation/VERSION`).

This crate contains only the harness skeleton:

- a provisional `Executor` trait, `ExecutionToken`, and `Handoff`;
- behind the `test-support` feature, a controllable `TestExecutor` and fixtures.

Not implemented: Start, Get, Observe, state machine, snapshots, storage, Zenoh adapter.

The crate has no dependencies. Core must stay independent of Zenoh and domain entities. The layout is provisional, not a contract.

## Layout

```text
crates/operations/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── executor.rs
│   └── testing.rs
├── tests/
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
