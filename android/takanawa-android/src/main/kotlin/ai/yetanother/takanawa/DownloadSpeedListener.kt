package ai.yetanother.takanawa

fun interface DownloadSpeedListener {
    fun onSpeed(snapshot: DownloadSpeedSnapshot)
}
