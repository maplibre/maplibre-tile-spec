package org.maplibre.mlt;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;

/**
 * Encoder options controlling which optimisations are attempted for
 * MVT → MLT conversion.
 *
 * <p>Must be used with try-with-resources or explicitly {@link #close() closed}
 * to free the underlying native memory.
 */
public final class MltEncoderOptions implements AutoCloseable {

    private final Arena arena;
    final MemorySegment handle;

    /**
     * Create encoder options with the default configuration (all
     * optimisations enabled except tessellation).
     */
    public MltEncoderOptions() {
        this.arena = Arena.ofConfined();
        try {
            this.handle = (MemorySegment) MltFfi.MltEncoderOptions_new.invokeExact();
        } catch (Throwable t) {
            arena.close();
            throw new AssertionError("FFI call failed", t);
        }
    }

    public MltEncoderOptions setTessellate(boolean enabled) {
        try { MltFfi.MltEncoderOptions_set_tessellate.invokeExact(handle, enabled); }
        catch (Throwable t) { throw new AssertionError("FFI call failed", t); }
        return this;
    }

    public MltEncoderOptions setTrySpatialMortonSort(boolean enabled) {
        try { MltFfi.MltEncoderOptions_set_try_spatial_morton_sort.invokeExact(handle, enabled); }
        catch (Throwable t) { throw new AssertionError("FFI call failed", t); }
        return this;
    }

    public MltEncoderOptions setTrySpatialHilbertSort(boolean enabled) {
        try { MltFfi.MltEncoderOptions_set_try_spatial_hilbert_sort.invokeExact(handle, enabled); }
        catch (Throwable t) { throw new AssertionError("FFI call failed", t); }
        return this;
    }

    public MltEncoderOptions setTryIdSort(boolean enabled) {
        try { MltFfi.MltEncoderOptions_set_try_id_sort.invokeExact(handle, enabled); }
        catch (Throwable t) { throw new AssertionError("FFI call failed", t); }
        return this;
    }

    public MltEncoderOptions setAllowFsst(boolean enabled) {
        try { MltFfi.MltEncoderOptions_set_allow_fsst.invokeExact(handle, enabled); }
        catch (Throwable t) { throw new AssertionError("FFI call failed", t); }
        return this;
    }

    public MltEncoderOptions setAllowFpf(boolean enabled) {
        try { MltFfi.MltEncoderOptions_set_allow_fpf.invokeExact(handle, enabled); }
        catch (Throwable t) { throw new AssertionError("FFI call failed", t); }
        return this;
    }

    public MltEncoderOptions setAllowSharedDict(boolean enabled) {
        try { MltFfi.MltEncoderOptions_set_allow_shared_dict.invokeExact(handle, enabled); }
        catch (Throwable t) { throw new AssertionError("FFI call failed", t); }
        return this;
    }

    @Override
    public void close() {
        try { MltFfi.MltEncoderOptions_destroy.invokeExact(handle); }
        catch (Throwable t) { throw new AssertionError("FFI call failed", t); }
        arena.close();
    }
}
