package ai.yetanother.takanawa

data class DownloadConfig @JvmOverloads constructor(
    val url: String,
    val targetPath: String,
    val chunkSize: Long = 0,
    val parallelism: Int = 0,
    val maxParallelChunks: Int = 0,
    val maxRetries: Int = 4,
    val backoffInitialMillis: Long = 100,
    val backoffMaxMillis: Long = 3_000,
    val connectTimeoutMillis: Long = 30_000,
    val readTimeoutMillis: Long = 0,
    val totalTimeoutMillis: Long = 0,
    val bytesPerSecondLimit: Long = 0,
    val expectedSha256: ByteArray? = null,
) {
    init {
        require(url.isNotBlank()) { "url must not be blank" }
        require(targetPath.isNotBlank()) { "targetPath must not be blank" }
        require(chunkSize >= 0) { "chunkSize must be greater than or equal to 0" }
        require(parallelism >= 0) { "parallelism must be greater than or equal to 0" }
        require(maxParallelChunks >= 0) { "maxParallelChunks must be greater than or equal to 0" }
        require(maxRetries >= 0) { "maxRetries must be greater than or equal to 0" }
        require(backoffInitialMillis >= 0) {
            "backoffInitialMillis must be greater than or equal to 0"
        }
        require(backoffMaxMillis >= 0) { "backoffMaxMillis must be greater than or equal to 0" }
        require(connectTimeoutMillis >= 0) {
            "connectTimeoutMillis must be greater than or equal to 0"
        }
        require(readTimeoutMillis >= 0) { "readTimeoutMillis must be greater than or equal to 0" }
        require(totalTimeoutMillis >= 0) { "totalTimeoutMillis must be greater than or equal to 0" }
        require(bytesPerSecondLimit >= 0) {
            "bytesPerSecondLimit must be greater than or equal to 0"
        }
        require(expectedSha256 == null || expectedSha256.size == SHA256_LENGTH) {
            "expectedSha256 must be exactly $SHA256_LENGTH bytes"
        }
    }

    internal fun expectedSha256Copy(): ByteArray? = expectedSha256?.copyOf()

    private companion object {
        private const val SHA256_LENGTH = 32
    }
}
