package ai.yetanother.takanawa

data class DownloadSpeedSnapshot(
    val phase: DownloadPhase,
    val contentLen: Long,
    val receivedBytes: Long,
    val intervalBytes: Long,
    val elapsedMillis: Long,
    val bytesPerSecond: Double,
    val activeIo: Int,
)
