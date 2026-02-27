package org.maplibre.mlt.cli

import org.maplibre.mlt.converter.ConversionConfig
import org.maplibre.mlt.converter.mvt.ColumnMappingConfig
import java.net.URI
import java.util.regex.Pattern

data class EncodeConfig(
    val columnMappingConfig: ColumnMappingConfig,
    val conversionConfig: ConversionConfig,
    val tessellateSource: URI?,
    val sortFeaturesPattern: Pattern?,
    val regenIDsPattern: Pattern?,
    val compressionType: String?,
    val minZoom: Int,
    val maxZoom: Int,
    val willOutput: Boolean,
    val willDecode: Boolean,
    val willPrintMLT: Boolean,
    val willPrintMVT: Boolean,
    val compareProp: Boolean,
    val compareGeom: Boolean,
    val willTime: Boolean,
    val dumpStreams: Boolean,
    val taskRunner: TaskRunner,
    val continueOnError: Boolean,
)
