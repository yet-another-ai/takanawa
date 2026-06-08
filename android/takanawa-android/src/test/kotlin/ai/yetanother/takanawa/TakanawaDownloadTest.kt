package ai.yetanother.takanawa

import kotlin.test.Test

class TakanawaDownloadTest {
    @Test
    fun closeIsIdempotentWhenAlreadyClosed() {
        TakanawaDownload(0).close()
        TakanawaDownload(0).close()
    }
}
