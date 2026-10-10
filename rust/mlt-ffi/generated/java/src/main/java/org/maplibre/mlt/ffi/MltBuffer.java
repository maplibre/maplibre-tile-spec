package org.maplibre.mlt.ffi;

import static java.lang.foreign.ValueLayout.ADDRESS;
import static java.lang.foreign.ValueLayout.JAVA_BYTE;
import static java.lang.foreign.ValueLayout.JAVA_LONG;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SegmentAllocator;
import org.maplibre.mlt.ffi.raw.DiplomatU8View;
import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/**
 * A native byte buffer that collects the encoded layers of one tile.
 * Not thread-safe, use one instance per thread.
 */
public final class MltBuffer implements AutoCloseable {

  static {
    NativeLibrary.load();
  }

  private final Arena arena = Arena.ofShared();
  private final SegmentAllocator viewAllocator = SegmentAllocator.prefixAllocator(arena.allocate(DiplomatU8View.layout()));
  private MemorySegment handle = mlt_ffi_h.MltBuffer_new();

  /** Creates an empty buffer. */
  public MltBuffer() {}

  MemorySegment handle() {
    if (handle == null) {
      throw new IllegalStateException("MltBuffer is closed");
    }
    return handle;
  }

  /** Empties the buffer, keeping its allocation for the next tile. */
  public void clear() {
    mlt_ffi_h.MltBuffer_clear(handle());
  }

  /** Returns the number of bytes in the buffer. */
  public long length() {
    return mlt_ffi_h.MltBuffer_len(handle());
  }

  /** Returns a view of the contents that stays valid until the buffer is next changed or closed. */
  public MemorySegment bytes() {
    MemorySegment view = mlt_ffi_h.MltBuffer_as_bytes(viewAllocator, handle());
    return view.get(ADDRESS, 0).reinterpret(view.get(JAVA_LONG, 8));
  }

  /** Returns a copy of the contents. */
  public byte[] toByteArray() {
    return bytes().toArray(JAVA_BYTE);
  }

  @Override
  public void close() {
    if (handle != null) {
      mlt_ffi_h.MltBuffer_destroy(handle);
      handle = null;
      arena.close();
    }
  }
}
