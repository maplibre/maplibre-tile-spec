package org.maplibre.mlt_ffi

import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface ConvertErrorKindLib : Library

/** Which stage of a conversion failed.
*/
enum class ConvertErrorKind {
    InvalidInput,
    EncodingFailed,
    InvalidFeature,
    ;

    fun toNative(): Int = this.ordinal

    companion object {
        internal val libClass: Class<ConvertErrorKindLib> = ConvertErrorKindLib::class.java
        internal val lib: ConvertErrorKindLib = Native.load("mlt_ffi", libClass)

        fun fromNative(native: Int): ConvertErrorKind = ConvertErrorKind.entries[native]

        fun default(): ConvertErrorKind = InvalidInput
    }
}
