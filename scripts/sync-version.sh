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

npm_package_manifests=(package.json packages/*/package.json)
for file in "${npm_package_manifests[@]}"; do
  [[ -e "$file" ]] || continue
  perl -0pi -e "s/(^\\s*\"version\": \")[^\"]+(\")/\${1}${version}\${2}/m" "$file"
done

if [[ -f packages/takanawa-capacitor/android/build.gradle ]]; then
  perl -0pi -e "s/(def takanawaVersion = \")[^\"]+(\")/\${1}${version}\${2}/" packages/takanawa-capacitor/android/build.gradle
fi

if [[ -f packages/takanawa-capacitor/ios/Package.swift ]]; then
  perl -0pi -e "s/(github\\.com\\/yetanother\\.ai\\/takanawa\\.git\", exact: \")[^\"]+(\")/\${1}${version}\${2}/" packages/takanawa-capacitor/ios/Package.swift
fi

perl -0pi -e "s/(implementation\\(\"ai\\.yetanother:takanawa-android:)[^\"]+(\"\\))/\${1}${version}\${2}/g" README.md
perl -0pi -e "s/(takanawa-tauri = \\{ package = \"tauri-plugin-takanawa\", version = \")[^\"]+(\")/\${1}${version}\${2}/g" packages/takanawa-tauri/README.md

echo "Synced release version references to $version"
