#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

checksum="$(awk -F '"' '/^let checksum = / { print $2 }' Package.swift)"
version="$(awk -F '"' '/^let version = / { print $2 }' Package.swift)"

if [[ ! "$checksum" =~ ^[0-9a-f]{64}$ ]]; then
  echo "Package.swift does not contain a valid SwiftPM checksum." >&2
  exit 1
fi

if [[ "$checksum" =~ ^0{64}$ ]]; then
  echo "Package.swift still contains the placeholder SwiftPM checksum." >&2
  echo "Build the SwiftPM artifact, run mise run swiftpm:update-checksum, commit Package.swift, then tag the release." >&2
  exit 1
fi

if [[ -n "${GITHUB_REF_NAME:-}" && "${GITHUB_REF_NAME}" =~ ^v && "v${version}" != "${GITHUB_REF_NAME}" ]]; then
  echo "Package.swift version does not match the release tag." >&2
  echo "Package.swift: v${version}" >&2
  echo "Tag:           ${GITHUB_REF_NAME}" >&2
  exit 1
fi

echo "Package.swift is ready for SwiftPM release v${version}"
