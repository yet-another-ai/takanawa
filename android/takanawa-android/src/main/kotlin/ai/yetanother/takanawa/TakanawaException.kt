package ai.yetanother.takanawa

class TakanawaException(
    val status: TakanawaStatus,
    message: String? = null,
) : RuntimeException(message ?: status.name)
