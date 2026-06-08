#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

rustup target add \
  aarch64-apple-darwin \
  x86_64-apple-darwin \
  aarch64-apple-ios \
  aarch64-apple-ios-sim \
  x86_64-apple-ios

for target in \
  aarch64-apple-darwin \
  x86_64-apple-darwin \
  aarch64-apple-ios \
  aarch64-apple-ios-sim \
  x86_64-apple-ios
do
  cargo build -p takanawa-ffi --release --target "$target"
done

mkdir -p target/apple/macos target/apple/ios-simulator

lipo -create \
  target/aarch64-apple-darwin/release/libtakanawa_ffi.a \
  target/x86_64-apple-darwin/release/libtakanawa_ffi.a \
  -output target/apple/macos/libtakanawa_ffi.a

lipo -create \
  target/aarch64-apple-ios-sim/release/libtakanawa_ffi.a \
  target/x86_64-apple-ios/release/libtakanawa_ffi.a \
  -output target/apple/ios-simulator/libtakanawa_ffi.a

rm -rf target/apple/Takanawa.xcframework
xcodebuild -create-xcframework \
  -library target/apple/macos/libtakanawa_ffi.a -headers include \
  -library target/aarch64-apple-ios/release/libtakanawa_ffi.a -headers include \
  -library target/apple/ios-simulator/libtakanawa_ffi.a -headers include \
  -output target/apple/Takanawa.xcframework
