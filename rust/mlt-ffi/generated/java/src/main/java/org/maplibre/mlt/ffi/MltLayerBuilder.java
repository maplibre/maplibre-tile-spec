package org.maplibre.mlt.ffi;

import static java.lang.foreign.ValueLayout.ADDRESS;
import static java.lang.foreign.ValueLayout.JAVA_BOOLEAN;
import static java.lang.foreign.ValueLayout.JAVA_BYTE;
import static java.lang.foreign.ValueLayout.JAVA_INT_UNALIGNED;
import static java.lang.foreign.ValueLayout.JAVA_LONG;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SegmentAllocator;
import java.nio.charset.StandardCharsets;
import org.maplibre.mlt.ffi.raw.DiplomatI32View;
import org.maplibre.mlt.ffi.raw.MltLayerBuilder_add_points_result;
import org.maplibre.mlt.ffi.raw.OptionU64;
import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/**
 * A layer written one feature at a time, then encoded without going through MVT.
 * <p>
 * Declare every property name with {@link #addProperty} first.
 * A feature starts with {@link #beginFeature}, gets its geometry and property values, and ends when the next one
 * begins or on {@link #encodeInto}.
 * Coordinates are interleaved {@code x, y} pairs in tile units.
 * Values of different kinds under one key merge the way the MVT importer does:
 * integers and floats widen to a double, any other mix becomes text.
 * <p>
 * Not thread-safe, use one instance per thread.
 */
public final class MltLayerBuilder implements AutoCloseable {

  private static final long RESULT_SIZE = 16;
  private static final long IS_OK_OFFSET = 8;

  static {
    NativeLibrary.load();
    if (MltLayerBuilder_add_points_result.layout().byteSize() != RESULT_SIZE) {
      throw new IllegalStateException("Unexpected native result layout");
    }
  }

  private final Arena arena = Arena.ofShared();
  private final SegmentAllocator results = SegmentAllocator.prefixAllocator(arena.allocate(RESULT_SIZE, 8));
  private final MemorySegment view = DiplomatI32View.allocate(arena);
  private final MemorySegment id = OptionU64.allocate(arena);
  private MemorySegment scratch = arena.allocate(1 << 12, 16);
  private MemorySegment handle;

  /** Starts a layer with the given extent. */
  public MltLayerBuilder(String name, int extent) {
    checkExtent(extent);
    MemorySegment result = mlt_ffi_h.MltLayerBuilder_new(results, string(name), extent);
    if (!result.get(JAVA_BOOLEAN, IS_OK_OFFSET)) {
      throw MltException.take(result.get(ADDRESS, 0));
    }
    handle = result.get(ADDRESS, 0);
  }

  private static void checkExtent(int extent) {
    if (extent <= 0) {
      throw new IllegalArgumentException("extent must be positive: " + extent);
    }
  }

  private MemorySegment self() {
    if (handle == null) {
      throw new IllegalStateException("MltLayerBuilder is closed");
    }
    return handle;
  }

  /** Starts another layer, keeping the allocations of the last one. */
  public void reset(String name, int extent) {
    checkExtent(extent);
    check(mlt_ffi_h.MltLayerBuilder_reset(results, self(), string(name), extent));
  }

  /** Declares a property column, returning the key to set it with. */
  public int addProperty(String name) {
    return mlt_ffi_h.MltLayerBuilder_add_property(self(), string(name));
  }

  /** Starts a feature without an id, ending the previous one. */
  public void beginFeature(GeometryType geometry) {
    beginFeature(geometry, null);
  }

  /** Starts a feature with an optional id, ending the previous one. */
  public void beginFeature(GeometryType geometry, Long featureId) {
    if (featureId == null) {
      OptionU64.is_ok(id, false);
    } else {
      OptionU64.ok(id, featureId);
      OptionU64.is_ok(id, true);
    }
    mlt_ffi_h.MltLayerBuilder_begin_feature(self(), geometry.nativeValue(), id);
  }

  /** Adds the points of a point or multi-point feature. */
  public void addPoints(int[] xy) {
    addPoints(xy, 0, xy.length);
  }

  /** Adds the points of a point or multi-point feature from {@code length} ints of {@code xy} at {@code offset}. */
  public void addPoints(int[] xy, int offset, int length) {
    check(mlt_ffi_h.MltLayerBuilder_add_points(results, self(), ints(xy, offset, length)));
  }

  /** Adds a line of a line or multi-line feature. */
  public void addLine(int[] xy) {
    addLine(xy, 0, xy.length);
  }

  /** Adds a line of a line or multi-line feature from {@code length} ints of {@code xy} at {@code offset}. */
  public void addLine(int[] xy, int offset, int length) {
    check(mlt_ffi_h.MltLayerBuilder_add_line(results, self(), ints(xy, offset, length)));
  }

  /** Starts a polygon of a polygon or multi-polygon feature with its exterior ring. */
  public void addExteriorRing(int[] xy) {
    addExteriorRing(xy, 0, xy.length);
  }

  /** Starts a polygon with its exterior ring from {@code length} ints of {@code xy} at {@code offset}. */
  public void addExteriorRing(int[] xy, int offset, int length) {
    check(mlt_ffi_h.MltLayerBuilder_add_exterior_ring(results, self(), ints(xy, offset, length)));
  }

  /** Adds a hole to the polygon the last exterior ring started. */
  public void addHole(int[] xy) {
    addHole(xy, 0, xy.length);
  }

  /** Adds a hole from {@code length} ints of {@code xy} at {@code offset}. */
  public void addHole(int[] xy, int offset, int length) {
    check(mlt_ffi_h.MltLayerBuilder_add_hole(results, self(), ints(xy, offset, length)));
  }

  /** Sets a boolean property of the current feature. */
  public void setBool(int key, boolean value) {
    check(mlt_ffi_h.MltLayerBuilder_set_bool(results, self(), key, value));
  }

  /** Sets an integer property of the current feature. */
  public void setLong(int key, long value) {
    check(mlt_ffi_h.MltLayerBuilder_set_i64(results, self(), key, value));
  }

  /** Sets a 32-bit float property of the current feature. */
  public void setFloat(int key, float value) {
    check(mlt_ffi_h.MltLayerBuilder_set_f32(results, self(), key, value));
  }

  /** Sets a 64-bit float property of the current feature. */
  public void setDouble(int key, double value) {
    check(mlt_ffi_h.MltLayerBuilder_set_f64(results, self(), key, value));
  }

  /** Sets a string property of the current feature. */
  public void setString(int key, String value) {
    check(mlt_ffi_h.MltLayerBuilder_set_str(results, self(), key, string(value)));
  }

  /** Encodes the layer and appends it to {@code out}, so a tile is its layers in one buffer. */
  public void encodeInto(EncoderOptions options, MltBuffer out) {
    check(mlt_ffi_h.MltLayerBuilder_encode_into(results, self(), options.handle(), out.handle()));
  }

  private static void check(MemorySegment result) {
    if (!result.get(JAVA_BOOLEAN, IS_OK_OFFSET)) {
      throw MltException.take(result.get(ADDRESS, 0));
    }
  }

  private void ensureScratch(long size) {
    if (scratch.byteSize() < size) {
      scratch = arena.allocate(Math.max(size, scratch.byteSize() * 2), 16);
    }
  }

  private MemorySegment string(String s) {
    byte[] bytes = s.getBytes(StandardCharsets.UTF_8);
    ensureScratch(bytes.length);
    MemorySegment.copy(bytes, 0, scratch, JAVA_BYTE, 0, bytes.length);
    view.set(ADDRESS, 0, scratch);
    view.set(JAVA_LONG, 8, bytes.length);
    return view;
  }

  private MemorySegment ints(int[] xy, int offset, int length) {
    ensureScratch(4L * length);
    MemorySegment.copy(xy, offset, scratch, JAVA_INT_UNALIGNED, 0, length);
    view.set(ADDRESS, 0, scratch);
    view.set(JAVA_LONG, 8, length);
    return view;
  }

  @Override
  public void close() {
    if (handle != null) {
      mlt_ffi_h.MltLayerBuilder_destroy(handle);
      handle = null;
      arena.close();
    }
  }
}
