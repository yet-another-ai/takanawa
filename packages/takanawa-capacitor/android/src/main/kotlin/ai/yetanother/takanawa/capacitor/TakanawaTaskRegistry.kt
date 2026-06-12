package ai.yetanother.takanawa.capacitor

import java.io.Closeable
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

internal class TakanawaTaskRegistry<T : Closeable> {
    private val tasks = ConcurrentHashMap<String, T>()

    fun insert(task: T): String {
        while (true) {
            val taskId = UUID.randomUUID().toString()
            if (tasks.putIfAbsent(taskId, task) == null) {
                return taskId
            }
        }
    }

    fun get(taskId: String): T =
        tasks[taskId] ?: throw IllegalArgumentException("unknown download task: $taskId")

    fun getOrNull(taskId: String): T? = tasks[taskId]

    fun close(taskId: String) {
        tasks.remove(taskId)?.close()
    }

    fun closeAll() {
        val current = tasks.values.toList()
        tasks.clear()
        current.forEach { it.close() }
    }

    fun size(): Int = tasks.size
}
