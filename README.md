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

## CocoaPods

The CocoaPods package is distributed as a prebuilt `Takanawa.xcframework`.

```bash
bundle install
mise run package:cocoapods
mise run lint:cocoapods
```

Publishing expects `Takanawa.xcframework.zip` to be uploaded to the matching
GitHub release tag, for example `v0.1.0`. The podspec reads
`TAKANAWA_XCFRAMEWORK_URL` and `TAKANAWA_XCFRAMEWORK_SHA256` for local linting;
without those variables it points at the GitHub release asset.
