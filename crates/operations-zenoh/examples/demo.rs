//! Runs the GKO-66 scenario: `make demo` (or `cargo run -p operations-zenoh
//! --features test-support --example demo`). Exits non-zero on the first failed check.

fn main() {
    if let Err(message) = operations_zenoh::demo::run(&mut std::io::stdout()) {
        eprintln!("demo failed: {message}");
        std::process::exit(1);
    }
}
