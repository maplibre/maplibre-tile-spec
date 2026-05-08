package org.maplibre.mlt_ffi;
import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

internal interface MltConverterLib: Library {
    fun MltConverter_destroy(handle: Pointer)
    fun MltConverter_mlt_to_mvt(mlt: Slice): ResultPointerInt
    fun MltConverter_mvt_to_mlt(mvt: Slice, options: Pointer): ResultPointerInt
}
/** Stateless FFI entry-points for MLT ↔ MVT conversion.
*/
class MltConverter internal constructor (
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

    private class MltConverterCleaner(val handle: Pointer, val lib: MltConverterLib) : Runnable {
        override fun run() {
            lib.MltConverter_destroy(handle)
        }
    }
    private fun registerCleaner() {
        CLEANER.register(this, MltConverter.MltConverterCleaner(handle, MltConverter.lib));
    }

    companion object {
        internal val libClass: Class<MltConverterLib> = MltConverterLib::class.java
        internal val lib: MltConverterLib = Native.load("mlt_ffi", libClass)
        @JvmStatic
        
        /** Decode MLT bytes into MVT bytes.
        */
        fun mltToMvt(mlt: UByteArray): Result<MltBuffer> {
            val mltSliceMemory = PrimitiveArrayTools.borrow(mlt)
            
            val returnVal = lib.MltConverter_mlt_to_mvt(mltSliceMemory.slice);
            try {
                val nativeOkVal = returnVal.getNativeOk();
                if (nativeOkVal != null) {
                    val selfEdges: List<Any> = listOf()
                    val handle = nativeOkVal 
                    val returnOpaque = MltBuffer(handle, selfEdges, true)
                    return returnOpaque.ok()
                } else {
                    return ConvertErrorError(ConvertError.fromNative(returnVal.getNativeErr()!!)).err()
                }
            } finally {
                mltSliceMemory.close()
            }
        }
        @JvmStatic
        
        /** Encode MVT bytes into MLT bytes using the given encoder options.
        */
        fun mvtToMlt(mvt: UByteArray, options: MltEncoderOptions): Result<MltBuffer> {
            val mvtSliceMemory = PrimitiveArrayTools.borrow(mvt)
            
            val returnVal = lib.MltConverter_mvt_to_mlt(mvtSliceMemory.slice, options.handle);
            try {
                val nativeOkVal = returnVal.getNativeOk();
                if (nativeOkVal != null) {
                    val selfEdges: List<Any> = listOf()
                    val handle = nativeOkVal 
                    val returnOpaque = MltBuffer(handle, selfEdges, true)
                    return returnOpaque.ok()
                } else {
                    return ConvertErrorError(ConvertError.fromNative(returnVal.getNativeErr()!!)).err()
                }
            } finally {
                mvtSliceMemory.close()
            }
        }
    }

}