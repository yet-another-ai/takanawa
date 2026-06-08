package ai.yetanother.takanawa

enum class TakanawaStatus(val code: Int) {
    OK(0),
    BUFFER_TOO_SMALL(1),
    NULL_POINTER(-1),
    ABI_MISMATCH(-2),
    INVALID_CONFIG(-3),
    RUNTIME_NOT_INITIALIZED(-4),
    TARGET_EXISTS(-10),
    PART_BUSY(-11),
    PART_SIZE_MISMATCH(-12),
    PART_CORRUPT(-13),
    REMOTE_CHANGED(-14),
    HTTP_PROTOCOL(-20),
    NETWORK(-21),
    IO(-30),
    HASH_MISMATCH(-40),
    CANCELLED(-50),
    ALREADY_STARTED(-51),
    PANIC(-100),
    INTERNAL(-101),
    ;

    companion object {
        @JvmStatic
        fun fromCode(code: Int): TakanawaStatus =
            values().firstOrNull { it.code == code } ?: INTERNAL
    }
}

internal fun checkStatus(statusCode: Int, handle: Long = 0) {
    val status = TakanawaStatus.fromCode(statusCode)
    if (status == TakanawaStatus.OK) {
        return
    }

    val nativeMessage = if (handle != 0L) {
        NativeBridge.downloadLastError(handle).ifBlank { null }
    } else {
        null
    }
    throw TakanawaException(status, nativeMessage)
}
