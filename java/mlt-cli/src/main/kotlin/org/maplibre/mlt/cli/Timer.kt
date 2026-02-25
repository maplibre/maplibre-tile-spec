package org.maplibre.mlt.cli

class Timer {
    private var startTime: Long

    init {
        startTime = System.nanoTime()
    }

    fun restart() {
        startTime = System.nanoTime()
    }

    fun stop(message: String?) {
        val endTime = System.nanoTime()
        val elapsedTime = (endTime - startTime) / 1000000 // divide by 1000000 to get milliseconds
        println("Time elapsed for " + message + ": " + elapsedTime + " milliseconds")
    }
}
