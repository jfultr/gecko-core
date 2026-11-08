# Build, test, and smoke commands for the Gecko Rust workspace.
# Run from the repository root on Linux or inside WSL (Ubuntu 24.04).
# rust-toolchain.toml pins the toolchain; rustup installs it on first use.

CARGO ?= cargo

.PHONY: all build test matrix fmt-check clippy smoke demo docs-check articles-check hooks check

all: check

build:
	$(CARGO) build --locked --workspace
	$(CARGO) build --locked --workspace --all-targets --all-features

test:
	$(CARGO) test --locked --workspace --all-features

# The P1-P6 checks only (GKO-67): Core directly and through Zenoh.
matrix:
	$(CARGO) test --locked -p operations --all-features --test acceptance --test contracts --test dispatch --test lifecycle --test observe
	$(CARGO) test --locked -p operations-zenoh --features test-support --test matrix --test scenario

fmt-check:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --locked --workspace --all-targets --all-features -- -D warnings

smoke:
	$(CARGO) run --locked -p operations --features test-support --example smoke

demo:
	$(CARGO) run --locked -p operations-zenoh --features test-support --example demo

docs-check:
	python3 docs/check_links.py

articles-check:
	python3 tools/articles/articles.py check

hooks:
	git config core.hooksPath .githooks

check: fmt-check build clippy test smoke demo docs-check articles-check
