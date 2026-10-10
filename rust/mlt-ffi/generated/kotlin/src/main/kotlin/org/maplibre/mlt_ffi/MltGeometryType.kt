package org.maplibre.mlt_ffi

import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface MltGeometryTypeLib : Library

/** The geometry type of one feature.
*/
enum class MltGeometryType {
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
    ;

    fun toNative(): Int = this.ordinal

    companion object {
        internal val libClass: Class<MltGeometryTypeLib> = MltGeometryTypeLib::class.java
        internal val lib: MltGeometryTypeLib = Native.load("mlt_ffi", libClass)

        fun fromNative(native: Int): MltGeometryType = MltGeometryType.entries[native]

        fun default(): MltGeometryType = Point
    }
}
