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
    implementation("ai.yetanother:takanawa-android:0.3.1")
}

val download = TakanawaDownload.create(config)
download.setProgressCallback { snapshot ->
    println("${snapshot.phase}: ${snapshot.downloadedBytes}/${snapshot.contentLen}")
}
download.start()
```

Apple consumers use the SwiftPM package and prebuilt `Takanawa.xcframework`.

```swift
let download = try TakanawaDownload.create(config)
try download.setProgressCallback { snapshot in
  print("\(snapshot.phase): \(snapshot.downloadedBytes)/\(snapshot.contentLen)")
}
try download.start()
```

C ABI consumers can register a nullable `TknwProgressCallback` with
`tknw_download_set_progress_callback`. Passing `NULL` clears the callback.
C and C++ projects can link the native library through the CMake target
`Takanawa::takanawa` or through the local vcpkg overlay port.

## Local Rustdoc

Generate local Rust API documentation with Cargo:

```sh
cargo doc --workspace --all-features --no-deps
```

The generated docs are written under `target/doc`.
