package org.maplibre.mlt.cli

import org.slf4j.LoggerFactory
import org.slf4j.MarkerFactory

/** Logger class for free functions */
private class CommonLogger

internal val logger = LoggerFactory.getLogger(CommonLogger::class.java)
internal val readMarker = MarkerFactory.getMarker("READ")
internal val writeMarker = MarkerFactory.getMarker("WRITE")
internal val cacheMarker = MarkerFactory.getMarker("CACHE")
internal val compressMarker = MarkerFactory.getMarker("COMPRESSION")
internal val colMapMarker = MarkerFactory.getMarker("COLMAP")
