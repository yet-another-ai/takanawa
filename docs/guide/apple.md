# Apple and SwiftPM

Use the SwiftPM package for native Swift apps. The package exposes the Swift
SDK and links a prebuilt `Takanawa.xcframework`.

## Install

Add the repository as a Swift Package dependency:

```swift
.package(url: "https://github.com/yetanother.ai/takanawa.git", exact: "{{ takanawaVersion }}")
```

Then depend on the `Takanawa` product:

```swift
.product(name: "Takanawa", package: "takanawa")
```

Current deployment targets are iOS 13.0, iOS Simulator 13.0, and macOS 10.15.

## Usage

```swift
import Foundation
import Takanawa

try Takanawa.initialize(maxIo: 4)

let download = try TakanawaDownload.create(
  DownloadConfig(
    url: "https://example.com/file.zip",
    targetPath: destination.path,
    parallelism: 4,
    hashKind: .sha256,
    expectedHash: expectedSha256Data
  )
)

try download.setProgressCallback { snapshot in
  print("\(snapshot.phase): \(snapshot.downloadedBytes)/\(snapshot.contentLen)")
}
try download.setSpeedCallback { snapshot in
  print("\(snapshot.bytesPerSecond) B/s")
}

try download.start()
```

Call `close()` when a task is no longer needed. The Swift wrapper also releases
the native handle during deinitialization.

## Local Development

Build the Apple XCFramework:

```sh
mise run package:apple
```

Build the release SwiftPM binary artifact:

```sh
mise run package:swiftpm
```

Run the SwiftPM integration smoke test on a machine with the required Apple SDK:

```sh
mise run test:swift-integration
```

Capacitor apps should install [the Capacitor npm plugin](./capacitor) instead
of depending on this SwiftPM package directly.
