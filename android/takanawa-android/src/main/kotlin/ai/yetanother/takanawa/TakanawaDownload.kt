package ai.yetanother.takanawa

import java.io.Closeable
import java.util.concurrent.atomic.AtomicLong

class TakanawaDownload internal constructor(handle: Long) : Closeable {
    private val handle = AtomicLong(handle)

    fun start() {
        withHandle { checkStatus(NativeBridge.downloadStart(it), it) }
    }

    fun pause() {
        withHandle { checkStatus(NativeBridge.downloadPause(it), it) }
    }

    fun cancel() {
        withHandle { checkStatus(NativeBridge.downloadCancel(it), it) }
    }

    fun snapshot(): DownloadSnapshot = withHandle { currentHandle ->
        val values = LongArray(SNAPSHOT_FIELD_COUNT)
        checkStatus(NativeBridge.downloadSnapshot(currentHandle, values), currentHandle)
        DownloadSnapshot(
            phase = DownloadPhase.fromCode(values[0].toInt()),
            contentLen = values[1],
            downloadedBytes = values[2],
            chunkSize = values[3],
            chunkCount = values[4],
            completedChunks = values[5],
            activeIo = values[6].toInt(),
        )
    }

    fun copyBitmap(): ByteArray = withHandle { currentHandle ->
        val size = LongArray(1)
        checkStatus(NativeBridge.downloadBitmapSize(currentHandle, size), currentHandle)
        check(size[0] <= Int.MAX_VALUE) { "bitmap is too large to copy into a ByteArray" }
        val bitmap = ByteArray(size[0].toInt())
        if (bitmap.isNotEmpty()) {
            checkStatus(NativeBridge.downloadCopyBitmap(currentHandle, bitmap), currentHandle)
        }
        bitmap
    }

    fun lastError(): String = withHandle { NativeBridge.downloadLastError(it) }

    override fun close() {
        val currentHandle = handle.getAndSet(CLOSED_HANDLE)
        if (currentHandle != CLOSED_HANDLE) {
            checkStatus(NativeBridge.downloadRelease(currentHandle))
        }
    }

    private inline fun <T> withHandle(block: (Long) -> T): T {
        val currentHandle = handle.get()
        if (currentHandle == CLOSED_HANDLE) {
            throw TakanawaException(TakanawaStatus.INVALID_CONFIG, "download is closed")
        }
        return block(currentHandle)
    }

    companion object {
        private const val CLOSED_HANDLE = 0L
        private const val SNAPSHOT_FIELD_COUNT = 7

        @JvmStatic
        fun create(config: DownloadConfig): TakanawaDownload {
            val outHandle = LongArray(1)
            checkStatus(
                NativeBridge.downloadCreate(
                    config.url,
                    config.targetPath,
                    config.chunkSize,
                    config.parallelism,
                    config.maxParallelChunks,
                    config.maxRetries,
                    config.backoffInitialMillis,
                    config.backoffMaxMillis,
                    config.connectTimeoutMillis,
                    config.readTimeoutMillis,
                    config.totalTimeoutMillis,
                    config.bytesPerSecondLimit,
                    config.expectedSha256Copy(),
                    outHandle,
                ),
            )
            return TakanawaDownload(outHandle[0])
        }
    }
}
