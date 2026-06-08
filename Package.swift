// swift-tools-version: 5.9
import PackageDescription

let version = "0.1.0"
let checksum = "0000000000000000000000000000000000000000000000000000000000000000"

let package = Package(
  name: "Takanawa",
  platforms: [
    .iOS(.v13),
    .macOS(.v10_15)
  ],
  products: [
    .library(
      name: "Takanawa",
      targets: ["Takanawa"]
    )
  ],
  targets: [
    .target(
      name: "Takanawa",
      dependencies: ["TakanawaBinary"],
      linkerSettings: [
        .linkedFramework("CoreFoundation"),
        .linkedFramework("Security"),
        .linkedLibrary("iconv")
      ]
    ),
    .binaryTarget(
      name: "TakanawaBinary",
      url: "https://github.com/yetanother.ai/takanawa/releases/download/v\(version)/Takanawa.xcframework.zip",
      checksum: checksum
    )
  ]
)
