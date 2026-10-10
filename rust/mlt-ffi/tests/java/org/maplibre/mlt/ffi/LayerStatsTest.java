package org.maplibre.mlt.ffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import org.junit.jupiter.api.Test;

class LayerStatsTest {

  private static final int EXTENT = 4096;

  private static int[] point(int x, int y) {
    return new int[]{(1 << 3) | 1, x << 1, y << 1};
  }

  private static List<MltLayerSize> sizes(List<MltLayerStats> stats) {
    return stats.stream().map(layer -> new MltLayerSize(layer.name(), layer.bytes())).toList();
  }

  @Test
  void statsSplitEachLayerByColumnRole() {
    try (var encoder = new MltEncoder()) {
      encoder.beginLayer("with_id", EXTENT);
      encoder.beginFeature(MvtGeometryType.POINT, point(1, 1), 3);
      encoder.setString("name", "alpha");
      encoder.beginLayer("bare", EXTENT);
      encoder.beginFeature(MvtGeometryType.POINT, point(2, 2));

      List<MltLayerStats> stats = MltLayerStats.of(encoder.toByteArray());

      assertEquals(List.of(
        new MltLayerStats("with_id", 52, 12, 15, 5),
        new MltLayerStats("bare", 22, 12, 0, 0)), stats);
      assertEquals(List.of(20, 10), stats.stream().map(MltLayerStats::metadataBytes).toList());
    }
  }

  @Test
  void lastLayerSizesMatchTheStatsOfTheTile() {
    try (var encoder = new MltEncoder()) {
      encoder.beginLayer("small", EXTENT);
      encoder.beginFeature(MvtGeometryType.POINT, point(1, 1));
      encoder.beginLayer("large", EXTENT);
      for (int i = 0; i < 100; i++) {
        encoder.beginFeature(MvtGeometryType.POINT, point(i * 37, i * 11), i);
        encoder.setLong("rank", i * 1009L);
      }

      byte[] tile = encoder.toByteArray();

      assertEquals(sizes(MltLayerStats.of(tile)), encoder.lastLayerSizes());
      assertTrue(encoder.lastLayerSizes().get(1).bytes() > 127);
    }
  }

  @Test
  void lastLayerSizesFollowTheMostRecentTile() {
    try (var encoder = new MltEncoder()) {
      encoder.beginLayer("first", EXTENT);
      encoder.beginFeature(MvtGeometryType.POINT, point(1, 1));
      encoder.toByteArray();
      encoder.beginLayer("second", EXTENT);
      encoder.beginFeature(MvtGeometryType.POINT, point(1, 1));

      byte[] tile = encoder.toByteArray();

      assertEquals(List.of(new MltLayerSize("second", tile.length - 1)), encoder.lastLayerSizes());
    }
  }

  @Test
  void lastLayerSizesLeaveOutALayerWithoutFeatures() {
    try (var encoder = new MltEncoder()) {
      encoder.beginLayer("empty", EXTENT);
      encoder.beginLayer("full", EXTENT);
      encoder.beginFeature(MvtGeometryType.POINT, point(1, 1));

      byte[] tile = encoder.toByteArray();

      assertEquals(List.of(new MltLayerSize("full", tile.length - 1)), encoder.lastLayerSizes());
    }
  }

  @Test
  void lastLayerSizesBeforeAnyTileAreEmpty() {
    try (var encoder = new MltEncoder()) {
      assertEquals(List.of(), encoder.lastLayerSizes());
    }
  }

  @Test
  void statsOfGarbageReportInvalidInput() {
    var error = assertThrows(MltException.class, () -> MltLayerStats.of(new byte[]{(byte) 0xff}));

    assertEquals(MltException.Kind.INVALID_INPUT, error.kind());
  }
}
