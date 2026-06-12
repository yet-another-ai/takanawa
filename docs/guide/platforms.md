# Platforms

Takanawa is designed to ship the same Rust download core through multiple
target surfaces. Each target has its own installation path.

| Target | Package | Install From | Use When |
| --- | --- | --- | --- |
| [Rust](./rust) | `takanawa-core`, `takanawa-http`, `takanawa-ffi` | crates.io or workspace path | You are building a Rust application or embedding the core directly. |
| [Node and Electron](./node) | `takanawa-node` | npm | You need Node-API bindings on desktop/server JavaScript. |
| [Capacitor](./capacitor) | `takanawa-capacitor` | npm | You are building a Capacitor v8 Android/iOS app and want a TypeScript API. |
| [Tauri](./tauri) | `takanawa-tauri`, `tauri-plugin-takanawa` | npm and crates.io | You are building a Tauri v2 desktop app and want the shared TypeScript API. |
| [Android](./android) | `ai.yetanother:takanawa-android` | Maven Central | You are building a native Kotlin or Java Android app. |
| [Apple and SwiftPM](./apple) | `Takanawa` | SwiftPM binary target | You are building a native Swift app on iOS or macOS. |
| [C and C++](./c-cpp) | `Takanawa::takanawa` | CMake or vcpkg overlay | You need the stable C ABI from C or C++. |

The Node, Capacitor, and Tauri packages use the same `takanawa-js-core` facade for
option names, hash forms, phase strings, snapshot fields, listener handles, and
promise-returning task methods. Native Android and Swift SDKs keep idiomatic
Kotlin and Swift names while sharing the same download core.
