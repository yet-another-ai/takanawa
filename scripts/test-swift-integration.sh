#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

ZIP_PATH="${TAKANAWA_XCFRAMEWORK_ZIP:-target/swiftpm/Takanawa.xcframework.zip}"
XCFRAMEWORK_PATH="${TAKANAWA_XCFRAMEWORK_PATH:-target/apple/Takanawa.xcframework}"
FIXTURE_DIR="fixtures/swift-integration"
WORK_DIR="target/swift-integration"
PACKAGE_DIR="$WORK_DIR/package"

rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR"
cp -R "$FIXTURE_DIR" "$PACKAGE_DIR"
rm -rf "$PACKAGE_DIR/Sources/Takanawa"
cp -R Sources/Takanawa "$PACKAGE_DIR/Sources/Takanawa"

if [[ -f "$ZIP_PATH" ]]; then
  unzip -q "$ZIP_PATH" -d "$PACKAGE_DIR"
elif [[ -d "$XCFRAMEWORK_PATH" ]]; then
  cp -R "$XCFRAMEWORK_PATH" "$PACKAGE_DIR/Takanawa.xcframework"
else
  echo "missing SwiftPM zip or XCFramework; run mise run package:apple first" >&2
  exit 1
fi

if [[ ! -d "$PACKAGE_DIR/Takanawa.xcframework" ]]; then
  echo "Takanawa.xcframework was not found in the Swift integration package" >&2
  exit 1
fi

swift build --package-path "$PACKAGE_DIR"
swift run --package-path "$PACKAGE_DIR" TakanawaSmoke
