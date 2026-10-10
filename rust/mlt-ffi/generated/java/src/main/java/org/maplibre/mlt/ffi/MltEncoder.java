package org.maplibre.mlt.ffi;

import static java.lang.foreign.ValueLayout.JAVA_BYTE;
import static java.lang.foreign.ValueLayout.JAVA_INT_UNALIGNED;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SegmentAllocator;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;
import org.maplibre.mlt.ffi.raw.DiplomatStringView;
import org.maplibre.mlt.ffi.raw.DiplomatU32View;
import org.maplibre.mlt.ffi.raw.DiplomatU8View;
import org.maplibre.mlt.ffi.raw.OptionU64;
import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/**
 * Encodes tiles one layer and one feature at a time, without serializing an MVT tile.
 * <p>
 * A layer starts with {@link #beginLayer} and ends when the next one begins or on {@link #toByteArray}.
 * A feature starts with {@link #beginFeature}, which takes its geometry as MVT commands in tile units.
 * It gets its property values by name, and ends when the next feature or layer begins.
 * Values of different kinds under one key merge the way the MVT importer does:
 * integers and floats widen to a double, any other mix becomes text.
 * <p>
 * One encoder is reused across tiles and keeps its allocations.
 * A layer that fails to encode discards the whole tile.
 * Not thread-safe, so only the thread that created an encoder may use and close it.
 */
public final class MltEncoder implements AutoCloseable {

  static {
    NativeLibrary.load();
  }

  private final Arena arena = Arena.ofConfined();
  private final SegmentAllocator results = SegmentAllocator.prefixAllocator(arena.allocate(Diplomat.RESULT_SIZE, 8));
  private final SegmentAllocator bytesResults = SegmentAllocator.prefixAllocator(arena.allocate(DiplomatU8View.layout()));
  private final MemorySegment view = arena.allocate(DiplomatU32View.layout());
  private final MemorySegment id = OptionU64.allocate(arena);
  private final Map<String, Integer> keys = new HashMap<>();
  private MemorySegment scratch = arena.allocate(1 << 12, 16);
  private MemorySegment options;
  private MemorySegment buffer;
  private MemorySegment layer;
  private boolean layerOpen;

  /** Creates an encoder with the default options. */
  public MltEncoder() {
    this(MltEncoderOptions.defaults());
  }

  /** Creates an encoder that encodes every layer with {@code options}. */
  public MltEncoder(MltEncoderOptions options) {
    this.options = options.newHandle();
    buffer = mlt_ffi_h.MltBuffer_new();
  }

  /**
   * Begins a layer with the given extent, encoding the previous one into the tile.
   *
   * @throws IllegalArgumentException if {@code extent} is not positive
   * @throws IllegalStateException if the encoder is closed
   * @throws MltException if the previous layer fails to encode or {@code name} is empty
   */
  public void beginLayer(String name, int extent) {
    if (extent <= 0) {
      throw new IllegalArgumentException("extent must be positive: " + extent);
    }
    finishLayer();
    if (layer == null) {
      layer = Diplomat.unwrap(mlt_ffi_h.MltLayerBuilder_new(results, utf8View(name), extent));
    } else {
      Diplomat.check(mlt_ffi_h.MltLayerBuilder_reset(results, layer, utf8View(name), extent));
    }
    keys.clear();
    layerOpen = true;
  }

  /**
   * Begins a feature without an id from its MVT geometry commands, ending the previous one.
   * A ring with positive area starts a polygon and any other ring is a hole of the last one.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if the commands do not form a geometry of the given type
   */
  public void beginFeature(MvtGeometryType geometry, int[] commands) {
    OptionU64.is_ok(id, false);
    begin(geometry, commands);
  }

  /**
   * Begins a feature with an id from its MVT geometry commands, ending the previous one.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if the commands do not form a geometry of the given type
   */
  public void beginFeature(MvtGeometryType geometry, int[] commands, long featureId) {
    OptionU64.ok(id, featureId);
    OptionU64.is_ok(id, true);
    begin(geometry, commands);
  }

  private void begin(MvtGeometryType geometry, int[] commands) {
    Diplomat.check(mlt_ffi_h.MltLayerBuilder_begin_mvt_feature(
      results, layer(), geometry.nativeValue(), u32View(commands), id));
  }

  /**
   * Sets a boolean property of the current feature.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if no feature has begun
   */
  public void setBool(String key, boolean value) {
    Diplomat.check(mlt_ffi_h.MltLayerBuilder_set_bool(results, layer(), key(key), value));
  }

  /**
   * Sets an integer property of the current feature.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if no feature has begun
   */
  public void setLong(String key, long value) {
    Diplomat.check(mlt_ffi_h.MltLayerBuilder_set_i64(results, layer(), key(key), value));
  }

  /**
   * Sets a 32-bit float property of the current feature.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if no feature has begun
   */
  public void setFloat(String key, float value) {
    Diplomat.check(mlt_ffi_h.MltLayerBuilder_set_f32(results, layer(), key(key), value));
  }

  /**
   * Sets a 64-bit float property of the current feature.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if no feature has begun
   */
  public void setDouble(String key, double value) {
    Diplomat.check(mlt_ffi_h.MltLayerBuilder_set_f64(results, layer(), key(key), value));
  }

  /**
   * Sets a string property of the current feature.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if no feature has begun
   */
  public void setString(String key, String value) {
    int column = key(key);
    Diplomat.check(mlt_ffi_h.MltLayerBuilder_set_str(results, layer(), column, utf8View(value)));
  }

  /**
   * Encodes the open layer and returns the tile, leaving the encoder empty for the next one.
   *
   * @throws IllegalStateException if the encoder is closed
   * @throws MltException if the open layer fails to encode
   */
  public byte[] toByteArray() {
    finishLayer();
    byte[] tile = Diplomat.toByteArray(mlt_ffi_h.MltBuffer_as_bytes(bytesResults, buffer()));
    mlt_ffi_h.MltBuffer_clear(buffer);
    return tile;
  }

  private void finishLayer() {
    if (layerOpen) {
      layerOpen = false;
      try {
        Diplomat.check(mlt_ffi_h.MltLayerBuilder_encode_into(results, layer, options, buffer()));
      } catch (MltException e) {
        mlt_ffi_h.MltBuffer_clear(buffer);
        throw e;
      }
    }
  }

  private MemorySegment buffer() {
    if (buffer == null) {
      throw new IllegalStateException("MltEncoder is closed");
    }
    return buffer;
  }

  private MemorySegment layer() {
    buffer();
    if (!layerOpen) {
      throw new IllegalStateException("no layer begun");
    }
    return layer;
  }

  private int key(String name) {
    return keys.computeIfAbsent(name, n -> mlt_ffi_h.MltLayerBuilder_add_property(layer(), utf8View(n)));
  }

  private void ensureScratch(long size) {
    if (scratch.byteSize() < size) {
      scratch = arena.allocate(Math.max(size, scratch.byteSize() * 2), 16);
    }
  }

  private MemorySegment utf8View(String s) {
    byte[] bytes = s.getBytes(StandardCharsets.UTF_8);
    ensureScratch(bytes.length);
    MemorySegment.copy(bytes, 0, scratch, JAVA_BYTE, 0, bytes.length);
    DiplomatStringView.data(view, scratch);
    DiplomatStringView.len(view, bytes.length);
    return view;
  }

  private MemorySegment u32View(int[] values) {
    ensureScratch(4L * values.length);
    MemorySegment.copy(values, 0, scratch, JAVA_INT_UNALIGNED, 0, values.length);
    DiplomatU32View.data(view, scratch);
    DiplomatU32View.len(view, values.length);
    return view;
  }

  @Override
  public void close() {
    if (buffer != null) {
      if (layer != null) {
        mlt_ffi_h.MltLayerBuilder_destroy(layer);
        layer = null;
      }
      mlt_ffi_h.MltBuffer_destroy(buffer);
      mlt_ffi_h.MltEncoderOptions_destroy(options);
      buffer = null;
      options = null;
      layerOpen = false;
      arena.close();
    }
  }
}
