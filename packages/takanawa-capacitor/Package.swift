// swift-tools-version: 5.9
import Foundation
import PackageDescription

let useLocalTakanawa = ProcessInfo.processInfo.environment["TAKANAWA_CAPACITOR_USE_LOCAL_TAKANAWA"] == "1"
let takanawaDependencies: [Package.Dependency] = useLocalTakanawa ?
  [.package(path: "../..")] :
  []
let capacitorSwiftPmDependency: Package.Dependency =
  .package(url: "https://github.com/ionic-team/capacitor-swift-pm.git", exact: "8.3.4")
let takanawaPluginDependencies: [Target.Dependency] = useLocalTakanawa ?
  [
    .product(name: "Capacitor", package: "capacitor-swift-pm"),
    .product(name: "Cordova", package: "capacitor-swift-pm"),
    .product(name: "Takanawa", package: "takanawa")
  ] :
  [
    .product(name: "Capacitor", package: "capacitor-swift-pm"),
    .product(name: "Cordova", package: "capacitor-swift-pm"),
    "Takanawa"
  ]
let bundledTakanawaTargets: [Target] = useLocalTakanawa ?
  [] :
  [
    .target(
      name: "Takanawa",
      dependencies: ["TakanawaBinary"],
      path: "ios/Sources/Takanawa",
      linkerSettings: [
        .linkedFramework("CoreFoundation"),
        .linkedFramework("Security"),
        .linkedLibrary("iconv")
      ]
    ),
    .binaryTarget(
      name: "TakanawaBinary",
      path: "ios/Takanawa.xcframework"
    )
  ]

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
    capacitorSwiftPmDependency
  ] + takanawaDependencies,
  targets: [
    .target(
      name: "TakanawaCapacitorPlugin",
      dependencies: takanawaPluginDependencies,
      path: "ios/Sources/TakanawaCapacitorPlugin"
    ),
    .testTarget(
      name: "TakanawaCapacitorPluginTests",
      dependencies: ["TakanawaCapacitorPlugin"],
      path: "ios/Tests/TakanawaCapacitorPluginTests"
    )
  ] + bundledTakanawaTargets
)
