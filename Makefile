# Build, test, and smoke commands for the Gecko Rust workspace.
# Run from the repository root on Linux or inside WSL (Ubuntu 24.04).
# rust-toolchain.toml pins the toolchain; rustup installs it on first use.

CARGO ?= cargo

.PHONY: all build test fmt-check clippy smoke docs-check articles-check hooks check

all: check

build:
	$(CARGO) build --locked --workspace
	$(CARGO) build --locked --workspace --all-targets --all-features

test:
	$(CARGO) test --locked --workspace --all-features

fmt-check:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --locked --workspace --all-targets --all-features -- -D warnings

smoke:
	$(CARGO) run --locked -p operations --features test-support --example smoke

docs-check:
	python3 docs/check_links.py

articles-check:
	python3 tools/articles/articles.py check

hooks:
	git config core.hooksPath .githooks

check: fmt-check build clippy test smoke docs-check articles-check
