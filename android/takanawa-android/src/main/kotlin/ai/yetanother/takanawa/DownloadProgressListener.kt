package ai.yetanother.takanawa

fun interface DownloadProgressListener {
    fun onProgress(snapshot: DownloadSnapshot)
}
