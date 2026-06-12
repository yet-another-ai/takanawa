package ai.yetanother.takanawa.capacitor

import java.io.Closeable
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

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

        assertFailsWith<IllegalArgumentException> {
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
