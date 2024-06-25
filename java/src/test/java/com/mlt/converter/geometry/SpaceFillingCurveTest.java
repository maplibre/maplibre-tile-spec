package com.mlt.converter.geometry;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

public class SpaceFillingCurveTest {

  @Test
  public void ctor_negativeBounds_throwException() {
    assertThrows(IllegalArgumentException.class, () -> new TestSpaceFillingCurve(-20, -10));
  }

  @Test
  public void numBits_positiveBounds() {
    SpaceFillingCurve sfc = new TestSpaceFillingCurve(2, 16);
    assertEquals(4, sfc.numBits());
  }

  @Test
  public void numBits_zeroAndPositiveBounds() {
    SpaceFillingCurve sfc = new TestSpaceFillingCurve(0, 16);
    assertEquals(5, sfc.numBits());
  }

  @Test
  public void numBits_positiveAndNegativeBounds() {
    SpaceFillingCurve sfc = new TestSpaceFillingCurve(-16, 16);
    assertEquals(6, sfc.numBits());
  }

  private static class TestSpaceFillingCurve extends SpaceFillingCurve {
    public TestSpaceFillingCurve(int minVertexValue, int maxVertexValue) {
      super(minVertexValue, maxVertexValue);
    }

    @Override
    public int encode(Vertex vertex) {
      return 0;
    }

    @Override
    public int[] decode(int mortonCode) {
      return new int[0];
    }
  }
}
