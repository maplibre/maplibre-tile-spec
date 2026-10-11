package org.maplibre.mlt_ffi

import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface MltLayerBytesLib : Library

internal class MltLayerBytesNative :
    Structure(),
    Structure.ByValue {
    /** The value of the layer's size varint: the tag and the body, without the varint itself.
     */
    @JvmField
    internal var size: FFIUint32 = FFIUint32()

    /** The geometry column, or the v2 geometry section.
     */
    @JvmField
    internal var geometry: FFIUint32 = FFIUint32()

    /** Every column that is neither geometry nor the id.
     */
    @JvmField
    internal var properties: FFIUint32 = FFIUint32()

    /** The id column.
     */
    @JvmField
    internal var ids: FFIUint32 = FFIUint32()

    /** What is left of `size` once every column is taken out.
     */
    @JvmField
    internal var metadata: FFIUint32 = FFIUint32()

    // Define the fields of the struct
    override fun getFieldOrder(): List<String> = listOf("size", "geometry", "properties", "ids", "metadata")
}

internal class OptionMltLayerBytesNative constructor() :
    Structure(),
    Structure.ByValue {
        @JvmField
        internal var value: MltLayerBytesNative = MltLayerBytesNative()

        @JvmField
        internal var isOk: Byte = 0

        // Define the fields of the struct
        override fun getFieldOrder(): List<String> = listOf("value", "isOk")

        internal fun option(): MltLayerBytesNative? {
            if (isOk == 1.toByte()) {
                return value
            } else {
                return null
            }
        }

        constructor(value: MltLayerBytesNative, isOk: Byte) : this() {
            this.value = value
            this.isOk = isOk
        }

        companion object {
            internal fun some(value: MltLayerBytesNative): OptionMltLayerBytesNative = OptionMltLayerBytesNative(value, 1)

            internal fun none(): OptionMltLayerBytesNative = OptionMltLayerBytesNative(MltLayerBytesNative(), 0)
        }
    }

/** The bytes one layer spends, by what they hold.
*/
class MltLayerBytes(
    var size: UInt,
    var geometry: UInt,
    var properties: UInt,
    var ids: UInt,
    var metadata: UInt,
) {
    companion object {
        internal val libClass: Class<MltLayerBytesLib> = MltLayerBytesLib::class.java
        internal val lib: MltLayerBytesLib = Native.load("mlt_ffi", libClass)
        val NATIVESIZE: Long = Native.getNativeSize(MltLayerBytesNative::class.java).toLong()

        internal fun fromNative(nativeStruct: MltLayerBytesNative): MltLayerBytes {
            val size: UInt = nativeStruct.size.toUInt()
            val geometry: UInt = nativeStruct.geometry.toUInt()
            val properties: UInt = nativeStruct.properties.toUInt()
            val ids: UInt = nativeStruct.ids.toUInt()
            val metadata: UInt = nativeStruct.metadata.toUInt()

            return MltLayerBytes(size, geometry, properties, ids, metadata)
        }
    }

    internal fun toNative(): MltLayerBytesNative {
        var native = MltLayerBytesNative()
        native.size = FFIUint32(this.size)
        native.geometry = FFIUint32(this.geometry)
        native.properties = FFIUint32(this.properties)
        native.ids = FFIUint32(this.ids)
        native.metadata = FFIUint32(this.metadata)
        return native
    }
}
