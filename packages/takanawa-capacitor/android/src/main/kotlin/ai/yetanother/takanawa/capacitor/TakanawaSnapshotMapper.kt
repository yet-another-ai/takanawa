package ai.yetanother.takanawa.capacitor

import ai.yetanother.takanawa.DownloadPhase
import ai.yetanother.takanawa.DownloadSnapshot
import com.getcapacitor.JSObject
import java.util.Locale

internal fun DownloadSnapshot.toJSObject(lastError: String? = null): JSObject {
    val payload = JSObject()
    payload.put("phase", phase.toJSPhase())
    payload.put("contentLen", contentLen.toString())
    payload.put("downloadedBytes", downloadedBytes.toString())
    payload.put("chunkSize", chunkSize.toString())
    payload.put("chunkCount", chunkCount.toString())
    payload.put("completedChunks", completedChunks.toString())
    payload.put("activeIo", activeIo)
    if (!lastError.isNullOrBlank()) {
        payload.put("lastError", lastError)
    }
    return payload
}

internal fun DownloadPhase.toJSPhase(): String = name.lowercase(Locale.US)

internal fun DownloadPhase.isTerminal(): Boolean =
    this == DownloadPhase.COMPLETED || this == DownloadPhase.FAILED || this == DownloadPhase.CANCELLED
