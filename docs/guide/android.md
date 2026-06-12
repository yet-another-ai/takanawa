# Android

Use `takanawa-android` for native Kotlin or Java Android apps. The AAR includes
the Kotlin SDK and JNI libraries for supported Android ABIs.

## Install

Add the Maven Central artifact to the app or library module:

```kotlin
dependencies {
    implementation("ai.yetanother:takanawa-android:0.4.0")
}
```

The Android SDK currently uses `minSdk = 23` and is built with Java 17.

## Usage

```kotlin
import ai.yetanother.takanawa.DownloadConfig
import ai.yetanother.takanawa.HashKind
import ai.yetanother.takanawa.Takanawa
import ai.yetanother.takanawa.TakanawaDownload

Takanawa.init(maxIo = 4)

val download = TakanawaDownload.create(
    DownloadConfig(
        url = "https://example.com/file.zip",
        targetPath = cacheDir.resolve("file.zip").absolutePath,
        parallelism = 4,
        hashKind = HashKind.SHA256,
        expectedHash = expectedSha256Bytes,
    )
)

download.setProgressCallback { snapshot ->
    println("${snapshot.phase}: ${snapshot.downloadedBytes}/${snapshot.contentLen}")
}

download.start()
```

Call `close()` when a task is no longer needed, or use Kotlin/Java resource
management around `TakanawaDownload`.

## Local Development

Build and verify the local AAR:

```sh
mise run package:android-aar
```

Publish to Maven local for smoke tests:

```sh
./gradlew :takanawa-android:publishToMavenLocal
```

Run the Android SDK tests:

```sh
./gradlew -Ptakanawa.skipRustBuild=true :takanawa-android:test
```

Capacitor apps should install [the Capacitor npm plugin](./capacitor) instead
of depending on this AAR directly.
