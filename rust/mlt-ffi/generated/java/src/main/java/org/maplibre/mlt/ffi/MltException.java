package org.maplibre.mlt.ffi;

import static java.lang.foreign.ValueLayout.JAVA_BYTE;

import java.lang.foreign.MemorySegment;
import java.nio.charset.StandardCharsets;
import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/** An error reported by the native library. */
public final class MltException extends RuntimeException {

  /** The stage that failed. */
  public enum Kind {
    /** Input bytes could not be parsed or decoded. */
    INVALID_INPUT,
    /** Encoding failed. */
    ENCODING_FAILED,
    /** A feature handed to a layer builder is malformed. */
    INVALID_FEATURE
  }

  private final Kind kind;

  MltException(Kind kind, String message) {
    super(message);
    this.kind = kind;
  }

  /** Returns the stage that failed. */
  public Kind kind() {
    return kind;
  }

  /** Reads and frees the native {@code ConvertError} behind {@code error}. */
  static MltException take(MemorySegment error) {
    int kind = mlt_ffi_h.ConvertError_kind(error);
    MemorySegment write = mlt_ffi_h.diplomat_buffer_write_create(64);
    try {
      mlt_ffi_h.ConvertError_message(error, write);
      long length = mlt_ffi_h.diplomat_buffer_write_len(write);
      byte[] bytes = mlt_ffi_h.diplomat_buffer_write_get_bytes(write).reinterpret(length).toArray(JAVA_BYTE);
      return new MltException(kindOf(kind), new String(bytes, StandardCharsets.UTF_8));
    } finally {
      mlt_ffi_h.diplomat_buffer_write_destroy(write);
      mlt_ffi_h.ConvertError_destroy(error);
    }
  }

  private static Kind kindOf(int kind) {
    if (kind == mlt_ffi_h.ConvertErrorKind_InvalidInput()) {
      return Kind.INVALID_INPUT;
    } else if (kind == mlt_ffi_h.ConvertErrorKind_EncodingFailed()) {
      return Kind.ENCODING_FAILED;
    }
    return Kind.INVALID_FEATURE;
  }
}
