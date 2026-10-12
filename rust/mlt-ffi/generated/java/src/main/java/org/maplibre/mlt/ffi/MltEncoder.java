package org.maplibre.mlt.ffi;

import static java.lang.foreign.ValueLayout.JAVA_BYTE;
import static java.lang.foreign.ValueLayout.JAVA_INT_UNALIGNED;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SegmentAllocator;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.EnumMap;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.jspecify.annotations.Nullable;
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
 * After {@link #toByteArray}, {@link #lastLayerSizes} holds the size of each layer of the tile it returned.
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
  private List<MltLayerSize> layerSizes = new ArrayList<>();
  private List<MltLayerSize> lastLayerSizes = List.of();
  private MemorySegment scratch = arena.allocate(1 << 12, 16);
  private @Nullable MemorySegment options;
  private @Nullable MemorySegment buffer;
  private @Nullable MemorySegment layer;
  private @Nullable String layerName;
  private boolean layerOpen;

  /** Creates an encoder with the default options. */
  public MltEncoder() {
    this(builder().options());
  }

  /** Starts encoder options from the defaults. */
  public static Builder builder() {
    return new Builder();
  }

  private MltEncoder(MltEncoderOptions options) {
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
    layerName = name;
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
  private void setBool(String key, boolean value) {
    Diplomat.check(mlt_ffi_h.MltLayerBuilder_set_bool(results, layer(), key(key), value));
  }

  /**
   * Sets an integer property of the current feature.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if no feature has begun
   */
  private void setLong(String key, long value) {
    Diplomat.check(mlt_ffi_h.MltLayerBuilder_set_i64(results, layer(), key(key), value));
  }

  /**
   * Sets a 32-bit float property of the current feature.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if no feature has begun
   */
  private void setFloat(String key, float value) {
    Diplomat.check(mlt_ffi_h.MltLayerBuilder_set_f32(results, layer(), key(key), value));
  }

  /**
   * Sets a 64-bit float property of the current feature.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if no feature has begun
   */
  private void setDouble(String key, double value) {
    Diplomat.check(mlt_ffi_h.MltLayerBuilder_set_f64(results, layer(), key(key), value));
  }

  /**
   * Sets a string property of the current feature.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if no feature has begun
   */
  private void setString(String key, String value) {
    int column = key(key);
    Diplomat.check(mlt_ffi_h.MltLayerBuilder_set_str(results, layer(), column, utf8View(value)));
  }

  /**
   * Sets a property of the current feature by the runtime type of its value.
   * <p>
   * Booleans, floats and doubles keep their kind, any other number is stored as a long, and everything else as its string form.
   * A null value is skipped.
   *
   * @throws IllegalStateException if no layer has begun or the encoder is closed
   * @throws MltException if no feature has begun
   */
  public void setProperty(String key, @Nullable Object value) {
    switch (value) {
      case null -> { }
      case Boolean b -> setBool(key, b);
      case Float f -> setFloat(key, f);
      case Double d -> setDouble(key, d);
      case Number n -> setLong(key, n.longValue());
      case Object o -> setString(key, o.toString());
    }
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
    lastLayerSizes = List.copyOf(layerSizes);
    layerSizes.clear();
    return tile;
  }

  /**
   * Returns the size of each layer of the tile {@link #toByteArray} last returned, in {@link #beginLayer} order.
   * A layer without features is left out, as the tile does not hold it.
   * Empty after a tile that failed to encode.
   */
  public List<MltLayerSize> lastLayerSizes() {
    return lastLayerSizes;
  }

  private void finishLayer() {
    if (layerOpen) {
      layerOpen = false;
      long start = mlt_ffi_h.MltBuffer_len(buffer());
      try {
        Diplomat.check(mlt_ffi_h.MltLayerBuilder_encode_into(results, layer, options, buffer));
      } catch (MltException e) {
        mlt_ffi_h.MltBuffer_clear(buffer);
        layerSizes.clear();
        lastLayerSizes = List.of();
        throw e;
      }
      // A layer without features encodes to nothing, so the tile does not hold it.
      if (mlt_ffi_h.MltBuffer_len(buffer) > start) {
        layerSizes.add(new MltLayerSize(layerName, sizeVarint(start)));
      }
    }
  }

  /** Reads the size varint that starts the layer at {@code offset} in the buffer. */
  private int sizeVarint(long offset) {
    MemorySegment view = mlt_ffi_h.MltBuffer_as_bytes(bytesResults, buffer);
    MemorySegment bytes = DiplomatU8View.data(view).reinterpret(DiplomatU8View.len(view));
    int value = 0;
    for (int shift = 0; ; shift += 7) {
      byte b = bytes.get(JAVA_BYTE, offset++);
      value |= (b & 0x7F) << shift;
      if (b >= 0) {
        return value;
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

  /**
   * Collects encoder options, leaving every unset one at its default.
   * <p>
   * Starts with FSST, FastPFOR and shared dictionaries enabled, no sorting and no tessellation.
   * Every method marked v2 only has no effect on a v1 layer.
   */
  public static final class Builder {
    private final Map<MltEncoderOptions.Flag, Boolean> flags = new EnumMap<>(MltEncoderOptions.Flag.class);
    private @Nullable MltWireVersion wireVersion;

    private Builder() {}

    private Builder set(MltEncoderOptions.Flag flag, boolean enabled) {
      flags.put(flag, enabled);
      return this;
    }

    /** Generates tessellation data for polygons and multi-polygons. */
    public Builder tessellate(boolean enabled) {
      return set(MltEncoderOptions.Flag.TESSELLATE, enabled);
    }

    /** Tries sorting features by the Z-order (Morton) curve index. */
    public Builder attemptSpatialMortonSort(boolean enabled) {
      return set(MltEncoderOptions.Flag.MORTON_SORT, enabled);
    }

    /** Tries sorting features by the Hilbert curve index. */
    public Builder attemptSpatialHilbertSort(boolean enabled) {
      return set(MltEncoderOptions.Flag.HILBERT_SORT, enabled);
    }

    /** Tries sorting features by their feature ID in ascending order. */
    public Builder attemptIdSort(boolean enabled) {
      return set(MltEncoderOptions.Flag.ID_SORT, enabled);
    }

    /** Allows FSST string compression. */
    public Builder allowFsst(boolean enabled) {
      return set(MltEncoderOptions.Flag.FSST, enabled);
    }

    /** Allows FastPFOR integer compression. */
    public Builder allowFastPfor(boolean enabled) {
      return set(MltEncoderOptions.Flag.FASTPFOR, enabled);
    }

    /** Allows string grouping into shared dictionaries. */
    public Builder allowSharedDict(boolean enabled) {
      return set(MltEncoderOptions.Flag.SHARED_DICT, enabled);
    }

    /** Selects the wire format to encode to. */
    public Builder wireVersion(MltWireVersion version) {
      wireVersion = version;
      return this;
    }

    /** v2 only: lets a tessellated all-polygon layer store its triangles without the outlines. */
    public Builder allowTrianglesOnly(boolean enabled) {
      return set(MltEncoderOptions.Flag.TRIANGLES_ONLY, enabled);
    }

    /** v2 only: allows integer and vertex streams to store the deltas of their deltas. */
    public Builder allowDelta2(boolean enabled) {
      return set(MltEncoderOptions.Flag.DELTA2, enabled);
    }

    /** v2 only: allows float columns to store one code per value into a dictionary. */
    public Builder allowFloatDict(boolean enabled) {
      return set(MltEncoderOptions.Flag.FLOAT_DICT, enabled);
    }

    /** v2 only: allows float columns to store each value as a decimal-scaled integer (ALP). */
    public Builder allowFloatAlp(boolean enabled) {
      return set(MltEncoderOptions.Flag.FLOAT_ALP, enabled);
    }

    /** v2 only: allows dictionary code streams to be bit-packed. */
    public Builder allowPackedDictCodes(boolean enabled) {
      return set(MltEncoderOptions.Flag.PACKED_DICT_CODES, enabled);
    }

    /** v2 only: allows plain vertex streams to be rANS-coded. */
    public Builder allowRansVertices(boolean enabled) {
      return set(MltEncoderOptions.Flag.RANS_VERTICES, enabled);
    }

    /** v2 only: lets a nested struct or map code its row shapes instead of per-field presence. */
    public Builder allowRowShapes(boolean enabled) {
      return set(MltEncoderOptions.Flag.ROW_SHAPES, enabled);
    }

    MltEncoderOptions options() {
      return new MltEncoderOptions(flags, wireVersion);
    }

    /** Creates an encoder with these options, which the caller closes. */
    public MltEncoder build() {
      return new MltEncoder(options());
    }
  }
}
