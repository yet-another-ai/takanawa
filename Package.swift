// swift-tools-version: 5.9
import PackageDescription

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
      path: "target/apple/Takanawa.xcframework"
    )
  ]
)
