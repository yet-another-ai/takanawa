package ai.yetanother.takanawa.capacitor

import java.io.Closeable
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class TakanawaTaskRegistryTest {
    @Test
    fun closesAndRemovesTasksById() {
        val registry = TakanawaTaskRegistry<FakeCloseable>()
        val task = FakeCloseable()
        val taskId = registry.insert(task)

        assertEquals(1, registry.size())
        registry.close(taskId)

        assertEquals(0, registry.size())
        assertEquals(1, task.closeCount)
        registry.close(taskId)
        assertEquals(1, task.closeCount)
    }

    @Test
    fun rejectsUnknownTaskIds() {
        val registry = TakanawaTaskRegistry<FakeCloseable>()

        assertThrows(IllegalArgumentException::class.java) {
            registry.get("missing")
        }
    }

    private class FakeCloseable : Closeable {
        var closeCount = 0

        override fun close() {
            closeCount += 1
        }
    }
}
