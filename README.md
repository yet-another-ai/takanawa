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

Default TLS uses `rustls` with bundled webpki roots via the `tls-rustls`
feature. Platform certificate roots are reserved for a future feature flag.

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
