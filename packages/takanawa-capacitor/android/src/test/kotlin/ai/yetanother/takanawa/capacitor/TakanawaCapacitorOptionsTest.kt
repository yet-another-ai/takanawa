package ai.yetanother.takanawa.capacitor

import ai.yetanother.takanawa.HashKind
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import org.json.JSONObject

class TakanawaCapacitorOptionsTest {
    @Test
    fun appliesMaxIoDefaultsAndZeroNormalization() {
        val base = baseOptions()

        assertEquals(4, TakanawaCapacitorOptions.parse(base).maxIo)
        assertEquals(1, TakanawaCapacitorOptions.parse(base.put("maxIo", 0)).maxIo)
        assertEquals(8, TakanawaCapacitorOptions.parse(base.put("maxIo", 8)).maxIo)
    }

    @Test
    fun parsesHashObjectsWithAliasesAndPrefixes() {
        val options = baseOptions().put(
            "hash",
            JSONObject()
                .put("kind", "sha-1")
                .put("expected", "sha-1:${"00".repeat(20)}"),
        )

        val config = TakanawaCapacitorOptions.parse(options).config

        assertEquals(HashKind.SHA1, config.hashKind)
        assertArrayEquals(ByteArray(20), config.expectedHash!!)
    }

    @Test
    fun parsesLegacySha256Option() {
        val config = TakanawaCapacitorOptions.parse(
            baseOptions().put("sha256", "11".repeat(32)),
        ).config

        assertEquals(HashKind.SHA256, config.hashKind)
        assertArrayEquals(ByteArray(32) { 0x11.toByte() }, config.expectedHash!!)
    }

    @Test
    fun rejectsInvalidHashes() {
        assertThrows(IllegalArgumentException::class.java) {
            TakanawaCapacitorOptions.parse(
                baseOptions().put(
                    "hash",
                    JSONObject().put("kind", "md5").put("expected", "00"),
                ),
            )
        }

        assertThrows(IllegalArgumentException::class.java) {
            TakanawaCapacitorOptions.parse(
                baseOptions()
                    .put("hash", JSONObject().put("kind", "sha256").put("expected", "00".repeat(32)))
                    .put("sha256", "11".repeat(32)),
            )
        }
    }

    @Test
    fun rejectsNegativeAndTooLargeValues() {
        assertThrows(IllegalArgumentException::class.java) {
            TakanawaCapacitorOptions.parse(baseOptions().put("chunkSize", -1))
        }

        assertThrows(IllegalArgumentException::class.java) {
            TakanawaCapacitorOptions.parse(baseOptions().put("chunkSize", Long.MAX_VALUE.toString() + "0"))
        }
    }

    private fun baseOptions(): JSONObject =
        JSONObject()
            .put("url", "https://example.test/file.bin")
            .put("targetPath", "/tmp/file.bin")
}
