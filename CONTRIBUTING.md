# Testing

Install cargo-nextest (`cargo install cargo-nextest --locked`), then run the same test command as CI:

```sh
cargo nextest run --profile ci --workspace --all-targets --features svm-rs/blocking
```

Use nextest locally as well as in CI. Tests mutate an SVM cache that is isolated per test process;
the standard `cargo test` runner shares that cache between tests. No global test serialization is
needed. Concurrency regression tests coordinate their own workers and run without test retries.
The `blocking` feature ensures both synchronous and asynchronous installation paths are tested.

Additional checks:

```sh
cargo nextest run --workspace --all-targets --all-features
cargo hack check --feature-powerset --depth 2
cargo +nightly fmt --all --check
RUSTFLAGS=-Dwarnings cargo +nightly clippy --workspace --all-targets --all-features
```
