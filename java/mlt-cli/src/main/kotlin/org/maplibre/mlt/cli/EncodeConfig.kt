package org.maplibre.mlt.cli

import org.maplibre.mlt.converter.ConversionConfig
import org.maplibre.mlt.converter.mvt.ColumnMappingConfig
import java.net.URI
import java.util.regex.Pattern

@JvmRecord
data class EncodeConfig(
    @JvmField val columnMappingConfig: ColumnMappingConfig,
    @JvmField val conversionConfig: ConversionConfig,
    @JvmField val tessellateSource: URI?,
    @JvmField val sortFeaturesPattern: Pattern?,
    @JvmField val regenIDsPattern: Pattern?,
    @JvmField val compressionType: String?,
    @JvmField val minZoom: Int,
    @JvmField val maxZoom: Int,
    @JvmField val willOutput: Boolean,
    @JvmField val willDecode: Boolean,
    @JvmField val willPrintMLT: Boolean,
    @JvmField val willPrintMVT: Boolean,
    @JvmField val compareProp: Boolean,
    @JvmField val compareGeom: Boolean,
    @JvmField val willTime: Boolean,
    @JvmField val dumpStreams: Boolean,
    @JvmField val taskRunner: TaskRunner,
    @JvmField val continueOnError: Boolean,
) {
    fun asBuilder(): Builder =
        Builder()
            .columnMappings(this.columnMappingConfig)
            .conversionConfig(this.conversionConfig)
            .tessellateSource(this.tessellateSource)
            .sortFeaturesPattern(this.sortFeaturesPattern)
            .regenIDsPattern(this.regenIDsPattern)
            .compressionType(this.compressionType)
            .minZoom(this.minZoom)
            .maxZoom(this.maxZoom)
            .willOutput(this.willOutput)
            .willDecode(this.willDecode)
            .willPrintMLT(this.willPrintMLT)
            .willPrintMVT(this.willPrintMVT)
            .compareProp(this.compareProp)
            .compareGeom(this.compareGeom)
            .willTime(this.willTime)
            .dumpStreams(this.dumpStreams)
            .taskRunner(this.taskRunner)
            .continueOnError(this.continueOnError)

    class Builder {
        private var columnMappingConfig: ColumnMappingConfig? = null
        private var conversionConfig: ConversionConfig? = null
        private var tessellateSource: URI? = null
        private var sortFeaturesPattern: Pattern? = null
        private var regenIDsPattern: Pattern? = null
        private var compressionType: String? = null
        private var minZoom = 0
        private var maxZoom = Int.Companion.MAX_VALUE
        private var willOutput = false
        private var willDecode = false
        private var willPrintMLT = false
        private var willPrintMVT = false
        private var compareProp = false
        private var compareGeom = false
        private var willTime = false
        private var dumpStreams = false
        private var taskRunner: TaskRunner? = null
        private var continueOnError = false

        fun columnMappings(v: ColumnMappingConfig): Builder {
            this.columnMappingConfig = v
            return this
        }

        fun conversionConfig(v: ConversionConfig?): Builder {
            this.conversionConfig = v
            return this
        }

        fun tessellateSource(v: URI?): Builder {
            this.tessellateSource = v
            return this
        }

        fun sortFeaturesPattern(v: Pattern?): Builder {
            this.sortFeaturesPattern = v
            return this
        }

        fun regenIDsPattern(v: Pattern?): Builder {
            this.regenIDsPattern = v
            return this
        }

        fun compressionType(v: String?): Builder {
            this.compressionType = v
            return this
        }

        fun minZoom(v: Int): Builder {
            this.minZoom = v
            return this
        }

        fun maxZoom(v: Int): Builder {
            this.maxZoom = v
            return this
        }

        fun willOutput(v: Boolean): Builder {
            this.willOutput = v
            return this
        }

        fun willDecode(v: Boolean): Builder {
            this.willDecode = v
            return this
        }

        fun willPrintMLT(v: Boolean): Builder {
            this.willPrintMLT = v
            return this
        }

        fun willPrintMVT(v: Boolean): Builder {
            this.willPrintMVT = v
            return this
        }

        fun compareProp(v: Boolean): Builder {
            this.compareProp = v
            return this
        }

        fun compareGeom(v: Boolean): Builder {
            this.compareGeom = v
            return this
        }

        fun willTime(v: Boolean): Builder {
            this.willTime = v
            return this
        }

        fun dumpStreams(v: Boolean): Builder {
            this.dumpStreams = v
            return this
        }

        fun taskRunner(v: TaskRunner): Builder {
            this.taskRunner = v
            return this
        }

        fun continueOnError(v: Boolean): Builder {
            this.continueOnError = v
            return this
        }

        fun build(): EncodeConfig =
            EncodeConfig(
                (if (columnMappingConfig != null) columnMappingConfig else ColumnMappingConfig())!!,
                (
                    if (conversionConfig != null) {
                        conversionConfig
                    } else {
                        org.maplibre.mlt.converter
                            .ConversionConfig()
                    }
                )!!,
                tessellateSource,
                sortFeaturesPattern,
                regenIDsPattern,
                compressionType,
                minZoom,
                maxZoom,
                willOutput,
                willDecode,
                willPrintMLT,
                willPrintMVT,
                compareProp,
                compareGeom,
                willTime,
                dumpStreams,
                (if (taskRunner != null) taskRunner else SerialTaskRunner())!!,
                continueOnError,
            )
    }

    companion object {
        @JvmStatic
        fun builder(): Builder = Builder()
    }
}
