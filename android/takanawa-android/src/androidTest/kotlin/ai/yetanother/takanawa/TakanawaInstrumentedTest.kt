package ai.yetanother.takanawa

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class TakanawaInstrumentedTest {
    @Test
    fun createsAndReleasesDownloadHandle() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val target = File(context.cacheDir, "takanawa-smoke.bin")

        Takanawa.init(maxIo = 2)
        TakanawaDownload.create(
            DownloadConfig(
                url = "https://example.test/file.bin",
                targetPath = target.absolutePath,
            ),
        ).use { download ->
            assertEquals(DownloadPhase.CREATED, download.snapshot().phase)
        }
        Takanawa.shutdown()
    }
}
