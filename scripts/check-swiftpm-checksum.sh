#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

CHECKSUM_PATH="target/swiftpm/Takanawa.xcframework.zip.checksum"

if [[ ! -f "$CHECKSUM_PATH" ]]; then
  echo "missing $CHECKSUM_PATH; run mise run package:swiftpm first" >&2
  exit 1
fi

expected="$(tr -d '[:space:]' < "$CHECKSUM_PATH")"
actual="$(awk -F '"' '/^let checksum = / { print $2 }' Package.swift)"

if [[ "$actual" != "$expected" ]]; then
  echo "Package.swift checksum does not match the SwiftPM artifact." >&2
  echo "Package.swift: $actual" >&2
  echo "Artifact:      $expected" >&2
  echo "Run mise run swiftpm:update-checksum before tagging a release." >&2
  exit 1
fi

echo "Package.swift checksum matches $CHECKSUM_PATH"
