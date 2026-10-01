package org.maplibre.mlt.cli

import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

/**  Simplify optional parallel operation by running tasks in a thread pool if provided, or directly if not. */
interface TaskRunner {
    /** Get the number of threads in use, not including the main thread */
    val threadCount: Int

    /**  Execute the given task either directly or on the given thread pool */
    fun run(task: Runnable?)

    /**  Wait for all tasks to complete.  Assumes shutdown has been called. */
    fun awaitTermination()

    /**  Stop accepting new tasks */
    fun shutdown()
}

fun createBoundedNonRejectingTaskRunner(threadCount: Int): TaskRunner {
    if (threadCount < 2) {
        return SerialTaskRunner()
    }
    // Because the main thread is also used for running tasks when the pool is saturated,
    // we count it as one of the threads and so reduce the pool size by one.
    val poolSize = threadCount - 1
    // Threads are expected to be used continuously, and so don't time out.
    val threadTimeout = Long.MAX_VALUE
    // Create a thread pool with a bounded task queue that will not reject tasks when
    // it's full.  Tasks beyond the limit will run on the calling thread, preventing
    // OOM from too many tasks while allowing for parallelism when the pool is available.
    val taskQueue = LinkedBlockingQueue<Runnable>(threadQueueSize * poolSize)
    val rejectHandler = ThreadPoolExecutor.CallerRunsPolicy()
    return ThreadPoolTaskRunner(
        ThreadPoolExecutor(
            poolSize,
            poolSize,
            threadTimeout,
            TimeUnit.SECONDS,
            taskQueue,
            rejectHandler,
        ),
    )
}
