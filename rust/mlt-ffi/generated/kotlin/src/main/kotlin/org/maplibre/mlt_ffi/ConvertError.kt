package org.maplibre.mlt_ffi
import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface ConvertErrorLib : Library {
    fun ConvertError_destroy(handle: Pointer)

    fun ConvertError_kind(handle: Pointer): Int

    fun ConvertError_message(
        handle: Pointer,
        write: Pointer,
    ): Unit
}

/** Error returned by FFI conversion functions.
*/
class ConvertError internal constructor(
    internal val handle: Pointer,
    // These ensure that anything that is borrowed is kept alive and not cleaned
    // up by the garbage collector.
    internal val selfEdges: List<Any>,
    internal var owned: Boolean,
) : Exception("Rust error result for ConvertError") {
    init {
        if (this.owned) {
            this.registerCleaner()
        }
    }

    private class ConvertErrorCleaner(
        val handle: Pointer,
        val lib: ConvertErrorLib,
    ) : Runnable {
        override fun run() {
            lib.ConvertError_destroy(handle)
        }
    }

    private fun registerCleaner() {
        CLEANER.register(this, ConvertError.ConvertErrorCleaner(handle, ConvertError.lib))
    }

    companion object {
        internal val libClass: Class<ConvertErrorLib> = ConvertErrorLib::class.java
        internal val lib: ConvertErrorLib = Native.load("mlt_ffi", libClass)
    }

    /** The stage that failed.
     */
    fun kind(): ConvertErrorKind {
        val returnVal = lib.ConvertError_kind(handle)
        return (ConvertErrorKind.fromNative(returnVal))
    }

    /** Human-readable cause.
     */
    fun message(): String {
        val write = DW.lib.diplomat_buffer_write_create(0)
        val returnVal = lib.ConvertError_message(handle, write)

        val returnString = DW.writeToString(write)
        return returnString
    }
}
