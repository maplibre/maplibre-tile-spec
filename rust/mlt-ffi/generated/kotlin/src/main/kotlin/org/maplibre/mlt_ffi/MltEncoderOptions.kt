package org.maplibre.mlt_ffi
import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface MltEncoderOptionsLib : Library {
    fun MltEncoderOptions_destroy(handle: Pointer)

    fun MltEncoderOptions_new(): Pointer

    fun MltEncoderOptions_set_tessellate(
        handle: Pointer,
        enabled: Boolean,
    ): Unit

    fun MltEncoderOptions_set_try_spatial_morton_sort(
        handle: Pointer,
        enabled: Boolean,
    ): Unit

    fun MltEncoderOptions_set_try_spatial_hilbert_sort(
        handle: Pointer,
        enabled: Boolean,
    ): Unit

    fun MltEncoderOptions_set_try_id_sort(
        handle: Pointer,
        enabled: Boolean,
    ): Unit

    fun MltEncoderOptions_set_allow_fsst(
        handle: Pointer,
        enabled: Boolean,
    ): Unit

    fun MltEncoderOptions_set_allow_fastpfor(
        handle: Pointer,
        enabled: Boolean,
    ): Unit

    fun MltEncoderOptions_set_allow_shared_dict(
        handle: Pointer,
        enabled: Boolean,
    ): Unit
}

/** Encoder options controlling which optimisations are attempted for
*MVT -> MLT conversion.
*
*Construct with [new](MltEncoderOptions::new) (all optimisations
*enabled except tessellation) and toggle individual flags with the
*setter methods.
*/
class MltEncoderOptions internal constructor(
    internal val handle: Pointer,
    // These ensure that anything that is borrowed is kept alive and not cleaned
    // up by the garbage collector.
    internal val selfEdges: List<Any>,
    internal var owned: Boolean,
) {
    init {
        if (this.owned) {
            this.registerCleaner()
        }
    }

    private class MltEncoderOptionsCleaner(
        val handle: Pointer,
        val lib: MltEncoderOptionsLib,
    ) : Runnable {
        override fun run() {
            lib.MltEncoderOptions_destroy(handle)
        }
    }

    private fun registerCleaner() {
        CLEANER.register(this, MltEncoderOptions.MltEncoderOptionsCleaner(handle, MltEncoderOptions.lib))
    }

    companion object {
        internal val libClass: Class<MltEncoderOptionsLib> = MltEncoderOptionsLib::class.java
        internal val lib: MltEncoderOptionsLib = Native.load("mlt_ffi", libClass)

        /** Create encoder options with the default configuration (all
         *optimisations enabled except tessellation).
         */
        @JvmStatic
        fun new_(): MltEncoderOptions {
            val returnVal = lib.MltEncoderOptions_new()
            val selfEdges: List<Any> = listOf()
            val handle = returnVal
            val returnOpaque = MltEncoderOptions(handle, selfEdges, true)
            return returnOpaque
        }
    }

    /** Generate tessellation data for polygons and multi-polygons.
     */
    fun setTessellate(enabled: Boolean) {
        val returnVal = lib.MltEncoderOptions_set_tessellate(handle, enabled)
    }

    /** Try sorting features by the Z-order (Morton) curve index.
     */
    fun setTrySpatialMortonSort(enabled: Boolean) {
        val returnVal = lib.MltEncoderOptions_set_try_spatial_morton_sort(handle, enabled)
    }

    /** Try sorting features by the Hilbert curve index.
     */
    fun setTrySpatialHilbertSort(enabled: Boolean) {
        val returnVal = lib.MltEncoderOptions_set_try_spatial_hilbert_sort(handle, enabled)
    }

    /** Try sorting features by their feature ID in ascending order.
     */
    fun setTryIdSort(enabled: Boolean) {
        val returnVal = lib.MltEncoderOptions_set_try_id_sort(handle, enabled)
    }

    /** Allow FSST string compression.
     */
    fun setAllowFsst(enabled: Boolean) {
        val returnVal = lib.MltEncoderOptions_set_allow_fsst(handle, enabled)
    }

    /** Allow `FastPFOR` integer compression.
     */
    fun setAllowFastpfor(enabled: Boolean) {
        val returnVal = lib.MltEncoderOptions_set_allow_fastpfor(handle, enabled)
    }

    /** Allow string grouping into shared dictionaries.
     */
    fun setAllowSharedDict(enabled: Boolean) {
        val returnVal = lib.MltEncoderOptions_set_allow_shared_dict(handle, enabled)
    }
}
