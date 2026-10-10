package org.maplibre.mlt.ffi;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.util.ArrayList;
import java.util.List;
import org.maplibre.mlt.ffi.raw.MltLayerBytes;
import org.maplibre.mlt.ffi.raw.MltTileStats_layer_bytes_result;
import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/**
 * The bytes one layer of a tile spends, by what they hold.
 *
 * @param name the layer name
 * @param bytes the value of the layer's size varint: the tag and the body, without the varint itself
 * @param geometryBytes the geometry column, or the v2 geometry section
 * @param propertyBytes every column that is neither geometry nor the id
 * @param idBytes the id column
 */
public record MltLayerStats(String name, int bytes, int geometryBytes, int propertyBytes, int idBytes) {

  static {
    NativeLibrary.load();
  }

  /** Returns what is left of {@link #bytes} once every column is taken out: the tag, name, header and column schema. */
  public int metadataBytes() {
    return bytes - geometryBytes - propertyBytes - idBytes;
  }

  /**
   * Reads the stats of every layer of an MLT tile without decoding it, in tile order.
   * A layer with a tag the native library does not know is left out.
   *
   * @throws MltException if {@code mlt} cannot be parsed
   */
  public static List<MltLayerStats> of(byte[] mlt) {
    try (Arena arena = Arena.ofConfined()) {
      MemorySegment stats = Diplomat.unwrap(mlt_ffi_h.MltTileStats_from_bytes(arena, Diplomat.u8View(arena, mlt)));
      try {
        int count = (int) mlt_ffi_h.MltTileStats_layer_count(stats);
        List<MltLayerStats> layers = new ArrayList<>(count);
        for (int i = 0; i < count; i++) {
          long layer = i;
          MemorySegment bytes = MltTileStats_layer_bytes_result.ok(mlt_ffi_h.MltTileStats_layer_bytes(arena, stats, i));
          layers.add(new MltLayerStats(
            Diplomat.string(write -> mlt_ffi_h.MltTileStats_layer_name(stats, layer, write)),
            MltLayerBytes.size(bytes),
            MltLayerBytes.geometry(bytes),
            MltLayerBytes.properties(bytes),
            MltLayerBytes.ids(bytes)));
        }
        return layers;
      } finally {
        mlt_ffi_h.MltTileStats_destroy(stats);
      }
    }
  }
}
