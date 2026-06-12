#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="${1:-${GITHUB_REF_NAME:-}}"
CHECKSUM_PATH="target/swiftpm/Takanawa.xcframework.zip.checksum"
OUTPUT_PATH="${TAKANAWA_SWIFTPM_RELEASE_MANIFEST:-target/swiftpm/Package.swift}"

if [[ -z "$VERSION" ]]; then
  VERSION="$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)"
fi

VERSION="${VERSION#v}"

if [[ -z "$VERSION" ]]; then
  echo "missing release version; pass vX.Y.Z, set GITHUB_REF_NAME, or define workspace.package version" >&2
  exit 1
fi

if [[ ! -f "$CHECKSUM_PATH" ]]; then
  echo "missing $CHECKSUM_PATH; run mise run package:swiftpm first" >&2
  exit 1
fi

checksum="$(tr -d '[:space:]' < "$CHECKSUM_PATH")"

mkdir -p "$(dirname "$OUTPUT_PATH")"

cat > "$OUTPUT_PATH" <<SWIFT
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
      url: "https://github.com/yet-another-ai/takanawa/releases/download/v${VERSION}/Takanawa.xcframework.zip",
      checksum: "${checksum}"
    )
  ]
)
SWIFT

echo "Generated $OUTPUT_PATH for v$VERSION"
