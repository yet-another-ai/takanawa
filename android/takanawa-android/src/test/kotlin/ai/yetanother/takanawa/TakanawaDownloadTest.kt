package ai.yetanother.takanawa

import org.junit.Test

class TakanawaDownloadTest {
    @Test
    fun closeIsIdempotentWhenAlreadyClosed() {
        TakanawaDownload(0).close()
        TakanawaDownload(0).close()
    }
}
