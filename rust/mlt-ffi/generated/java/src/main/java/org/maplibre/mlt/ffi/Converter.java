package org.maplibre.mlt.ffi;

import static java.lang.foreign.ValueLayout.ADDRESS;
import static java.lang.foreign.ValueLayout.JAVA_BOOLEAN;
import static java.lang.foreign.ValueLayout.JAVA_BYTE;
import static java.lang.foreign.ValueLayout.JAVA_LONG;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import org.maplibre.mlt.ffi.raw.DiplomatU8View;
import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/** Converts whole tiles between MLT and MVT. */
public final class Converter {

  static {
    NativeLibrary.load();
  }

  private static final long IS_OK_OFFSET = 8;

  private Converter() {}

  /** Decodes MLT bytes into MVT bytes. */
  public static byte[] mltToMvt(byte[] mlt) {
    try (Arena arena = Arena.ofConfined()) {
      return takeBytes(mlt_ffi_h.MltConverter_mlt_to_mvt(arena, view(arena, mlt)), arena);
    }
  }

  /** Decodes MLT bytes into MVT bytes with a memory budget of {@code maxBytes} for parsing and decoding. */
  public static byte[] mltToMvt(byte[] mlt, long maxBytes) {
    if (maxBytes < 0 || maxBytes > 0xFFFF_FFFFL) {
      throw new IllegalArgumentException("maxBytes must fit an unsigned 32-bit integer: " + maxBytes);
    }
    try (Arena arena = Arena.ofConfined()) {
      return takeBytes(mlt_ffi_h.MltConverter_mlt_to_mvt_with_limit(arena, view(arena, mlt), (int) maxBytes), arena);
    }
  }

  /** Encodes MVT bytes into MLT bytes using the given encoder options. */
  public static byte[] mvtToMlt(byte[] mvt, EncoderOptions options) {
    try (Arena arena = Arena.ofConfined()) {
      return takeBytes(mlt_ffi_h.MltConverter_mvt_to_mlt(arena, view(arena, mvt), options.handle()), arena);
    }
  }

  private static MemorySegment view(Arena arena, byte[] bytes) {
    MemorySegment data = arena.allocate(Math.max(bytes.length, 1));
    MemorySegment.copy(bytes, 0, data, JAVA_BYTE, 0, bytes.length);
    MemorySegment view = DiplomatU8View.allocate(arena);
    DiplomatU8View.data(view, data);
    DiplomatU8View.len(view, bytes.length);
    return view;
  }

  private static byte[] takeBytes(MemorySegment result, Arena arena) {
    if (!result.get(JAVA_BOOLEAN, IS_OK_OFFSET)) {
      throw MltException.take(result.get(ADDRESS, 0));
    }
    MemorySegment buffer = result.get(ADDRESS, 0);
    try {
      MemorySegment view = mlt_ffi_h.MltBuffer_as_bytes(arena, buffer);
      return view.get(ADDRESS, 0).reinterpret(view.get(JAVA_LONG, 8)).toArray(JAVA_BYTE);
    } finally {
      mlt_ffi_h.MltBuffer_destroy(buffer);
    }
  }
}
