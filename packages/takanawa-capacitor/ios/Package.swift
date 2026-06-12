// swift-tools-version: 5.9
import Foundation
import PackageDescription

let useLocalTakanawa = ProcessInfo.processInfo.environment["TAKANAWA_CAPACITOR_USE_LOCAL_TAKANAWA"] == "1"
let takanawaDependency: Package.Dependency = useLocalTakanawa ?
  .package(path: "../../..") :
  .package(url: "https://github.com/yet-another-ai/takanawa.git", exact: "0.5.0")
let capacitorSwiftPmDependency: Package.Dependency =
  .package(url: "https://github.com/ionic-team/capacitor-swift-pm.git", exact: "8.3.4")

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
    capacitorSwiftPmDependency,
    takanawaDependency
  ],
  targets: [
    .target(
      name: "TakanawaCapacitorPlugin",
      dependencies: [
        .product(name: "Capacitor", package: "capacitor-swift-pm"),
        .product(name: "Cordova", package: "capacitor-swift-pm"),
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
