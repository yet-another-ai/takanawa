// swift-tools-version: 5.9
import PackageDescription

let package = Package(
  name: "TakanawaSwiftIntegration",
  platforms: [
    .macOS(.v12)
  ],
  targets: [
    .executableTarget(
      name: "TakanawaSmoke",
      dependencies: ["TakanawaLinkage"]
    ),
    .target(
      name: "TakanawaLinkage",
      dependencies: ["Takanawa"],
      linkerSettings: [
        .linkedFramework("CoreFoundation"),
        .linkedFramework("Security"),
        .linkedLibrary("iconv")
      ]
    ),
    .binaryTarget(
      name: "Takanawa",
      path: "Takanawa.xcframework"
    )
  ]
)
