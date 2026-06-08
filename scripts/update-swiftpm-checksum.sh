#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

CHECKSUM_PATH="target/swiftpm/Takanawa.xcframework.zip.checksum"

if [[ ! -f "$CHECKSUM_PATH" ]]; then
  echo "missing $CHECKSUM_PATH; run mise run package:swiftpm first" >&2
  exit 1
fi

checksum="$(tr -d '[:space:]' < "$CHECKSUM_PATH")"

perl -0pi -e "s/let checksum = \"[0-9a-f]{64}\"/let checksum = \"$checksum\"/" Package.swift

echo "Updated Package.swift checksum to $checksum"
