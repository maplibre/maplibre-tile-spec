package org.maplibre.mlt.cli

/**  Simplify optional parallel operation by running tasks in a thread pool if provided, or directly if not. */
interface TaskRunner {
    /** Get the number of threads in use, not including the main thread */
    val threadCount: Int

    /**  Execute the given task either directly or on the given thread pool */
    fun run(task: Runnable?)

    /**  Wait for all tasks to complete.  Assumes shutdown has been called. */
    @Throws(InterruptedException::class)
    fun awaitTermination()

    /**  Stop accepting new tasks */
    fun shutdown()
}
