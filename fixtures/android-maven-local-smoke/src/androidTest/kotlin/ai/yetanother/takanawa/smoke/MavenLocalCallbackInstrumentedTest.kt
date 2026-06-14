package ai.yetanother.takanawa.smoke

import ai.yetanother.takanawa.DownloadConfig
import ai.yetanother.takanawa.DownloadPhase
import ai.yetanother.takanawa.DownloadSnapshot
import ai.yetanother.takanawa.Takanawa
import ai.yetanother.takanawa.TakanawaDownload
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.Closeable
import java.io.File
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.net.SocketException
import java.nio.charset.StandardCharsets
import java.util.Locale
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class MavenLocalCallbackInstrumentedTest {
    @Test
    fun publishedAarDeliversProgressAndSpeedCallbacksFromNativeWorkerThreads() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val target = File(context.cacheDir, "takanawa-maven-local-callback.bin")
        val data = ByteArray(128 * 1024) { index -> (index % 251).toByte() }
        val progressLatch = CountDownLatch(1)
        val speedLatch = CountDownLatch(1)
        val terminalLatch = CountDownLatch(1)
        val terminal = AtomicReference<DownloadSnapshot?>()

        RangeHttpServer(data).use { server ->
            Takanawa.init(maxIo = 2)
            try {
                target.delete()
                TakanawaDownload.create(
                    DownloadConfig(
                        url = server.url,
                        targetPath = target.absolutePath,
                        chunkSize = 16 * 1024,
                        parallelism = 2,
                        maxRetries = 0,
                        connectTimeoutMillis = 5_000,
                        readTimeoutMillis = 5_000,
                        totalTimeoutMillis = 10_000,
                    ),
                ).use { download ->
                    download.setProgressCallback { snapshot ->
                        if (snapshot.downloadedBytes > 0 || snapshot.phase.isTerminal()) {
                            progressLatch.countDown()
                        }
                        if (snapshot.phase.isTerminal()) {
                            terminal.compareAndSet(null, snapshot)
                            terminalLatch.countDown()
                        }
                    }
                    download.setSpeedCallback { snapshot ->
                        if (
                            snapshot.receivedBytes > 0 ||
                            snapshot.intervalBytes > 0 ||
                            snapshot.bytesPerSecond > 0.0
                        ) {
                            speedLatch.countDown()
                        }
                    }

                    download.start()

                    assertTrue("expected progress callback from published AAR", progressLatch.await(15, TimeUnit.SECONDS))
                    assertTrue("expected speed callback from published AAR", speedLatch.await(15, TimeUnit.SECONDS))
                    assertTrue("expected terminal callback from published AAR", terminalLatch.await(15, TimeUnit.SECONDS))
                    assertEquals(DownloadPhase.COMPLETED, terminal.get()?.phase)
                    assertArrayEquals(data, target.readBytes())
                }
            } finally {
                target.delete()
                File("${target.absolutePath}.part").delete()
                Takanawa.shutdown()
            }
        }
    }

    private fun DownloadPhase.isTerminal(): Boolean =
        this == DownloadPhase.COMPLETED || this == DownloadPhase.FAILED || this == DownloadPhase.CANCELLED

    private class RangeHttpServer(private val data: ByteArray) : Closeable {
        private val running = AtomicBoolean(true)
        private val server = ServerSocket(0, 50, InetAddress.getByName("127.0.0.1"))
        private val thread = Thread(::serve, "takanawa-smoke-range-server")

        val url: String = "http://127.0.0.1:${server.localPort}/file.bin"

        init {
            thread.isDaemon = true
            thread.start()
        }

        private fun serve() {
            while (running.get()) {
                try {
                    server.accept().use(::handle)
                } catch (error: SocketException) {
                    if (running.get()) {
                        throw error
                    }
                }
            }
        }

        private fun handle(socket: Socket) {
            socket.soTimeout = 5_000
            val input = socket.getInputStream().bufferedReader(StandardCharsets.US_ASCII)
            val requestLine = input.readLine() ?: return
            val headers = mutableMapOf<String, String>()
            while (true) {
                val line = input.readLine() ?: return
                if (line.isEmpty()) {
                    break
                }
                val separator = line.indexOf(':')
                if (separator > 0) {
                    headers[line.substring(0, separator).lowercase(Locale.US)] = line.substring(separator + 1).trim()
                }
            }

            val method = requestLine.substringBefore(' ')
            val range = headers["range"]?.let(::parseRange)
            val output = socket.getOutputStream()
            if (range == null) {
                writeResponse(output, status = "200 OK", start = 0, end = data.lastIndex, method = method)
            } else {
                val (start, requestedEnd) = range
                if (start !in data.indices) {
                    val header = "HTTP/1.1 416 Range Not Satisfiable\r\n" +
                        "Content-Range: bytes */${data.size}\r\n" +
                        "Content-Length: 0\r\n" +
                        "Connection: close\r\n\r\n"
                    output.write(header.toByteArray(StandardCharsets.US_ASCII))
                } else {
                    writeResponse(
                        output = output,
                        status = "206 Partial Content",
                        start = start,
                        end = requestedEnd.coerceAtMost(data.lastIndex),
                        method = method,
                    )
                }
            }
            output.flush()
        }

        private fun writeResponse(
            output: java.io.OutputStream,
            status: String,
            start: Int,
            end: Int,
            method: String,
        ) {
            val length = end - start + 1
            val header = "HTTP/1.1 $status\r\n" +
                "Content-Range: bytes $start-$end/${data.size}\r\n" +
                "Content-Length: $length\r\n" +
                "Accept-Ranges: bytes\r\n" +
                "Connection: close\r\n\r\n"
            output.write(header.toByteArray(StandardCharsets.US_ASCII))
            if (!method.equals("HEAD", ignoreCase = true)) {
                output.write(data, start, length)
            }
        }

        private fun parseRange(value: String): Pair<Int, Int>? {
            val bytes = value.removePrefix("bytes=").split('-', limit = 2)
            if (bytes.size != 2) {
                return null
            }
            val start = bytes[0].toIntOrNull() ?: return null
            val end = bytes[1].toIntOrNull() ?: return null
            return start to end
        }

        override fun close() {
            running.set(false)
            server.close()
            thread.join(1_000)
        }
    }
}
