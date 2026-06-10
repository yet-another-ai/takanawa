package ai.yetanother.takanawa

internal object NativeBridge {
    init {
        System.loadLibrary("takanawa_ffi")
    }

    @JvmStatic
    external fun globalInit(maxIo: Int): Int

    @JvmStatic
    external fun globalShutdown(): Int

    @JvmStatic
    external fun globalSetMaxIo(maxIo: Int): Int

    @JvmStatic
    external fun downloadCreate(
        url: String,
        targetPath: String,
        chunkSize: Long,
        parallelism: Int,
        maxParallelChunks: Int,
        maxRetries: Int,
        backoffInitialMillis: Long,
        backoffMaxMillis: Long,
        connectTimeoutMillis: Long,
        readTimeoutMillis: Long,
        totalTimeoutMillis: Long,
        bytesPerSecondLimit: Long,
        expectedSha256: ByteArray?,
        outHandle: LongArray,
    ): Int

    @JvmStatic
    external fun downloadStart(handle: Long): Int

    @JvmStatic
    external fun downloadPause(handle: Long): Int

    @JvmStatic
    external fun downloadCancel(handle: Long): Int

    @JvmStatic
    external fun downloadSnapshot(handle: Long, outSnapshot: LongArray): Int

    @JvmStatic
    external fun downloadBitmapSize(handle: Long, outSize: LongArray): Int

    @JvmStatic
    external fun downloadCopyBitmap(handle: Long, outBitmap: ByteArray): Int

    @JvmStatic
    external fun downloadLastError(handle: Long): String

    @JvmStatic
    external fun downloadRelease(handle: Long): Int
}
