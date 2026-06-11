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
    implementation("ai.yetanother:takanawa-android:0.3.1")
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

## C and C++

Build and link the C ABI library with CMake:

```cmake
add_subdirectory(path/to/takanawa)
target_link_libraries(app PRIVATE Takanawa::takanawa)
```

Verify the CMake smoke test:

```sh
mise run test:cmake-integration
```

Use the local vcpkg overlay port when consuming from a vcpkg project:

```sh
vcpkg install takanawa --overlay-ports=/path/to/takanawa/ports
```
