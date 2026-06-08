# Takanawa

Takanawa is a Rust range-download library designed to ship as a C ABI dynamic
library on Windows, macOS, Linux, Android, and iOS. The current implementation stores
download state in a `.part` file with dual metadata slots so interrupted
downloads can resume automatically.

## Workspace

- `takanawa-core`: chunk planning, `.part` metadata, file recovery, hash checks.
- `takanawa-http`: Tokio/reqwest HTTP range download engine.
- `takanawa-ffi`: C ABI wrapper built as `cdylib` and `staticlib`.
- `takanawa-cli`: small dogfood CLI.
- `android/takanawa-android`: Kotlin-first Android SDK published as an AAR.

Default TLS uses `rustls` with bundled webpki roots via the `tls-rustls`
feature. Platform certificate roots are reserved for a future feature flag.

## Android

The Android SDK is published as:

```kotlin
dependencies {
    implementation("ai.yetanother:takanawa-android:0.1.0")
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
- `SIGNING_IN_MEMORY_KEY_ID`: optional GPG key id.
- `SIGNING_IN_MEMORY_KEY_PASSWORD`: optional GPG key password.

The release job builds the Android native libraries and runs:

```sh
./gradlew -Ptakanawa.skipRustBuild=true :takanawa-android:publishAndReleaseToMavenCentral
```

## SwiftPM

The SwiftPM package is distributed as a prebuilt `Takanawa.xcframework`.
The current Apple deployment targets are iOS 13.0, iOS Simulator 13.0, and
macOS 10.15.
The static XCFramework links against Apple's CoreFoundation and Security
frameworks, plus libiconv.

```bash
mise run package:swiftpm
mise run swiftpm:update-checksum
mise run test:swift-integration
```

Publishing expects `Takanawa.xcframework.zip` to be uploaded to the matching
GitHub release tag, for example `v0.1.0`. Update the checksum in `Package.swift`
with the value from `target/swiftpm/Takanawa.xcframework.zip.checksum` before
tagging a release.
