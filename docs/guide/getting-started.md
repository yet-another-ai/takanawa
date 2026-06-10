# Getting Started

Takanawa is organized as a Rust workspace with a core planner, an HTTP download
engine, native FFI bindings, and packaging for Android and Apple platforms.

## Rust Workspace

Run the full workspace check:

```sh
mise run check
```

Run tests:

```sh
mise run test
```

Build all distributable platform artifacts:

```sh
mise run package
```

## Android

The Android SDK is published as an AAR:

```kotlin
dependencies {
    implementation("ai.yetanother:takanawa-android:0.2.0")
}
```

Build and verify the local AAR:

```sh
mise run package:android-aar
```

## SwiftPM

Build the Apple XCFramework and SwiftPM binary artifact:

```sh
mise run package:swiftpm
```

Verify the SwiftPM smoke test:

```sh
mise run test:swift-integration
```
