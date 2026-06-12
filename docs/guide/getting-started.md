# Getting Started

Takanawa is organized as a Rust workspace with a core planner, an HTTP download
engine, native FFI bindings, and package targets for JavaScript, Android, Apple,
C#, and C/C++ consumers.

Start from the target you are integrating:

- [Rust](./rust): use the workspace crates directly.
- [Node and Electron](./node): install the Node-API npm package.
- [Capacitor](./capacitor): install the Capacitor v8 npm plugin for Android and
  iOS apps.
- [Tauri](./tauri): install the Tauri v2 npm package and Rust plugin crate.
- [Android](./android): install the Kotlin-first AAR from Maven Central.
- [Apple and SwiftPM](./apple): install the SwiftPM package backed by the
  prebuilt XCFramework.
- [C# and NuGet](./csharp): install the `YetAnotherAI.Takanawa` package for
  desktop .NET, Unity, Godot, Android, or iOS.
- [C and C++](./c-cpp): link the C ABI library through CMake or vcpkg.

Use the [target matrix](./platforms) when deciding which package matches a
runtime.

## Workspace Checks

These commands are for contributors working in this repository.

Run the full workspace check:

```sh
mise run check
```

Run tests:

```sh
mise run test
```

Build all distributable artifacts that are available on the current machine:

```sh
mise run package
```
