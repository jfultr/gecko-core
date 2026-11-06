# zenoh-spike

Throwaway measurement harness for the GKO-60 Zenoh `1.10.2` spike. It is evidence for the
[report](../../docs-modules/operation/research/zenoh-spike/REPORT.md), not product code, and it is
deliberately outside the root workspace (own `[workspace]`), so `make check` does not build it.

```sh
cd spikes/zenoh-spike
cargo test --release            # canonicalization unit tests
cargo run --release -- all      # every scenario; see the report for single scenarios
```
