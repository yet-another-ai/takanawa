package ai.yetanother.takanawa

import org.junit.Test
import kotlin.test.assertEquals

class TakanawaStatusTest {
    @Test
    fun mapsKnownStatusCodes() {
        assertEquals(TakanawaStatus.OK, TakanawaStatus.fromCode(0))
        assertEquals(TakanawaStatus.BUFFER_TOO_SMALL, TakanawaStatus.fromCode(1))
        assertEquals(TakanawaStatus.NETWORK, TakanawaStatus.fromCode(-21))
    }

    @Test
    fun mapsUnknownStatusCodeToInternal() {
        assertEquals(TakanawaStatus.INTERNAL, TakanawaStatus.fromCode(-9999))
    }
}
