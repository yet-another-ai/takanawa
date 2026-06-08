package ai.yetanother.takanawa

import kotlin.test.Test
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
    }
}
