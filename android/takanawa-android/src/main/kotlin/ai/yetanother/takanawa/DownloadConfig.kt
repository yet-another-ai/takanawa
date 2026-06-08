package ai.yetanother.takanawa

data class DownloadConfig @JvmOverloads constructor(
    val url: String,
    val targetPath: String,
    val chunkSize: Long = 0,
    val parallelism: Int = 0,
    val expectedSha256: ByteArray? = null,
) {
    init {
        require(url.isNotBlank()) { "url must not be blank" }
        require(targetPath.isNotBlank()) { "targetPath must not be blank" }
        require(chunkSize >= 0) { "chunkSize must be greater than or equal to 0" }
        require(parallelism >= 0) { "parallelism must be greater than or equal to 0" }
        require(expectedSha256 == null || expectedSha256.size == SHA256_LENGTH) {
            "expectedSha256 must be exactly $SHA256_LENGTH bytes"
        }
    }

    internal fun expectedSha256Copy(): ByteArray? = expectedSha256?.copyOf()

    private companion object {
        private const val SHA256_LENGTH = 32
    }
}
