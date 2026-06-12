# API Docs

Takanawa exposes the same download core through Rust crates, npm packages, and
native platform packages. Published Rust APIs are documented on docs.rs, while
platform SDKs are distributed through npm, Maven Central, SwiftPM artifacts, and
NuGet.

## Rust Crates

- [`takanawa-core`](https://docs.rs/takanawa-core): chunk planning, `.part`
  metadata, file recovery, and hash verification.
- [`takanawa-http`](https://docs.rs/takanawa-http): Tokio and reqwest based HTTP
  range download engine.
- [`takanawa-ffi`](https://docs.rs/takanawa-ffi): C ABI wrapper for native
  library consumers.
- [`takanawa-cli`](https://docs.rs/takanawa-cli): dogfood command-line client.

## JavaScript Packages

- `takanawa-node`: Node.js and Electron bindings built on Node-API.
- `takanawa-capacitor`: Capacitor v8 plugin for Android and iOS apps.
- `takanawa-tauri`: Tauri v2 frontend package for desktop apps, paired with the
  `tauri-plugin-takanawa` Rust crate.

`takanawa-node`, `takanawa-capacitor`, and `takanawa-tauri` are built from the same internal
TypeScript facade, so their public option names, listener handles, hash forms,
phase strings, and snapshot fields stay aligned. That shared facade is bundled
into each target package and is not published as a standalone npm package.

## Hash Verification

Download configurations can request final file verification with SHA-1,
SHA-256, SHA-512, MD5, or CRC32. Rust callers pass a `HashConfig` variant, the
C ABI uses `hash_kind` with the expected digest bytes, Android uses
`HashKind`/`expectedHash`, Swift uses `HashKind`/`expectedHash`, and the
JavaScript packages use `hash: { kind, expected }` or `hash: "kind:<hex>"`.
Existing Android, Swift, and JavaScript `expectedSha256`/`sha256` shortcuts
continue to select SHA-256.

Digest byte lengths are SHA-1 = 20, SHA-256 = 32, SHA-512 = 64, MD5 = 16, and
CRC32 = 4 bytes. CRC32 is represented in standard big-endian hexadecimal order
(for example, `352441c2` becomes bytes `35 24 41 c2`).

## Native Interfaces

The public C header is generated from `takanawa-ffi`:

```sh
mise run header
```

Android consumers use the Kotlin-first SDK:

```kotlin
dependencies {
    implementation("ai.yetanother:takanawa-android:{{ takanawaVersion }}")
}

val download = TakanawaDownload.create(config)
download.setProgressCallback { snapshot ->
    println("${snapshot.phase}: ${snapshot.downloadedBytes}/${snapshot.contentLen}")
}
download.setSpeedCallback { snapshot ->
    println("${snapshot.bytesPerSecond} B/s")
}
download.start()
```

Apple consumers use the SwiftPM package and prebuilt `Takanawa.xcframework`.

```swift
let download = try TakanawaDownload.create(config)
try download.setProgressCallback { snapshot in
  print("\(snapshot.phase): \(snapshot.downloadedBytes)/\(snapshot.contentLen)")
}
try download.setSpeedCallback { snapshot in
  print("\(snapshot.bytesPerSecond) B/s")
}
try download.start()
```

C# consumers use the `YetAnotherAI.Takanawa` NuGet package.

```csharp
using YetAnotherAI.Takanawa;

using var download = TakanawaDownload.Create(config);
download.SetProgressCallback(snapshot =>
    Console.WriteLine($"{snapshot.Phase}: {snapshot.DownloadedBytes}/{snapshot.ContentLen}"));
download.SetSpeedCallback(snapshot =>
    Console.WriteLine($"{snapshot.BytesPerSecond} B/s"));
download.Start();
```

C ABI consumers can register a nullable `TknwProgressCallback` with
`tknw_download_set_progress_callback` and a nullable `TknwSpeedCallback` with
`tknw_download_set_speed_callback`. Passing `NULL` clears the callback.
C and C++ projects can link the native library through the CMake target
`Takanawa::takanawa` or through the local vcpkg overlay port.

## Local Rustdoc

Generate local Rust API documentation with Cargo:

```sh
cargo doc --workspace --all-features --no-deps
```

The generated docs are written under `target/doc`.
