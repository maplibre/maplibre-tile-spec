package org.maplibre.mlt.ffi;

import java.lang.foreign.MemorySegment;
import java.util.EnumMap;
import java.util.Map;
import java.util.Objects;
import java.util.function.BiConsumer;
import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/**
 * Immutable encoder options controlling which optimisations are attempted.
 * <p>
 * Starts with FSST, FastPFOR and shared dictionaries enabled, no sorting and no tessellation.
 * Every builder method marked v2 only has no effect on a v1 layer.
 */
public final class MltEncoderOptions {

  static {
    NativeLibrary.load();
  }

  private static final MltEncoderOptions DEFAULTS = builder().build();

  private enum Flag {
    TESSELLATE(mlt_ffi_h::MltEncoderOptions_set_tessellate),
    MORTON_SORT(mlt_ffi_h::MltEncoderOptions_set_attempt_spatial_morton_sort),
    HILBERT_SORT(mlt_ffi_h::MltEncoderOptions_set_attempt_spatial_hilbert_sort),
    ID_SORT(mlt_ffi_h::MltEncoderOptions_set_attempt_id_sort),
    FSST(mlt_ffi_h::MltEncoderOptions_set_allow_fsst),
    FASTPFOR(mlt_ffi_h::MltEncoderOptions_set_allow_fastpfor),
    SHARED_DICT(mlt_ffi_h::MltEncoderOptions_set_allow_shared_dict),
    TRIANGLES_ONLY(mlt_ffi_h::MltEncoderOptions_set_allow_triangles_only),
    DELTA2(mlt_ffi_h::MltEncoderOptions_set_allow_delta2),
    FLOAT_DICT(mlt_ffi_h::MltEncoderOptions_set_allow_float_dict),
    FLOAT_ALP(mlt_ffi_h::MltEncoderOptions_set_allow_float_alp),
    PACKED_DICT_CODES(mlt_ffi_h::MltEncoderOptions_set_allow_packed_dict_codes),
    RANS_VERTICES(mlt_ffi_h::MltEncoderOptions_set_allow_rans_vertices),
    ROW_SHAPES(mlt_ffi_h::MltEncoderOptions_set_allow_row_shapes);

    private final BiConsumer<MemorySegment, Boolean> setter;

    Flag(BiConsumer<MemorySegment, Boolean> setter) {
      this.setter = setter;
    }
  }

  private final Map<Flag, Boolean> flags;
  private final MltWireVersion wireVersion;

  private MltEncoderOptions(Builder builder) {
    flags = new EnumMap<>(builder.flags);
    wireVersion = builder.wireVersion;
  }

  /** Returns the default options. */
  public static MltEncoderOptions defaults() {
    return DEFAULTS;
  }

  /** Starts options from the defaults. */
  public static Builder builder() {
    return new Builder();
  }

  /** Allocates native options, which the caller destroys. */
  MemorySegment newHandle() {
    MemorySegment handle = mlt_ffi_h.MltEncoderOptions_new();
    flags.forEach((flag, enabled) -> flag.setter.accept(handle, enabled));
    if (wireVersion != null) {
      mlt_ffi_h.MltEncoderOptions_set_wire_version(handle, wireVersion.nativeValue());
    }
    return handle;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof MltEncoderOptions options
      && flags.equals(options.flags)
      && wireVersion == options.wireVersion;
  }

  @Override
  public int hashCode() {
    return Objects.hash(flags, wireVersion);
  }

  /** Lists the options that were set, leaving out the ones at their default. */
  @Override
  public String toString() {
    StringBuilder text = new StringBuilder("MltEncoderOptions{");
    if (wireVersion != null) {
      text.append("wireVersion=").append(wireVersion);
    }
    flags.forEach((flag, enabled) -> {
      if (text.charAt(text.length() - 1) != '{') {
        text.append(", ");
      }
      text.append(flag).append('=').append(enabled);
    });
    return text.append('}').toString();
  }

  /** Collects options, leaving every unset one at its default. */
  public static final class Builder {
    private final Map<Flag, Boolean> flags = new EnumMap<>(Flag.class);
    private MltWireVersion wireVersion;

    private Builder() {}

    private Builder set(Flag flag, boolean enabled) {
      flags.put(flag, enabled);
      return this;
    }

    /** Generates tessellation data for polygons and multi-polygons. */
    public Builder tessellate(boolean enabled) {
      return set(Flag.TESSELLATE, enabled);
    }

    /** Tries sorting features by the Z-order (Morton) curve index. */
    public Builder attemptSpatialMortonSort(boolean enabled) {
      return set(Flag.MORTON_SORT, enabled);
    }

    /** Tries sorting features by the Hilbert curve index. */
    public Builder attemptSpatialHilbertSort(boolean enabled) {
      return set(Flag.HILBERT_SORT, enabled);
    }

    /** Tries sorting features by their feature ID in ascending order. */
    public Builder attemptIdSort(boolean enabled) {
      return set(Flag.ID_SORT, enabled);
    }

    /** Allows FSST string compression. */
    public Builder allowFsst(boolean enabled) {
      return set(Flag.FSST, enabled);
    }

    /** Allows FastPFOR integer compression. */
    public Builder allowFastPfor(boolean enabled) {
      return set(Flag.FASTPFOR, enabled);
    }

    /** Allows string grouping into shared dictionaries. */
    public Builder allowSharedDict(boolean enabled) {
      return set(Flag.SHARED_DICT, enabled);
    }

    /** Selects the wire format to encode to. */
    public Builder wireVersion(MltWireVersion version) {
      wireVersion = version;
      return this;
    }

    /** v2 only: lets a tessellated all-polygon layer store its triangles without the outlines. */
    public Builder allowTrianglesOnly(boolean enabled) {
      return set(Flag.TRIANGLES_ONLY, enabled);
    }

    /** v2 only: allows integer and vertex streams to store the deltas of their deltas. */
    public Builder allowDelta2(boolean enabled) {
      return set(Flag.DELTA2, enabled);
    }

    /** v2 only: allows float columns to store one code per value into a dictionary. */
    public Builder allowFloatDict(boolean enabled) {
      return set(Flag.FLOAT_DICT, enabled);
    }

    /** v2 only: allows float columns to store each value as a decimal-scaled integer (ALP). */
    public Builder allowFloatAlp(boolean enabled) {
      return set(Flag.FLOAT_ALP, enabled);
    }

    /** v2 only: allows dictionary code streams to be bit-packed. */
    public Builder allowPackedDictCodes(boolean enabled) {
      return set(Flag.PACKED_DICT_CODES, enabled);
    }

    /** v2 only: allows plain vertex streams to be rANS-coded. */
    public Builder allowRansVertices(boolean enabled) {
      return set(Flag.RANS_VERTICES, enabled);
    }

    /** v2 only: lets a nested struct or map code its row shapes instead of per-field presence. */
    public Builder allowRowShapes(boolean enabled) {
      return set(Flag.ROW_SHAPES, enabled);
    }

    /** Freezes the options, leaving this builder usable. */
    public MltEncoderOptions build() {
      return new MltEncoderOptions(this);
    }
  }
}
