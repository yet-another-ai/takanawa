// swift-tools-version: 5.9
import Foundation
import PackageDescription

let useLocalTakanawa = ProcessInfo.processInfo.environment["TAKANAWA_CAPACITOR_USE_LOCAL_TAKANAWA"] == "1"
let takanawaDependency: Package.Dependency = useLocalTakanawa ?
  .package(path: "../../..") :
  .package(url: "https://github.com/yetanother.ai/takanawa.git", exact: "0.4.0")

let package = Package(
  name: "TakanawaCapacitor",
  platforms: [
    .iOS(.v14),
    .macOS(.v10_15)
  ],
  products: [
    .library(
      name: "TakanawaCapacitor",
      targets: ["TakanawaCapacitorPlugin"]
    )
  ],
  dependencies: [
    .package(url: "https://github.com/ionic-team/capacitor-swift-pm.git", from: "8.0.0"),
    takanawaDependency
  ],
  targets: [
    .target(
      name: "TakanawaCapacitorPlugin",
      dependencies: [
        .product(name: "Capacitor", package: "capacitor-swift-pm"),
        .product(name: "Takanawa", package: "takanawa")
      ],
      path: "Sources/TakanawaCapacitorPlugin"
    ),
    .testTarget(
      name: "TakanawaCapacitorPluginTests",
      dependencies: ["TakanawaCapacitorPlugin"],
      path: "Tests/TakanawaCapacitorPluginTests"
    )
  ]
)
