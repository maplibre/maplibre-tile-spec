package org.maplibre.mlt;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SegmentAllocator;
import java.lang.foreign.ValueLayout;

/**
 * Stateless entry-points for MLT ↔ MVT conversion.
 *
 * <p>Usage example:
 * <pre>{@code
 * byte[] mvtBytes = ...;
 * try (var opts = new MltEncoderOptions()) {
 *     byte[] mltBytes = MltConverter.mvtToMlt(mvtBytes, opts);
 *     byte[] roundTrip = MltConverter.mltToMvt(mltBytes);
 * }
 * }</pre>
 */
public final class MltConverter {

    private MltConverter() {}

    /**
     * Decode MLT bytes into MVT bytes.
     *
     * @param mlt the MLT-encoded tile
     * @return the MVT-encoded tile
     * @throws MltException if the input cannot be parsed or decoded
     */
    public static byte[] mltToMvt(byte[] mlt) throws MltException {
        try (var arena = Arena.ofConfined()) {
            var inputSeg = arena.allocateFrom(ValueLayout.JAVA_BYTE, mlt);
            var view = arena.allocate(MltFfi.DIPLOMAT_U8_VIEW);
            MltFfi.setU8View(view, inputSeg, mlt.length);

            var result = (MemorySegment) MltFfi.MltConverter_mlt_to_mvt.invokeExact(
                    (SegmentAllocator) arena, view);
            return extractResult(result);
        } catch (MltException e) {
            throw e;
        } catch (Throwable t) {
            throw new AssertionError("FFI call failed", t);
        }
    }

    /**
     * Encode MVT bytes into MLT bytes using the given encoder options.
     *
     * @param mvt the MVT-encoded tile
     * @param options encoder configuration
     * @return the MLT-encoded tile
     * @throws MltException if the encoding fails
     */
    public static byte[] mvtToMlt(byte[] mvt, MltEncoderOptions options) throws MltException {
        try (var arena = Arena.ofConfined()) {
            var inputSeg = arena.allocateFrom(ValueLayout.JAVA_BYTE, mvt);
            var view = arena.allocate(MltFfi.DIPLOMAT_U8_VIEW);
            MltFfi.setU8View(view, inputSeg, mvt.length);

            var result = (MemorySegment) MltFfi.MltConverter_mvt_to_mlt.invokeExact(
                    (SegmentAllocator) arena, view, options.handle);
            return extractResult(result);
        } catch (MltException e) {
            throw e;
        } catch (Throwable t) {
            throw new AssertionError("FFI call failed", t);
        }
    }

    private static byte[] extractResult(MemorySegment result) throws MltException {
        if (!MltFfi.resultIsOk(result)) {
            throw new MltException(ConvertError.fromCode(MltFfi.resultErr(result)));
        }

        var bufferPtr = MltFfi.resultOk(result);

        try {
            long len = (long) MltFfi.MltBuffer_len.invokeExact(bufferPtr);

            try (var arena = Arena.ofConfined()) {
                var u8view = (MemorySegment) MltFfi.MltBuffer_as_bytes.invokeExact(
                        (SegmentAllocator) arena, bufferPtr);
                var dataPtr = MltFfi.u8ViewData(u8view);
                return dataPtr.reinterpret(len).toArray(ValueLayout.JAVA_BYTE);
            }
        } catch (Throwable t) {
            throw new AssertionError("FFI call failed", t);
        } finally {
            try { MltFfi.MltBuffer_destroy.invokeExact(bufferPtr); }
            catch (Throwable t) { throw new AssertionError("FFI call failed", t); }
        }
    }
}
