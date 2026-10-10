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
import java.util.HashMap;
import java.util.Map;
import org.maplibre.mlt.ffi.raw.DiplomatU32View;
import org.maplibre.mlt.ffi.raw.MltLayerBuilder_begin_mvt_feature_result;
import org.maplibre.mlt.ffi.raw.OptionU64;
import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/**
 * A layer written one feature at a time, then encoded without serializing an MVT tile.
 * <p>
 * A feature starts with {@link #beginMvtFeature}, which takes its geometry as MVT commands in tile units.
 * It gets its property values by name, and ends when the next one begins or on {@link #encodeInto}.
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
    if (MltLayerBuilder_begin_mvt_feature_result.layout().byteSize() != RESULT_SIZE) {
      throw new IllegalStateException("Unexpected native result layout");
    }
  }

  private final Arena arena = Arena.ofShared();
  private final SegmentAllocator results = SegmentAllocator.prefixAllocator(arena.allocate(RESULT_SIZE, 8));
  private final MemorySegment view = DiplomatU32View.allocate(arena);
  private final Map<String, Integer> keys = new HashMap<>();
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
    keys.clear();
  }

  /** Starts a feature without an id from its MVT geometry commands, ending the previous one. */
  public void beginMvtFeature(MvtGeometryType geometry, int[] commands) {
    beginMvtFeature(geometry, commands, null);
  }

  /**
   * Starts a feature with an optional id from its MVT geometry commands, ending the previous one.
   * A ring with positive area starts a polygon and any other ring is a hole of the last one.
   */
  public void beginMvtFeature(MvtGeometryType geometry, int[] commands, Long featureId) {
    if (featureId == null) {
      OptionU64.is_ok(id, false);
    } else {
      OptionU64.ok(id, featureId);
      OptionU64.is_ok(id, true);
    }
    check(mlt_ffi_h.MltLayerBuilder_begin_mvt_feature(results, self(), geometry.nativeValue(), ints(commands), id));
  }

  /** Sets a boolean property of the current feature. */
  public void setBool(String key, boolean value) {
    check(mlt_ffi_h.MltLayerBuilder_set_bool(results, self(), key(key), value));
  }

  /** Sets an integer property of the current feature. */
  public void setLong(String key, long value) {
    check(mlt_ffi_h.MltLayerBuilder_set_i64(results, self(), key(key), value));
  }

  /** Sets a 32-bit float property of the current feature. */
  public void setFloat(String key, float value) {
    check(mlt_ffi_h.MltLayerBuilder_set_f32(results, self(), key(key), value));
  }

  /** Sets a 64-bit float property of the current feature. */
  public void setDouble(String key, double value) {
    check(mlt_ffi_h.MltLayerBuilder_set_f64(results, self(), key(key), value));
  }

  /** Sets a string property of the current feature. */
  public void setString(String key, String value) {
    int column = key(key);
    check(mlt_ffi_h.MltLayerBuilder_set_str(results, self(), column, string(value)));
  }

  private int key(String name) {
    Integer key = keys.get(name);
    if (key == null) {
      key = mlt_ffi_h.MltLayerBuilder_add_property(self(), string(name));
      keys.put(name, key);
    }
    return key;
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

  private MemorySegment ints(int[] values) {
    ensureScratch(4L * values.length);
    MemorySegment.copy(values, 0, scratch, JAVA_INT_UNALIGNED, 0, values.length);
    view.set(ADDRESS, 0, scratch);
    view.set(JAVA_LONG, 8, values.length);
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
