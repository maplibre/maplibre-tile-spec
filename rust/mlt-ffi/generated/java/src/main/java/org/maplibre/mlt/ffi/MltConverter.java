package org.maplibre.mlt.ffi;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/** Converts whole tiles between MLT and MVT. */
public final class MltConverter {

  static {
    NativeLibrary.load();
  }

  private MltConverter() {}

  /**
   * Decodes MLT bytes into MVT bytes.
   *
   * @throws MltException if {@code mlt} cannot be parsed or decoded
   */
  public static byte[] mltToMvt(byte[] mlt) {
    try (Arena arena = Arena.ofConfined()) {
      return takeBytes(mlt_ffi_h.MltConverter_mlt_to_mvt(arena, Diplomat.u8View(arena, mlt)), arena);
    }
  }

  /**
   * Decodes MLT bytes into MVT bytes with a memory budget of {@code maxBytes} for parsing and decoding.
   *
   * @throws IllegalArgumentException if {@code maxBytes} does not fit an unsigned 32-bit integer
   * @throws MltException if {@code mlt} cannot be parsed or decoded within the budget
   */
  public static byte[] mltToMvt(byte[] mlt, long maxBytes) {
    if (maxBytes < 0 || maxBytes > 0xFFFF_FFFFL) {
      throw new IllegalArgumentException("maxBytes must fit an unsigned 32-bit integer: " + maxBytes);
    }
    try (Arena arena = Arena.ofConfined()) {
      MemorySegment mltView = Diplomat.u8View(arena, mlt);
      return takeBytes(mlt_ffi_h.MltConverter_mlt_to_mvt_with_limit(arena, mltView, (int) maxBytes), arena);
    }
  }

  /**
   * Encodes MVT bytes into MLT bytes using the default encoder options.
   *
   * @throws MltException if {@code mvt} cannot be parsed or encoded
   */
  public static byte[] mvtToMlt(byte[] mvt) {
    return mvtToMlt(mvt, MltEncoderOptions.defaults());
  }

  /**
   * Encodes MVT bytes into MLT bytes using the given encoder options.
   *
   * @throws MltException if {@code mvt} cannot be parsed or encoded
   */
  public static byte[] mvtToMlt(byte[] mvt, MltEncoderOptions options) {
    MemorySegment handle = options.newHandle();
    try (Arena arena = Arena.ofConfined()) {
      return takeBytes(mlt_ffi_h.MltConverter_mvt_to_mlt(arena, Diplomat.u8View(arena, mvt), handle), arena);
    } finally {
      mlt_ffi_h.MltEncoderOptions_destroy(handle);
    }
  }

  private static byte[] takeBytes(MemorySegment result, Arena arena) {
    MemorySegment buffer = Diplomat.unwrap(result);
    try {
      return Diplomat.toByteArray(mlt_ffi_h.MltBuffer_as_bytes(arena, buffer));
    } finally {
      mlt_ffi_h.MltBuffer_destroy(buffer);
    }
  }
}
