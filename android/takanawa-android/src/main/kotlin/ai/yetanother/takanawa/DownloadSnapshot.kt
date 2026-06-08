package ai.yetanother.takanawa

data class DownloadSnapshot(
    val phase: DownloadPhase,
    val contentLen: Long,
    val downloadedBytes: Long,
    val chunkSize: Long,
    val chunkCount: Long,
    val completedChunks: Long,
    val activeIo: Int,
)
