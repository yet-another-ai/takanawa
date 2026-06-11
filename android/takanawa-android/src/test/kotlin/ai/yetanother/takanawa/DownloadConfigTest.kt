package ai.yetanother.takanawa

import org.junit.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class DownloadConfigTest {
    @Test
    fun rejectsInvalidSha256Length() {
        assertFailsWith<IllegalArgumentException> {
            DownloadConfig(
                url = "https://example.test/file.bin",
                targetPath = "/tmp/file.bin",
                expectedSha256 = ByteArray(31),
            )
        }
    }

    @Test
    fun acceptsSupportedHashKinds() {
        val cases = mapOf(
            HashKind.SHA1 to 20,
            HashKind.SHA256 to 32,
            HashKind.SHA512 to 64,
            HashKind.MD5 to 16,
            HashKind.CRC32 to 4,
        )

        for ((hashKind, length) in cases) {
            val config = DownloadConfig(
                url = "https://example.test/file.bin",
                targetPath = "/tmp/file.bin",
                hashKind = hashKind,
                expectedHash = ByteArray(length),
            )

            assertEquals(hashKind, config.hashKind)
            assertEquals(length, config.expectedHashCopy()?.size)
        }
    }

    @Test
    fun rejectsInvalidHashLength() {
        assertFailsWith<IllegalArgumentException> {
            DownloadConfig(
                url = "https://example.test/file.bin",
                targetPath = "/tmp/file.bin",
                hashKind = HashKind.SHA1,
                expectedHash = ByteArray(32),
            )
        }
    }

    @Test
    fun rejectsNegativeTuningValues() {
        assertFailsWith<IllegalArgumentException> {
            DownloadConfig(
                url = "https://example.test/file.bin",
                targetPath = "/tmp/file.bin",
                chunkSize = -1,
            )
        }

        assertFailsWith<IllegalArgumentException> {
            DownloadConfig(
                url = "https://example.test/file.bin",
                targetPath = "/tmp/file.bin",
                parallelism = -1,
            )
        }

        assertFailsWith<IllegalArgumentException> {
            DownloadConfig(
                url = "https://example.test/file.bin",
                targetPath = "/tmp/file.bin",
                maxParallelChunks = -1,
            )
        }

        assertFailsWith<IllegalArgumentException> {
            DownloadConfig(
                url = "https://example.test/file.bin",
                targetPath = "/tmp/file.bin",
                maxRetries = -1,
            )
        }

        assertFailsWith<IllegalArgumentException> {
            DownloadConfig(
                url = "https://example.test/file.bin",
                targetPath = "/tmp/file.bin",
                bytesPerSecondLimit = -1,
            )
        }
    }
}
