#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

XCFRAMEWORK="target/apple/Takanawa.xcframework"
PACKAGE_DIR="target/swiftpm"
ZIP_PATH="$PACKAGE_DIR/Takanawa.xcframework.zip"
CHECKSUM_PATH="$ZIP_PATH.checksum"

if [[ ! -d "$XCFRAMEWORK" ]]; then
  echo "missing $XCFRAMEWORK; run mise run package:apple first" >&2
  exit 1
fi

mkdir -p "$PACKAGE_DIR"
rm -f "$ZIP_PATH" "$CHECKSUM_PATH"

ditto -c -k --sequesterRsrc --keepParent "$XCFRAMEWORK" "$ZIP_PATH"
swift package compute-checksum "$ZIP_PATH" > "$CHECKSUM_PATH"

echo "Created $ZIP_PATH"
echo "SwiftPM checksum: $(cat "$CHECKSUM_PATH")"
