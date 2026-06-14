# Takanawa

Takanawa is a Rust range-download library designed to ship as a C ABI dynamic
library on Windows, macOS, Linux, Android, and iOS. The current implementation stores
download state in a `.part` file with dual metadata slots so interrupted
downloads can resume automatically.

## Workspace

- `takanawa-core`: chunk planning, `.part` metadata, file recovery, SHA-1/SHA-256/SHA-512/MD5/CRC32 hash checks.
- `takanawa-http`: Tokio/reqwest HTTP range download engine.
- `takanawa-ffi`: C ABI wrapper built as `cdylib` and `staticlib`.
- `takanawa-cli`: small dogfood CLI.
- `packages/takanawa-csharp`: C# SDK published as `YetAnotherAI.Takanawa`
  on NuGet for desktop .NET, Unity, Godot, Android, and iOS consumers.
- `android/takanawa-android`: Kotlin-first Android SDK published as an AAR.
- `packages/takanawa-js-core`: Private shared TypeScript facade bundled into
  npm target packages.
- `packages/takanawa-node`: Node.js and Electron bindings published to npm.
- `packages/takanawa-capacitor`: Capacitor plugin published to npm. The plugin
  ships Android and iOS bridge source and depends on the Android AAR and SwiftPM
  package at the same Takanawa version.
- `packages/takanawa-tauri`: Tauri v2 plugin published as the `takanawa-tauri`
  npm package and the `tauri-plugin-takanawa` Rust crate. The frontend package
  uses the shared TypeScript API while the Rust plugin compiles into the host
  Tauri app.

Default TLS uses `rustls` with bundled webpki roots via the `tls-rustls`
feature. Platform-native TLS can be selected with `default-features = false`
and the `tls-platform-native` or `tls-platform-roots` feature. That backend
uses the operating system TLS stack on Windows and macOS, and OpenSSL on Linux.

## Versioning

The release version is defined in the root `Cargo.toml` under
`[workspace.package]`. Gradle projects derive their `group` and `version` from
that value, and `crates/takanawa-core/tests/workspace_versions.rs` verifies that
published version references stay in sync.

To bump the release version and sync published references, run:

```sh
mise run version:sync <version>
```

## npm

The `npm` GitHub Actions workflow publishes all non-private packages under
`packages/*` when a `v*` tag is pushed. This includes `takanawa-node`,
`takanawa-capacitor`, and `takanawa-tauri`. The private `takanawa-js-core`
package is bundled into those target packages at build time and is not published
separately. The workflow builds each package before `npm publish` so generated
`dist` files and package-specific native artifacts are included in the packed
tarball.

## Android

The Android SDK is published as:

```kotlin
dependencies {
    implementation("ai.yetanother:takanawa-android:0.7.4")
}
```

Basic usage:

```kotlin
Takanawa.init()
TakanawaDownload.create(
    DownloadConfig(
        url = "https://example.com/file.bin",
        targetPath = "/data/user/0/example/cache/file.bin",
    ),
).use { download ->
    download.start()
    val snapshot = download.snapshot()
}
Takanawa.shutdown()
```

Build and verify the local AAR:

```sh
mise run package:android-aar
```

Publish to Maven local and build the smoke app against the local coordinates:

```sh
mise run publish:android-local
```

Maven Central releases are published from the `Publish` GitHub Actions workflow
when a `v*` tag is pushed. Configure a GitHub Environment named
`maven-central` with these secrets:

- `MAVEN_CENTRAL_USERNAME`: Central Portal user token username.
- `MAVEN_CENTRAL_PASSWORD`: Central Portal user token password.
- `SIGNING_IN_MEMORY_KEY`: ASCII-armored private GPG key.
- `SIGNING_IN_MEMORY_KEY_ID`: GPG key id.
- `SIGNING_IN_MEMORY_KEY_PASSWORD`: optional GPG key password.

The release job builds the Android native libraries and runs:

```sh
./gradlew -Ptakanawa.skipRustBuild=true :takanawa-android:publishAndReleaseToMavenCentral
```

The Capacitor plugin does not publish a separate Maven artifact; its Android
bridge is distributed in the npm package and depends on `takanawa-android`.

## SwiftPM

The SwiftPM package is distributed as a prebuilt `Takanawa.xcframework`.
The current Apple deployment targets are iOS 13.0, iOS Simulator 13.0, and
macOS 10.15.
The static XCFramework links against Apple's CoreFoundation and Security
frameworks, plus libiconv.

```bash
mise run package:swiftpm
mise run test:swift-integration
mise run swiftpm:release-manifest
```

The checked-in `Package.swift` uses the local `target/apple/Takanawa.xcframework`
path so development and CI do not need to precompute a future release checksum.
Release builds generate `target/swiftpm/Package.swift` with the checksum for the
uploaded `Takanawa.xcframework.zip`.

The Capacitor plugin does not publish a separate SwiftPM plugin artifact. Its
iOS bridge and `Takanawa.xcframework` are bundled in the npm package, and
`packages/takanawa-capacitor/Package.swift` uses the bundled binary by default.

## C# and NuGet

The C# SDK is published as:

```xml
<PackageReference Include="YetAnotherAI.Takanawa" Version="0.6.0" />
```

Basic usage:

```csharp
using YetAnotherAI.Takanawa;

Takanawa.Init();
using var download = TakanawaDownload.Create(new DownloadConfig(
    url: "https://example.com/file.bin",
    targetPath: "/tmp/file.bin"));
download.Start();
var snapshot = download.Snapshot();
Takanawa.Shutdown();
```

The package targets `netstandard2.0` and includes managed bindings plus native
runtime assets for desktop, Android, and Apple targets. Build and test locally:

```sh
mise run test:csharp
```

Release packing expects staged native artifacts from the release workflow:

```sh
mise run pack:csharp
```

## C and C++

C and C++ consumers can link the C ABI library with CMake:

```cmake
add_subdirectory(path/to/takanawa)
target_link_libraries(app PRIVATE Takanawa::takanawa)
```

The same CMake package is available through the local vcpkg overlay port:

```sh
vcpkg install takanawa --overlay-ports=/path/to/takanawa/ports
```

Build the CMake smoke fixture:

```sh
mise run test:cmake-integration
```
