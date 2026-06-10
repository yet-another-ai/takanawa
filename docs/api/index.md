# API Docs

Takanawa exposes the same download core through Rust crates and native platform
packages. Published Rust APIs are documented on docs.rs, while platform SDKs are
distributed through Maven Central and SwiftPM artifacts.

## Rust Crates

- [`takanawa-core`](https://docs.rs/takanawa-core): chunk planning, `.part`
  metadata, file recovery, and hash verification.
- [`takanawa-http`](https://docs.rs/takanawa-http): Tokio and reqwest based HTTP
  range download engine.
- [`takanawa-ffi`](https://docs.rs/takanawa-ffi): C ABI wrapper for native
  library consumers.
- [`takanawa-cli`](https://docs.rs/takanawa-cli): dogfood command-line client.

## Native Interfaces

The public C header is generated from `takanawa-ffi`:

```sh
mise run header
```

Android consumers use the Kotlin-first SDK:

```kotlin
dependencies {
    implementation("ai.yetanother:takanawa-android:0.2.0")
}
```

Apple consumers use the SwiftPM package and prebuilt `Takanawa.xcframework`.

## Local Rustdoc

Generate local Rust API documentation with Cargo:

```sh
cargo doc --workspace --all-features --no-deps
```

The generated docs are written under `target/doc`.
