-keep class ai.yetanother.takanawa.NativeBridge { *; }
-keep class ai.yetanother.takanawa.DownloadPhase { *; }
-keep class ai.yetanother.takanawa.DownloadSnapshot { *; }
-keep class ai.yetanother.takanawa.DownloadSpeedSnapshot { *; }
-keep interface ai.yetanother.takanawa.DownloadProgressListener { *; }
-keep interface ai.yetanother.takanawa.DownloadSpeedListener { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}
