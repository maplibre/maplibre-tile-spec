package org.maplibre.mlt_ffi;
import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface MltBufferLib: Library {
    fun MltBuffer_destroy(handle: Pointer)
    fun MltBuffer_as_bytes(handle: Pointer): Slice
    fun MltBuffer_len(handle: Pointer): FFISizet
}
/** Owned byte buffer returned from conversion functions.
*
*The caller borrows the contents via [as_bytes](MltBuffer::as_bytes)
*and the buffer is freed when the handle is dropped.
*/
class MltBuffer internal constructor (
    internal val handle: Pointer,
    // These ensure that anything that is borrowed is kept alive and not cleaned
    // up by the garbage collector.
    internal val selfEdges: List<Any>,
    internal var owned: Boolean,
)  {

    init {
        if (this.owned) {
            this.registerCleaner()
        }
    }

    private class MltBufferCleaner(val handle: Pointer, val lib: MltBufferLib) : Runnable {
        override fun run() {
            lib.MltBuffer_destroy(handle)
        }
    }
    private fun registerCleaner() {
        CLEANER.register(this, MltBuffer.MltBufferCleaner(handle, MltBuffer.lib));
    }

    companion object {
        internal val libClass: Class<MltBufferLib> = MltBufferLib::class.java
        internal val lib: MltBufferLib = Native.load("mlt_ffi", libClass)
    }
    
    /** Borrow the contents as a byte slice.
    */
    fun asBytes(): UByteArray {
        // This lifetime edge depends on lifetimes: 'a
        val aEdges: MutableList<Any> = mutableListOf(this);
        
        val returnVal = lib.MltBuffer_as_bytes(handle);
            return PrimitiveArrayTools.getUByteArray(returnVal)
    }
    
    /** Number of bytes in the buffer.
    */
    fun len(): ULong {
        
        val returnVal = lib.MltBuffer_len(handle);
        return (returnVal.toULong())
    }

}