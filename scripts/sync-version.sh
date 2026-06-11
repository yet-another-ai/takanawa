#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

version="$(
  awk '
    /^\[workspace.package\]$/ { in_workspace_package = 1; next }
    /^\[/ { in_workspace_package = 0 }
    in_workspace_package && /^version = / {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' Cargo.toml
)"

if [[ -z "$version" ]]; then
  echo "missing [workspace.package] version in Cargo.toml" >&2
  exit 1
fi

perl -0pi -e "s/(takanawa-core = \\{ version = \")[^\"]+(\", path = \"crates\\/takanawa-core\" \\})/\${1}${version}\${2}/" Cargo.toml
perl -0pi -e "s/(takanawa-http = \\{ version = \")[^\"]+(\", path = \"crates\\/takanawa-http\")/\${1}${version}\${2}/" Cargo.toml
perl -0pi -e "s/(project\\(Takanawa VERSION )[0-9]+\\.[0-9]+\\.[0-9]+/\${1}${version}/" CMakeLists.txt
perl -0pi -e "s/(\"version\": \")[^\"]+(\")/\${1}${version}\${2}/" ports/takanawa/vcpkg.json

android_version_reference_files=(
  README.md
  docs/api/index.md
  docs/guide/getting-started.md
)

for file in "${android_version_reference_files[@]}"; do
  perl -0pi -e "s/(implementation\\(\"ai\\.yetanother:takanawa-android:)[^\"]+(\"\\))/\${1}${version}\${2}/g" "$file"
done

echo "Synced release version references to $version"
