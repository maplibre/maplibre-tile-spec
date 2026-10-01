package org.maplibre.mlt.converter.geometry;

public class HilbertCurve extends SpaceFillingCurve {

  public HilbertCurve(int minVertexValue, int maxVertexValue) {
    super(minVertexValue, maxVertexValue);
  }

  public int encode(Vertex vertex) {
    var shiftedX = vertex.x() + coordinateShift;
    var shiftedY = vertex.y() + coordinateShift;
    return Hilbert.hilbertXYToIndex(numBits, shiftedX, shiftedY);
  }

  public int[] decode(int hilbertIndex) {
    return Hilbert.hilbertPositionToXY(numBits, hilbertIndex);
  }
}
