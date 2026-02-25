package org.maplibre.mlt.cli

class SerialTaskRunner : TaskRunner {
    override val threadCount: Int
        get() = 0

    override fun run(task: Runnable?) {
        if (task != null) {
            task.run()
        }
    }

    @Throws(InterruptedException::class)
    override fun awaitTermination() {
    }

    override fun shutdown() {}
}
