package org.maplibre.mlt_ffi

import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface ConvertErrorLib: Library {
}
/** Error type returned by FFI conversion functions.
*/
enum class ConvertError {
    InvalidInput,
    EncodingFailed;

    fun toNative(): Int {
        return this.ordinal
    }


    companion object {
        internal val libClass: Class<ConvertErrorLib> = ConvertErrorLib::class.java
        internal val lib: ConvertErrorLib = Native.load("mlt_ffi", libClass)
        fun fromNative(native: Int): ConvertError {
            return ConvertError.entries[native]
        }

        fun default(): ConvertError {
            return InvalidInput
        }
    }
}
class ConvertErrorError internal constructor(internal val value: ConvertError): Exception("Rust error result for ConvertError") {
    override fun toString(): String {
        return "ConvertError error with value " + value
    }

    fun getValue(): ConvertError = value
}
