package ai.yetanother.takanawa

enum class DownloadPhase(val code: Int) {
    CREATED(0),
    RUNNING(1),
    PAUSED(2),
    CANCELLED(3),
    COMPLETED(4),
    FAILED(5),
    PAUSING(6),
    CANCELLING(7),
    ;

    companion object {
        @JvmStatic
        fun fromCode(code: Int): DownloadPhase =
            values().firstOrNull { it.code == code } ?: FAILED
    }
}
