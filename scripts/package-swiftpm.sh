#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

XCFRAMEWORK="target/apple/Takanawa.xcframework"
PACKAGE_DIR="target/swiftpm"
ZIP_PATH="$PACKAGE_DIR/Takanawa.xcframework.zip"
CHECKSUM_PATH="$ZIP_PATH.checksum"
STAGING_DIR="$PACKAGE_DIR/staging"

if [[ ! -d "$XCFRAMEWORK" ]]; then
  echo "missing $XCFRAMEWORK; run mise run package:apple first" >&2
  exit 1
fi

mkdir -p "$PACKAGE_DIR"
rm -f "$ZIP_PATH" "$CHECKSUM_PATH"
rm -rf "$STAGING_DIR"

mkdir -p "$STAGING_DIR"
ditto "$XCFRAMEWORK" "$STAGING_DIR/Takanawa.xcframework"
find "$STAGING_DIR" -exec touch -h -t 202001010000.00 {} +
(
  cd "$STAGING_DIR"
  find Takanawa.xcframework -print | LC_ALL=C sort | zip -X -q -@ "../Takanawa.xcframework.zip"
)
swift package compute-checksum "$ZIP_PATH" > "$CHECKSUM_PATH"

echo "Created $ZIP_PATH"
echo "SwiftPM checksum: $(cat "$CHECKSUM_PATH")"
