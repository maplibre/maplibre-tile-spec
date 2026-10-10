package org.maplibre.mlt_ffi
import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface MltLayerBuilderLib : Library {
    fun MltLayerBuilder_destroy(handle: Pointer)

    fun MltLayerBuilder_new(
        name: Slice,
        extent: FFIUint32,
    ): ResultPointerPointer

    fun MltLayerBuilder_reset(
        handle: Pointer,
        name: Slice,
        extent: FFIUint32,
    ): ResultUnitPointer

    fun MltLayerBuilder_add_property(
        handle: Pointer,
        name: Slice,
    ): FFIUint32

    fun MltLayerBuilder_begin_feature(
        handle: Pointer,
        geometry: Int,
        id: OptionFFIUint64,
    ): Unit

    fun MltLayerBuilder_add_points(
        handle: Pointer,
        xy: Slice,
    ): ResultUnitPointer

    fun MltLayerBuilder_add_line(
        handle: Pointer,
        xy: Slice,
    ): ResultUnitPointer

    fun MltLayerBuilder_add_exterior_ring(
        handle: Pointer,
        xy: Slice,
    ): ResultUnitPointer

    fun MltLayerBuilder_add_hole(
        handle: Pointer,
        xy: Slice,
    ): ResultUnitPointer

    fun MltLayerBuilder_set_bool(
        handle: Pointer,
        key: FFIUint32,
        value: Boolean,
    ): ResultUnitPointer

    fun MltLayerBuilder_set_i64(
        handle: Pointer,
        key: FFIUint32,
        value: Long,
    ): ResultUnitPointer

    fun MltLayerBuilder_set_f32(
        handle: Pointer,
        key: FFIUint32,
        value: Float,
    ): ResultUnitPointer

    fun MltLayerBuilder_set_f64(
        handle: Pointer,
        key: FFIUint32,
        value: Double,
    ): ResultUnitPointer

    fun MltLayerBuilder_set_str(
        handle: Pointer,
        key: FFIUint32,
        value: Slice,
    ): ResultUnitPointer

    fun MltLayerBuilder_encode_into(
        handle: Pointer,
        options: Pointer,
        out: Pointer,
    ): ResultUnitPointer
}

/** A layer written one feature at a time, then encoded without going through MVT.
*/
class MltLayerBuilder internal constructor(
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

    private class MltLayerBuilderCleaner(
        val handle: Pointer,
        val lib: MltLayerBuilderLib,
    ) : Runnable {
        override fun run() {
            lib.MltLayerBuilder_destroy(handle)
        }
    }

    private fun registerCleaner() {
        CLEANER.register(this, MltLayerBuilder.MltLayerBuilderCleaner(handle, MltLayerBuilder.lib))
    }

    companion object {
        internal val libClass: Class<MltLayerBuilderLib> = MltLayerBuilderLib::class.java
        internal val lib: MltLayerBuilderLib = Native.load("mlt_ffi", libClass)

        /** Start a layer.
         */
        @JvmStatic
        fun new_(
            name: String,
            extent: UInt,
        ): Result<MltLayerBuilder> {
            val nameSliceMemory = PrimitiveArrayTools.borrowUtf8(name)

            val returnVal = lib.MltLayerBuilder_new(nameSliceMemory.slice, FFIUint32(extent))
            try {
                val nativeOkVal = returnVal.getNativeOk()
                if (nativeOkVal != null) {
                    val selfEdges: List<Any> = listOf()
                    val handle = nativeOkVal
                    val returnOpaque = MltLayerBuilder(handle, selfEdges, true)
                    return returnOpaque.ok()
                } else {
                    val selfEdges: List<Any> = listOf()
                    val handle = returnVal.getNativeErr()!!
                    val returnOpaque = ConvertError(handle, selfEdges, true)
                    return returnOpaque.err()
                }
            } finally {
                nameSliceMemory.close()
            }
        }
    }

    /** Start another layer, keeping the allocations of the last one.
     */
    fun reset(
        name: String,
        extent: UInt,
    ): Result<Unit> {
        val nameSliceMemory = PrimitiveArrayTools.borrowUtf8(name)

        val returnVal = lib.MltLayerBuilder_reset(handle, nameSliceMemory.slice, FFIUint32(extent))
        try {
            val nativeOkVal = returnVal.getNativeOk()
            if (nativeOkVal != null) {
                return Unit.ok()
            } else {
                val selfEdges: List<Any> = listOf()
                val handle = returnVal.getNativeErr()!!
                val returnOpaque = ConvertError(handle, selfEdges, true)
                return returnOpaque.err()
            }
        } finally {
            nameSliceMemory.close()
        }
    }

    /** Declare a property column, returning the key to set it with.
     */
    fun addProperty(name: String): UInt {
        val nameSliceMemory = PrimitiveArrayTools.borrowUtf8(name)

        val returnVal = lib.MltLayerBuilder_add_property(handle, nameSliceMemory.slice)
        try {
            return (returnVal.toUInt())
        } finally {
            nameSliceMemory.close()
        }
    }

    /** Start a feature, ending the previous one.
     */
    fun beginFeature(
        geometry: MltGeometryType,
        id: ULong?,
    ) {
        val returnVal =
            lib.MltLayerBuilder_begin_feature(
                handle,
                geometry.toNative(),
                id?.let {
                    OptionFFIUint64.some(FFIUint64(it))
                } ?: OptionFFIUint64.none(),
            )
    }

    /** Add the points of a point or multi-point feature.
     */
    fun addPoints(xy: IntArray): Result<Unit> {
        val xySliceMemory = PrimitiveArrayTools.borrow(xy)

        val returnVal = lib.MltLayerBuilder_add_points(handle, xySliceMemory.slice)
        try {
            val nativeOkVal = returnVal.getNativeOk()
            if (nativeOkVal != null) {
                return Unit.ok()
            } else {
                val selfEdges: List<Any> = listOf()
                val handle = returnVal.getNativeErr()!!
                val returnOpaque = ConvertError(handle, selfEdges, true)
                return returnOpaque.err()
            }
        } finally {
            xySliceMemory.close()
        }
    }

    /** Add a line of a line or multi-line feature.
     */
    fun addLine(xy: IntArray): Result<Unit> {
        val xySliceMemory = PrimitiveArrayTools.borrow(xy)

        val returnVal = lib.MltLayerBuilder_add_line(handle, xySliceMemory.slice)
        try {
            val nativeOkVal = returnVal.getNativeOk()
            if (nativeOkVal != null) {
                return Unit.ok()
            } else {
                val selfEdges: List<Any> = listOf()
                val handle = returnVal.getNativeErr()!!
                val returnOpaque = ConvertError(handle, selfEdges, true)
                return returnOpaque.err()
            }
        } finally {
            xySliceMemory.close()
        }
    }

    /** Start a polygon of a polygon or multi-polygon feature with its exterior ring.
     */
    fun addExteriorRing(xy: IntArray): Result<Unit> {
        val xySliceMemory = PrimitiveArrayTools.borrow(xy)

        val returnVal = lib.MltLayerBuilder_add_exterior_ring(handle, xySliceMemory.slice)
        try {
            val nativeOkVal = returnVal.getNativeOk()
            if (nativeOkVal != null) {
                return Unit.ok()
            } else {
                val selfEdges: List<Any> = listOf()
                val handle = returnVal.getNativeErr()!!
                val returnOpaque = ConvertError(handle, selfEdges, true)
                return returnOpaque.err()
            }
        } finally {
            xySliceMemory.close()
        }
    }

    /** Add a hole to the polygon the last exterior ring started.
     */
    fun addHole(xy: IntArray): Result<Unit> {
        val xySliceMemory = PrimitiveArrayTools.borrow(xy)

        val returnVal = lib.MltLayerBuilder_add_hole(handle, xySliceMemory.slice)
        try {
            val nativeOkVal = returnVal.getNativeOk()
            if (nativeOkVal != null) {
                return Unit.ok()
            } else {
                val selfEdges: List<Any> = listOf()
                val handle = returnVal.getNativeErr()!!
                val returnOpaque = ConvertError(handle, selfEdges, true)
                return returnOpaque.err()
            }
        } finally {
            xySliceMemory.close()
        }
    }

    /** Set a boolean property of the current feature.
     */
    fun setBool(
        key: UInt,
        value: Boolean,
    ): Result<Unit> {
        val returnVal = lib.MltLayerBuilder_set_bool(handle, FFIUint32(key), value)
        val nativeOkVal = returnVal.getNativeOk()
        if (nativeOkVal != null) {
            return Unit.ok()
        } else {
            val selfEdges: List<Any> = listOf()
            val handle = returnVal.getNativeErr()!!
            val returnOpaque = ConvertError(handle, selfEdges, true)
            return returnOpaque.err()
        }
    }

    /** Set an integer property of the current feature.
     */
    fun setI64(
        key: UInt,
        value: Long,
    ): Result<Unit> {
        val returnVal = lib.MltLayerBuilder_set_i64(handle, FFIUint32(key), value)
        val nativeOkVal = returnVal.getNativeOk()
        if (nativeOkVal != null) {
            return Unit.ok()
        } else {
            val selfEdges: List<Any> = listOf()
            val handle = returnVal.getNativeErr()!!
            val returnOpaque = ConvertError(handle, selfEdges, true)
            return returnOpaque.err()
        }
    }

    /** Set a 32-bit float property of the current feature.
     */
    fun setF32(
        key: UInt,
        value: Float,
    ): Result<Unit> {
        val returnVal = lib.MltLayerBuilder_set_f32(handle, FFIUint32(key), value)
        val nativeOkVal = returnVal.getNativeOk()
        if (nativeOkVal != null) {
            return Unit.ok()
        } else {
            val selfEdges: List<Any> = listOf()
            val handle = returnVal.getNativeErr()!!
            val returnOpaque = ConvertError(handle, selfEdges, true)
            return returnOpaque.err()
        }
    }

    /** Set a 64-bit float property of the current feature.
     */
    fun setF64(
        key: UInt,
        value: Double,
    ): Result<Unit> {
        val returnVal = lib.MltLayerBuilder_set_f64(handle, FFIUint32(key), value)
        val nativeOkVal = returnVal.getNativeOk()
        if (nativeOkVal != null) {
            return Unit.ok()
        } else {
            val selfEdges: List<Any> = listOf()
            val handle = returnVal.getNativeErr()!!
            val returnOpaque = ConvertError(handle, selfEdges, true)
            return returnOpaque.err()
        }
    }

    /** Set a string property of the current feature.
     */
    fun setStr(
        key: UInt,
        value: String,
    ): Result<Unit> {
        val valueSliceMemory = PrimitiveArrayTools.borrowUtf8(value)

        val returnVal = lib.MltLayerBuilder_set_str(handle, FFIUint32(key), valueSliceMemory.slice)
        try {
            val nativeOkVal = returnVal.getNativeOk()
            if (nativeOkVal != null) {
                return Unit.ok()
            } else {
                val selfEdges: List<Any> = listOf()
                val handle = returnVal.getNativeErr()!!
                val returnOpaque = ConvertError(handle, selfEdges, true)
                return returnOpaque.err()
            }
        } finally {
            valueSliceMemory.close()
        }
    }

    /** Encode the layer and append it to `out`.
     */
    fun encodeInto(
        options: MltEncoderOptions,
        out: MltBuffer,
    ): Result<Unit> {
        val returnVal =
            lib.MltLayerBuilder_encode_into(
                handle,
                options.handle,
                out.handle, // note this is a mutable reference. Think carefully about using, especially concurrently
            )
        val nativeOkVal = returnVal.getNativeOk()
        if (nativeOkVal != null) {
            return Unit.ok()
        } else {
            val selfEdges: List<Any> = listOf()
            val handle = returnVal.getNativeErr()!!
            val returnOpaque = ConvertError(handle, selfEdges, true)
            return returnOpaque.err()
        }
    }
}
