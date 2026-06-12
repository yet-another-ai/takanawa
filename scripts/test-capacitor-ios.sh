#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

PACKAGE_DIR="packages/takanawa-capacitor/ios"
XCFRAMEWORK_PATH="target/apple/Takanawa.xcframework"

if [[ ! -d "$XCFRAMEWORK_PATH" ]]; then
  echo "missing $XCFRAMEWORK_PATH; run mise run package:apple first" >&2
  exit 1
fi

sdk_path="$(xcrun --sdk iphonesimulator --show-sdk-path)"
host_arch="$(uname -m)"

case "$host_arch" in
  arm64)
    triple="arm64-apple-ios-simulator"
    ;;
  x86_64)
    triple="x86_64-apple-ios-simulator"
    ;;
  *)
    echo "unsupported host architecture for iOS simulator build: $host_arch" >&2
    exit 1
    ;;
esac

TAKANAWA_CAPACITOR_USE_LOCAL_TAKANAWA=1 \
  swift build \
    --package-path "$PACKAGE_DIR" \
    --triple "$triple" \
    -Xswiftc -sdk \
    -Xswiftc "$sdk_path" \
    -Xcc -isysroot \
    -Xcc "$sdk_path"
