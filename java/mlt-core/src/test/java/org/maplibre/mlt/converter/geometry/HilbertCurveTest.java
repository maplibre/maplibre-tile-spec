package org.maplibre.mlt.converter.geometry;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

public class HilbertCurveTest {

  @Test
  public void level1_hasExpectedStandardOrder() {
    final var curve = new HilbertCurve(0, 1);
    assertArrayEquals(new int[] {0, 0}, curve.decode(0));
    assertArrayEquals(new int[] {0, 1}, curve.decode(1));
    assertArrayEquals(new int[] {1, 1}, curve.decode(2));
    assertArrayEquals(new int[] {1, 0}, curve.decode(3));
  }

  @ParameterizedTest(name = "level={0}")
  @ValueSource(ints = {1, 2, 5, 10, 16})
  public void encodeDecode_roundTripsForRepresentativeLevels(int level) {
    assertRoundTripForLevel(level);
  }

  @Test
  public void consecutiveIndices_areAxisAdjacentAtLevel6() {
    int level = 6;
    int maxCoordinate = (1 << level) - 1;
    final var curve = new HilbertCurve(0, maxCoordinate);

    int maxIndexExclusive = 1 << (2 * level);
    for (int index = 0; index < maxIndexExclusive - 1; index++) {
      final var a = curve.decode(index);
      final var b = curve.decode(index + 1);
      final int manhattan = Math.abs(a[0] - b[0]) + Math.abs(a[1] - b[1]);
      assertEquals(1, manhattan, "Hilbert curve should move one grid step per index increment");
    }
  }

  @Test
  public void throwsWhenLevelIsAboveMaximumAllowed() {
    assertThrows(IllegalArgumentException.class, () -> assertRoundTripForLevel(17));
  }

  private static void assertRoundTripForLevel(int level) {
    int maxCoordinate = (1 << level) - 1;
    final var curve = new HilbertCurve(0, maxCoordinate);

    assertRoundTrip(curve, 0, 0);
    assertRoundTrip(curve, maxCoordinate, 0);
    assertRoundTrip(curve, 0, maxCoordinate);
    assertRoundTrip(curve, maxCoordinate, maxCoordinate);
    assertRoundTrip(curve, maxCoordinate / 2, maxCoordinate / 2);

    final int step = Math.max(1, maxCoordinate / 7);
    for (int y = 0; y <= maxCoordinate; y += step) {
      for (int x = 0; x <= maxCoordinate; x += step) {
        assertRoundTrip(curve, x, y);
      }
    }
  }

  private static void assertRoundTrip(HilbertCurve curve, int x, int y) {
    final int index = curve.encode(new Vertex(x, y));
    final int[] decoded = curve.decode(index);
    assertArrayEquals(new int[] {x, y}, decoded, "Coordinate should round-trip through encode/decode");
  }
}
