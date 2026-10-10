package org.maplibre.mlt_ffi

import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface MltWireVersionLib : Library

/** The wire format an encoded layer uses.
*/
enum class MltWireVersion {
    V01,
    V02,
    ;

    fun toNative(): Int = this.ordinal

    companion object {
        internal val libClass: Class<MltWireVersionLib> = MltWireVersionLib::class.java
        internal val lib: MltWireVersionLib = Native.load("mlt_ffi", libClass)

        fun fromNative(native: Int): MltWireVersion = MltWireVersion.entries[native]

        fun default(): MltWireVersion = V01
    }
}
