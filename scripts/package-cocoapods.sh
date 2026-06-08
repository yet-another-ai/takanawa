#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

XCFRAMEWORK="target/apple/Takanawa.xcframework"
PACKAGE_DIR="target/cocoapods"
ZIP_PATH="$PACKAGE_DIR/Takanawa.xcframework.zip"
CHECKSUM_PATH="$ZIP_PATH.sha256"

if [[ ! -d "$XCFRAMEWORK" ]]; then
  echo "missing $XCFRAMEWORK; run mise run package:apple first" >&2
  exit 1
fi

mkdir -p "$PACKAGE_DIR"
rm -f "$ZIP_PATH" "$CHECKSUM_PATH"

ditto -c -k --sequesterRsrc --keepParent "$XCFRAMEWORK" "$ZIP_PATH"
shasum -a 256 "$ZIP_PATH" | awk '{ print $1 }' > "$CHECKSUM_PATH"

echo "Created $ZIP_PATH"
echo "SHA-256: $(cat "$CHECKSUM_PATH")"
