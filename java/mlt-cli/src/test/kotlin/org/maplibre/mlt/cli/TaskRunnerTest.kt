package org.maplibre.mlt.cli

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertDoesNotThrow
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

class TaskRunnerTest {
    @Test
    fun `serial runner executes task immediately when thread count is less than two`() {
        val ran = AtomicBoolean(false)
        val runner = createBoundedNonRejectingTaskRunner(1)
        assertTrue(runner is SerialTaskRunner)

        runner.run(Runnable { ran.set(true) })

        assertTrue(ran.get())
        assertEquals(0, runner.threadCount)
    }

    @Test
    fun `thread pool runner reports pool size excluding the main thread`() {
        val runner = createBoundedNonRejectingTaskRunner(4)
        try {
            assertEquals(3, runner.threadCount)
        } finally {
            runner.shutdown()
            runner.awaitTermination()
        }
    }

    // Ensure that tasks run in parallel
    @Test
    fun `thread pool runner executes submitted tasks`() {
        val threadCount = 3
        val runner = createBoundedNonRejectingTaskRunner(threadCount)
        val startLatch = CountDownLatch(1)
        val stopLatch = CountDownLatch(threadCount - 1)
        val mainThread = Thread.currentThread().threadId()
        try {
            repeat(threadCount - 1) {
                runner.run({
                    assertTrue(Thread.currentThread().threadId() != mainThread)
                    assertDoesNotThrow { startLatch.await(1, TimeUnit.SECONDS) }
                    stopLatch.countDown()
                })
            }
            // allow threads to start
            startLatch.countDown()
            // ensure that all tasks complete
            assertTrue(stopLatch.await(1, TimeUnit.SECONDS))
        } finally {
            runner.shutdown()
            runner.awaitTermination()
        }
    }

    // Add more tasks than threads to ensure that the pool doesn't reject tasks
    // when saturated.
    // Ideally we would fill the pool before allowing tasks to complete so it's
    // not sensitive to timing, but the main thread will execute a task at saturation
    // resulting in a deadlock.
    @Test
    fun `thread pool runner doesn't reject tasks`() {
        val threadCount = 3
        val taskCount = threadCount * 10
        val runner = createBoundedNonRejectingTaskRunner(threadCount)
        val startLatch = CountDownLatch(1)
        val stopLatch = CountDownLatch(taskCount)
        val mainThread = Thread.currentThread().threadId()
        try {
            repeat(taskCount) {
                runner.run({
                    if (Thread.currentThread().threadId() == mainThread) {
                        // don't wait, we would deadlock
                    } else {
                        assertDoesNotThrow { startLatch.await(1, TimeUnit.SECONDS) }
                    }
                    stopLatch.countDown()
                })
            }
            // allow threads to start
            startLatch.countDown()
            // ensure that all tasks complete
            assertTrue(stopLatch.await(1, TimeUnit.SECONDS))
        } finally {
            runner.shutdown()
            runner.awaitTermination()
        }
    }
}
