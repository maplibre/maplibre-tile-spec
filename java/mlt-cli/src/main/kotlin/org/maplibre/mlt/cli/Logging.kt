package org.maplibre.mlt.cli

import org.slf4j.LoggerFactory
import org.slf4j.MarkerFactory
import java.text.DecimalFormat
import kotlin.math.log10
import kotlin.math.pow

/** Logger class for free functions */
private class CommonLogger {
    private constructor()
}

internal val logger = LoggerFactory.getLogger(CommonLogger::class.java)
internal val readMarker = MarkerFactory.getMarker("READ")
internal val writeMarker = MarkerFactory.getMarker("WRITE")
internal val cacheMarker = MarkerFactory.getMarker("CACHE")
internal val compressMarker = MarkerFactory.getMarker("COMPRESSION")
internal val colMapMarker = MarkerFactory.getMarker("COLMAP")

private val sizeUnits = arrayOf<String>("B", "kiB", "MiB", "GiB", "TiB", "PiB", "EiB")
private val sizeFormatter = DecimalFormat("#,##0.#")

// org.apache.commons.io.FileUtils.byteCountToDisplaySize does this, but always rounds down to GB
fun formatSize(size: Long): String {
    if (size <= 0) return "0"
    val digitGroups = Math.floor((log10(size.toDouble()) / log10(1024.0)))
    return sizeFormatter.format(size / 1024.0.pow(digitGroups)) + " " + sizeUnits[digitGroups.toInt()]
}

fun formatNanosecDuration(nanos: Double) =
    when (nanos) {
        in 1e9..Double.MAX_VALUE -> String.format("%.2fs", nanos / 1e9)
        in 1e6..1e9 -> String.format("%.2fms", nanos / 1e6)
        in 1e3..1e6 -> String.format("%.2fus", nanos / 1e3)
        in Double.MIN_VALUE..1e3 -> String.format("%.2fns", nanos)
        else -> "0"
    }
