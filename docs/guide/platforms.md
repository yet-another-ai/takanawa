# Platforms

Takanawa is designed to ship the same Rust download core through multiple
platform surfaces.

## Desktop

The workspace builds on Windows, macOS, and Linux. The FFI crate produces native
library artifacts for embedding through a stable C ABI.

```sh
mise run build:desktop
```

C and C++ consumers can use the generated CMake package target
`Takanawa::takanawa` or the local vcpkg overlay port.

```sh
mise run test:cmake-integration
```

## Android

Android packages include JNI libraries for supported ABIs and a Kotlin-first
SDK published as an AAR.

```sh
mise run package:android-aar
```

## Capacitor

Capacitor users can install the `takanawa-capacitor` package for Android and
iOS apps. Its TypeScript API mirrors the Node package while delegating work to
the native Android and Swift SDKs.

```sh
pnpm --filter takanawa-capacitor build
```

## Apple

Apple packages are distributed as a prebuilt `Takanawa.xcframework` for SwiftPM.
Current deployment targets are iOS 13.0, iOS Simulator 13.0, and macOS 10.15.

```sh
mise run package:apple
```
