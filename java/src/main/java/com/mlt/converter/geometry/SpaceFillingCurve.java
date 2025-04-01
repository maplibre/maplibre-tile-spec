package com.mlt.converter.geometry;

public abstract class SpaceFillingCurve {
  protected int tileExtent;
  protected int numBits;
  protected int coordinateShift;
  private final int minBound;
  private final int maxBound;

  public SpaceFillingCurve(int minVertexValue, int maxVertexValue) {
    // TODO: fix tile buffer problem
    /* Each tile can have a buffer around, which means the coordinate values are extended beyond the specified tileExtent.
     * Currently we are extending size tile size be half of to the size into each direction, which works well for the test tilesets.
     * But this leads to problems if the tile coordinates are not within this boundaries.
     * */
    coordinateShift = minVertexValue < 0 ? Math.abs(minVertexValue) : 0;
    this.tileExtent = maxVertexValue + coordinateShift;
    this.numBits = (int) Math.ceil((Math.log(this.tileExtent + 1) / Math.log(2)));
    ;
    minBound = minVertexValue;
    maxBound = maxVertexValue;
  }

  protected void validateCoordinates(Vertex vertex) {
    // TODO: also check of int overflow as we limiting the sfc ids to max int size
    if (vertex.x() < minBound
        || vertex.y() < minBound
        || vertex.x() > maxBound
        || vertex.y() > maxBound) {
      // System.err.println("The specified tile buffer size is currently not supported.");
      throw new IllegalArgumentException(
          "The specified tile buffer size is currently not supported.");
    }
  }

  public abstract int encode(Vertex vertex);

  public abstract int[] decode(int mortonCode);

  public int numBits() {
    return this.numBits;
  }

  public int coordinateShift() {
    return this.coordinateShift;
  }
}
