package org.maplibre.mlt.cli

import me.lemire.integercompression.IntegerCODEC
import org.maplibre.mlt.compare.CompareHelper.CompareMode
import org.maplibre.mlt.converter.ColumnMappingConfig
import org.maplibre.mlt.converter.ConversionConfig
import org.maplibre.mlt.converter.encodings.EncodingUtils
import java.net.URI
import java.util.concurrent.ConcurrentHashMap
import java.util.regex.Pattern

class FastPforCodecCache {
    private val codecs = ConcurrentHashMap<Long, IntegerCODEC>()

    fun forCurrentThread(): IntegerCODEC =
        codecs.computeIfAbsent(Thread.currentThread().threadId()) {
            EncodingUtils.createFastPforCodec()
        }
}

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
    val taskRunner: TaskRunner,
    val continueOnError: Boolean,
    val logCacheStats: Boolean,
    val fastPforCodecCache: FastPforCodecCache = FastPforCodecCache(),
) {
    val compareMode get() =
        if (compareGeom && compareProp) {
            CompareMode.All
        } else if (compareGeom) {
            CompareMode.Geometry
        } else if (compareProp) {
            CompareMode.Properties
        } else {
            CompareMode.None
        }
}
