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
    /** A feature handed to an encoder is malformed. */
    INVALID_FEATURE;

    int nativeValue() {
      return switch (this) {
        case INVALID_INPUT -> mlt_ffi_h.ConvertErrorKind_InvalidInput();
        case ENCODING_FAILED -> mlt_ffi_h.ConvertErrorKind_EncodingFailed();
        case INVALID_FEATURE -> mlt_ffi_h.ConvertErrorKind_InvalidFeature();
      };
    }

    static Kind fromNative(int value) {
      for (Kind kind : values()) {
        if (kind.nativeValue() == value) {
          return kind;
        }
      }
      throw new IllegalStateException("Unknown native error kind " + value);
    }
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

  /** Reads the native {@code ConvertError} behind {@code error} and frees it. */
  static MltException fromNative(MemorySegment error) {
    MemorySegment write = mlt_ffi_h.diplomat_buffer_write_create(64);
    try {
      Kind kind = Kind.fromNative(mlt_ffi_h.ConvertError_kind(error));
      mlt_ffi_h.ConvertError_message(error, write);
      long length = mlt_ffi_h.diplomat_buffer_write_len(write);
      byte[] bytes = mlt_ffi_h.diplomat_buffer_write_get_bytes(write).reinterpret(length).toArray(JAVA_BYTE);
      return new MltException(kind, new String(bytes, StandardCharsets.UTF_8));
    } finally {
      mlt_ffi_h.diplomat_buffer_write_destroy(write);
      mlt_ffi_h.ConvertError_destroy(error);
    }
  }
}
