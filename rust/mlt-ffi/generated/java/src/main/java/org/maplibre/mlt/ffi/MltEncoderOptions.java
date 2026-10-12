package org.maplibre.mlt.ffi;

import java.lang.foreign.MemorySegment;
import java.util.EnumMap;
import java.util.Map;
import java.util.Objects;
import java.util.function.BiConsumer;
import org.jspecify.annotations.Nullable;
import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/** Immutable snapshot of the settings collected by {@link MltEncoder.Builder}. */
final class MltEncoderOptions {

  enum Flag {
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
  private final @Nullable MltWireVersion wireVersion;

  MltEncoderOptions(Map<Flag, Boolean> flags, @Nullable MltWireVersion wireVersion) {
    this.flags = new EnumMap<>(flags);
    this.wireVersion = wireVersion;
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
  public boolean equals(@Nullable Object other) {
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
}
