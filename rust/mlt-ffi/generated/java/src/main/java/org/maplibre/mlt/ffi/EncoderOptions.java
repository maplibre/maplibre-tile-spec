package org.maplibre.mlt.ffi;

import java.lang.foreign.MemorySegment;
import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/**
 * Encoder options controlling which optimisations are attempted.
 * <p>
 * Starts with FSST, FastPFOR and shared dictionaries enabled, no sorting and no tessellation.
 * Every setter marked v2 only has no effect on a v1 layer.
 * Not thread-safe, use one instance per thread.
 */
public final class EncoderOptions implements AutoCloseable {

  static {
    NativeLibrary.load();
  }

  private MemorySegment handle = mlt_ffi_h.MltEncoderOptions_new();

  /** Creates options with the default configuration. */
  public EncoderOptions() {}

  MemorySegment handle() {
    if (handle == null) {
      throw new IllegalStateException("EncoderOptions is closed");
    }
    return handle;
  }

  /** Generates tessellation data for polygons and multi-polygons. */
  public EncoderOptions tessellate(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_tessellate(handle(), enabled);
    return this;
  }

  /** Tries sorting features by the Z-order (Morton) curve index. */
  public EncoderOptions attemptSpatialMortonSort(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_attempt_spatial_morton_sort(handle(), enabled);
    return this;
  }

  /** Tries sorting features by the Hilbert curve index. */
  public EncoderOptions attemptSpatialHilbertSort(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_attempt_spatial_hilbert_sort(handle(), enabled);
    return this;
  }

  /** Tries sorting features by their feature ID in ascending order. */
  public EncoderOptions attemptIdSort(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_attempt_id_sort(handle(), enabled);
    return this;
  }

  /** Allows FSST string compression. */
  public EncoderOptions allowFsst(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_allow_fsst(handle(), enabled);
    return this;
  }

  /** Allows FastPFOR integer compression. */
  public EncoderOptions allowFastPfor(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_allow_fastpfor(handle(), enabled);
    return this;
  }

  /** Allows string grouping into shared dictionaries. */
  public EncoderOptions allowSharedDict(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_allow_shared_dict(handle(), enabled);
    return this;
  }

  /** Selects the wire format to encode to. */
  public EncoderOptions wireVersion(WireVersion version) {
    mlt_ffi_h.MltEncoderOptions_set_wire_version(handle(), version.nativeValue());
    return this;
  }

  /** v2 only: lets a tessellated all-polygon layer store its triangles without the outlines. */
  public EncoderOptions allowTrianglesOnly(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_allow_triangles_only(handle(), enabled);
    return this;
  }

  /** v2 only: allows integer and vertex streams to store the deltas of their deltas. */
  public EncoderOptions allowDelta2(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_allow_delta2(handle(), enabled);
    return this;
  }

  /** v2 only: allows float columns to store one code per value into a dictionary. */
  public EncoderOptions allowFloatDict(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_allow_float_dict(handle(), enabled);
    return this;
  }

  /** v2 only: allows float columns to store each value as a decimal-scaled integer (ALP). */
  public EncoderOptions allowFloatAlp(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_allow_float_alp(handle(), enabled);
    return this;
  }

  /** v2 only: allows dictionary code streams to be bit-packed. */
  public EncoderOptions allowPackedDictCodes(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_allow_packed_dict_codes(handle(), enabled);
    return this;
  }

  /** v2 only: allows plain vertex streams to be rANS-coded. */
  public EncoderOptions allowRansVertices(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_allow_rans_vertices(handle(), enabled);
    return this;
  }

  /** v2 only: lets a nested struct or map code its row shapes instead of per-field presence. */
  public EncoderOptions allowRowShapes(boolean enabled) {
    mlt_ffi_h.MltEncoderOptions_set_allow_row_shapes(handle(), enabled);
    return this;
  }

  @Override
  public void close() {
    if (handle != null) {
      mlt_ffi_h.MltEncoderOptions_destroy(handle);
      handle = null;
    }
  }
}
