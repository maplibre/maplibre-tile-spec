package org.maplibre.mlt_ffi

import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface MltMvtGeometryTypeLib : Library

/** The geometry type of an MVT feature, which also covers its multi variant.
*/
enum class MltMvtGeometryType {
    Point,
    LineString,
    Polygon,
    ;

    fun toNative(): Int = this.ordinal

    companion object {
        internal val libClass: Class<MltMvtGeometryTypeLib> = MltMvtGeometryTypeLib::class.java
        internal val lib: MltMvtGeometryTypeLib = Native.load("mlt_ffi", libClass)

        fun fromNative(native: Int): MltMvtGeometryType = MltMvtGeometryType.entries[native]

        fun default(): MltMvtGeometryType = Point
    }
}
