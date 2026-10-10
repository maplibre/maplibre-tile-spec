package org.maplibre.mlt_ffi
import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface MltTileStatsLib : Library {
    fun MltTileStats_destroy(handle: Pointer)

    fun MltTileStats_from_bytes(mlt: Slice): ResultPointerPointer

    fun MltTileStats_layer_count(handle: Pointer): FFISizet

    fun MltTileStats_layer_name(
        handle: Pointer,
        i: FFISizet,
        write: Pointer,
    ): Unit

    fun MltTileStats_layer_bytes(
        handle: Pointer,
        i: FFISizet,
    ): OptionMltLayerBytesNative
}

/** The bytes each layer of a tile spends, read without decoding the tile.
*/
class MltTileStats internal constructor(
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

    private class MltTileStatsCleaner(
        val handle: Pointer,
        val lib: MltTileStatsLib,
    ) : Runnable {
        override fun run() {
            lib.MltTileStats_destroy(handle)
        }
    }

    private fun registerCleaner() {
        CLEANER.register(this, MltTileStats.MltTileStatsCleaner(handle, MltTileStats.lib))
    }

    companion object {
        internal val libClass: Class<MltTileStatsLib> = MltTileStatsLib::class.java
        internal val lib: MltTileStatsLib = Native.load("mlt_ffi", libClass)

        /** Parse the layers of an MLT tile, leaving out any with a tag this build does not know.
         */
        @JvmStatic
        fun fromBytes(mlt: UByteArray): Result<MltTileStats> {
            val mltSliceMemory = PrimitiveArrayTools.borrow(mlt)

            val returnVal = lib.MltTileStats_from_bytes(mltSliceMemory.slice)
            try {
                val nativeOkVal = returnVal.getNativeOk()
                if (nativeOkVal != null) {
                    val selfEdges: List<Any> = listOf()
                    val handle = nativeOkVal
                    val returnOpaque = MltTileStats(handle, selfEdges, true)
                    return returnOpaque.ok()
                } else {
                    val selfEdges: List<Any> = listOf()
                    val handle = returnVal.getNativeErr()!!
                    val returnOpaque = ConvertError(handle, selfEdges, true)
                    return returnOpaque.err()
                }
            } finally {
                mltSliceMemory.close()
            }
        }
    }

    /** Number of layers, in tile order.
     */
    fun layerCount(): ULong {
        val returnVal = lib.MltTileStats_layer_count(handle)
        return (returnVal.toULong())
    }

    /** The name of layer `i`, or nothing past the last layer.
     */
    fun layerName(i: ULong): String {
        val write = DW.lib.diplomat_buffer_write_create(0)
        val returnVal = lib.MltTileStats_layer_name(handle, FFISizet(i), write)

        val returnString = DW.writeToString(write)
        return returnString
    }

    /** The bytes of layer `i`, or `None` past the last layer.
     */
    fun layerBytes(i: ULong): MltLayerBytes? {
        val returnVal = lib.MltTileStats_layer_bytes(handle, FFISizet(i))

        val intermediateOption = returnVal.option() ?: return null
        val returnStruct = MltLayerBytes.fromNative(intermediateOption)
        return returnStruct
    }
}
