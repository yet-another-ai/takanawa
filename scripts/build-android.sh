#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  x86_64-linux-android \
  i686-linux-android

cargo ndk \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -t x86 \
  --platform 23 \
  -o target/android/jniLibs \
  build -p takanawa-ffi --release --locked --features jni
