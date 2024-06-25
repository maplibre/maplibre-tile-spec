package com.mlt.converter.geometry;

public class ZOrderCurve extends SpaceFillingCurve {

  public ZOrderCurve(int minVertexValue, int maxVertexValue) {
    super(minVertexValue, maxVertexValue);
  }

  public int encode(Vertex vertex) {
    validateCoordinates(vertex);
    var shiftedX = vertex.x() + coordinateShift;
    var shiftedY = vertex.y() + coordinateShift;
    int mortonCode = 0;
    for (int i = 0; i < numBits; i++) {
      mortonCode |= (shiftedX & (1 << i)) << i | (shiftedY & (1 << i)) << (i + 1);
    }
    return mortonCode;
  }

  public int[] decode(int mortonCode) {
    int x = decodeMorton(mortonCode) - coordinateShift;
    int y = decodeMorton(mortonCode >> 1) - coordinateShift;
    return new int[] {x, y};
  }

  private int decodeMorton(int code) {
    int coordinate = 0;
    for (int i = 0; i < numBits; i++) {
      coordinate |= (int) ((code & (1L << (2 * i))) >> i);
    }
    return coordinate;
  }

  public static int[] decode(int mortonCode, int numBits, int coordinateShift) {
    int x = decodeMorton(mortonCode, numBits) - coordinateShift;
    int y = decodeMorton(mortonCode >> 1, numBits) - coordinateShift;
    return new int[] {x, y};
  }

  private static int decodeMorton(int code, int numBits) {
    int coordinate = 0;
    for (int i = 0; i < numBits; i++) {
      coordinate |= (int) ((code & (1L << (2 * i))) >> i);
    }
    return coordinate;
  }
}
