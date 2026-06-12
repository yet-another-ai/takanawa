# Rust

Use the Rust crates when the application is itself written in Rust or when you
want to compose the download engine directly.

## Install

Add the crates you need to `Cargo.toml`:

```toml
[dependencies]
takanawa-core = "{{ takanawaVersion }}"
takanawa-http = "{{ takanawaVersion }}"
```

For workspace development, depend on local paths instead:

```toml
[dependencies]
takanawa-core = { path = "../takanawa/crates/takanawa-core" }
takanawa-http = { path = "../takanawa/crates/takanawa-http" }
```

## Local Development

Run the Rust checks from the repository root:

```sh
cargo test --workspace
```

Build the command-line dogfood client:

```sh
cargo build -p takanawa-cli
```

Published Rust API reference is available on docs.rs for
[`takanawa-core`](https://docs.rs/takanawa-core),
[`takanawa-http`](https://docs.rs/takanawa-http), and
[`takanawa-ffi`](https://docs.rs/takanawa-ffi).
