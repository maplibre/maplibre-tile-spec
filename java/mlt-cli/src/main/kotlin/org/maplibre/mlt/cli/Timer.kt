package org.maplibre.mlt.cli

import org.slf4j.Logger
import org.slf4j.LoggerFactory

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
        val elapsedTime = (endTime - startTime) / 1_000_000 // divide by 1000000 to get milliseconds
        logger.info("Time elapsed for {}: {} ms", message, elapsedTime)
    }

    private val logger: Logger = LoggerFactory.getLogger(Timer::class.java)
}
