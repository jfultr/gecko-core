# Build, test, and smoke commands for the Gecko Rust workspace.
# Run from the repository root on Linux or inside WSL (Ubuntu 24.04).
# rust-toolchain.toml pins the toolchain; rustup installs it on first use.

CARGO ?= cargo

.PHONY: all build test matrix fmt-check clippy smoke demo lab-server lab-client docs-check articles-check hooks check

all: check

# The plain build leaves out operations-lab: it enables the test-support
# features, which would otherwise be unified into the production crates.
build:
	$(CARGO) build --locked --workspace --exclude operations-lab
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

# Interactive lab stand (GKO-68): run each in its own terminal.
# Options go in LAB_ARGS, for example: make lab-client LAB_ARGS=--via-proxy
LAB_ARGS ?=

lab-server:
	$(CARGO) run --locked -q -p operations-lab -- server $(LAB_ARGS)

lab-client:
	$(CARGO) run --locked -q -p operations-lab -- client $(LAB_ARGS)

docs-check:
	python3 docs/check_links.py

articles-check:
	python3 tools/articles/articles.py check

hooks:
	git config core.hooksPath .githooks

check: fmt-check build clippy test smoke demo docs-check articles-check
