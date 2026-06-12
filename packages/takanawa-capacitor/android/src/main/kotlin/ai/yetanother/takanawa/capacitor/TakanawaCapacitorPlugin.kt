package ai.yetanother.takanawa.capacitor

import ai.yetanother.takanawa.DownloadPhase
import ai.yetanother.takanawa.DownloadSnapshot
import ai.yetanother.takanawa.DownloadSpeedSnapshot
import ai.yetanother.takanawa.Takanawa
import ai.yetanother.takanawa.TakanawaDownload
import android.os.Handler
import android.os.Looper
import android.util.Base64
import com.getcapacitor.JSObject
import com.getcapacitor.Plugin
import com.getcapacitor.PluginCall
import com.getcapacitor.PluginMethod
import com.getcapacitor.annotation.CapacitorPlugin
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicReference

@CapacitorPlugin(name = "TakanawaCapacitor")
class TakanawaCapacitorPlugin : Plugin() {
    private val tasks = TakanawaTaskRegistry<TakanawaDownload>()
    private val executor = Executors.newCachedThreadPool()
    private val mainHandler = Handler(Looper.getMainLooper())

    @PluginMethod
    fun create(call: PluginCall) {
        runAsync(call) {
            val parsed = TakanawaCapacitorOptions.parse(call.data)
            Takanawa.init(parsed.maxIo)
            val download = TakanawaDownload.create(parsed.config)
            val taskId = tasks.insert(download)
            try {
                download.setProgressCallback { snapshot -> emitProgress(taskId, snapshot) }
                download.setSpeedCallback { snapshot -> emitSpeed(taskId, snapshot) }
            } catch (error: Throwable) {
                tasks.close(taskId)
                throw error
            }

            JSObject().also { it.put("taskId", taskId) }
        }
    }

    @PluginMethod
    fun start(call: PluginCall) {
        runAsync(call) {
            tasks.get(requiredTaskId(call)).start()
            null
        }
    }

    @PluginMethod
    fun pause(call: PluginCall) {
        runAsync(call) {
            tasks.get(requiredTaskId(call)).pause()
            null
        }
    }

    @PluginMethod
    fun cancel(call: PluginCall) {
        runAsync(call) {
            tasks.get(requiredTaskId(call)).cancel()
            null
        }
    }

    @PluginMethod
    fun snapshot(call: PluginCall) {
        runAsync(call) {
            val task = tasks.get(requiredTaskId(call))
            val snapshot = task.snapshot()
            JSObject().also {
                it.put("snapshot", snapshot.toJSObject(snapshotLastError(task, snapshot)))
            }
        }
    }

    @PluginMethod
    fun bitmap(call: PluginCall) {
        runAsync(call) {
            val bytes = tasks.get(requiredTaskId(call)).copyBitmap()
            JSObject().also {
                it.put("data", Base64.encodeToString(bytes, Base64.NO_WRAP))
            }
        }
    }

    @PluginMethod
    fun close(call: PluginCall) {
        runAsync(call) {
            tasks.close(requiredTaskId(call))
            null
        }
    }

    @PluginMethod
    fun downloadToCompletion(call: PluginCall) {
        runAsync(call) {
            val parsed = TakanawaCapacitorOptions.parse(call.data)
            Takanawa.init(parsed.maxIo)
            val download = TakanawaDownload.create(parsed.config)
            try {
                val terminal = AtomicReference<DownloadSnapshot>()
                val latch = CountDownLatch(1)
                download.setProgressCallback { snapshot ->
                    if (snapshot.phase.isTerminal()) {
                        terminal.compareAndSet(null, snapshot)
                        latch.countDown()
                    }
                }
                download.start()
                latch.await()

                val snapshot = terminal.get() ?: download.snapshot()
                val lastError = snapshotLastError(download, snapshot)
                when (snapshot.phase) {
                    DownloadPhase.COMPLETED -> JSObject().also {
                        it.put("snapshot", snapshot.toJSObject())
                    }
                    DownloadPhase.FAILED -> throw IllegalStateException(lastError ?: "download failed")
                    DownloadPhase.CANCELLED -> throw IllegalStateException(lastError ?: "download cancelled")
                    else -> throw IllegalStateException("download ended before reaching a terminal phase")
                }
            } finally {
                download.close()
            }
        }
    }

    private fun emitProgress(taskId: String, snapshot: DownloadSnapshot) {
        val task = tasks.getOrNull(taskId)
        val payload = JSObject()
        payload.put("taskId", taskId)
        payload.put("snapshot", snapshot.toJSObject(task?.let { snapshotLastError(it, snapshot) }))
        mainHandler.post {
            notifyListeners("downloadProgress", payload)
        }
    }

    private fun emitSpeed(taskId: String, snapshot: DownloadSpeedSnapshot) {
        val payload = JSObject()
        payload.put("taskId", taskId)
        payload.put("snapshot", snapshot.toJSObject())
        mainHandler.post {
            notifyListeners("downloadSpeed", payload)
        }
    }

    private fun snapshotLastError(task: TakanawaDownload, snapshot: DownloadSnapshot): String? =
        if (snapshot.phase == DownloadPhase.FAILED) {
            task.lastError().ifBlank { null }
        } else {
            null
        }

    private fun requiredTaskId(call: PluginCall): String =
        call.getString("taskId") ?: throw IllegalArgumentException("taskId is required")

    private fun runAsync(call: PluginCall, block: () -> JSObject?) {
        executor.execute {
            try {
                val result = block()
                mainHandler.post {
                    if (result == null) {
                        call.resolve()
                    } else {
                        call.resolve(result)
                    }
                }
            } catch (error: Throwable) {
                mainHandler.post {
                    call.reject(error.message ?: error.javaClass.simpleName)
                }
            }
        }
    }
}
