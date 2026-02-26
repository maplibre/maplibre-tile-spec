package org.maplibre.mlt.cli

import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

class ThreadPoolTaskRunner(
    private val threadPool: ThreadPoolExecutor,
) : TaskRunner {
    override val threadCount: Int
        get() = threadPool.maximumPoolSize

    override fun run(task: Runnable?) {
        if (task != null) {
            threadPool.execute(task)
        }
    }

    @Throws(InterruptedException::class)
    override fun awaitTermination() {
        threadPool.awaitTermination(Long.MAX_VALUE, TimeUnit.NANOSECONDS)
    }

    override fun shutdown() {
        threadPool.shutdown()
    }
}
