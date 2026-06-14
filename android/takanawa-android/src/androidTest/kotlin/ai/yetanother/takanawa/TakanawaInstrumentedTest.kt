package ai.yetanother.takanawa

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
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

    @Test
    fun receivesProgressCallbackFromNativeWorkerThread() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val target = File(context.cacheDir, "takanawa-callback-smoke.bin")
        val terminal = AtomicReference<DownloadSnapshot?>()
        val latch = CountDownLatch(1)

        Takanawa.init(maxIo = 2)
        try {
            TakanawaDownload.create(
                DownloadConfig(
                    url = "http://127.0.0.1:1/takanawa-callback-smoke.bin",
                    targetPath = target.absolutePath,
                    maxRetries = 0,
                    connectTimeoutMillis = 1_000,
                    totalTimeoutMillis = 1_000,
                ),
            ).use { download ->
                download.setProgressCallback { snapshot ->
                    if (snapshot.phase.isTerminal()) {
                        terminal.compareAndSet(null, snapshot)
                        latch.countDown()
                    }
                }

                download.start()

                assertTrue(
                    "expected terminal progress callback from native worker thread",
                    latch.await(10, TimeUnit.SECONDS),
                )
                assertEquals(DownloadPhase.FAILED, terminal.get()?.phase)
            }
        } finally {
            target.delete()
            Takanawa.shutdown()
        }
    }

    private fun DownloadPhase.isTerminal(): Boolean =
        this == DownloadPhase.COMPLETED || this == DownloadPhase.FAILED || this == DownloadPhase.CANCELLED
}
