# testutil

This repo contains docker modules for [`testcontainers-rs`](https://github.com/testcontainers/testcontainers-rs): GGX, BTC, Vault, Cosmos, Hermes.

And end-to-end (e2e) tests under [`/tests`](./tests)

To run all tests you need:
1. Rust
2. Docker to be up and running

Then:
```bash
export RUST_LOG=info
cargo test
```
