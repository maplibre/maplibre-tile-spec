package org.maplibre.mlt.converter.geometry;

public abstract class SpaceFillingCurve {
  protected int tileExtent;
  protected int numBits;
  protected int coordinateShift;
  private final int minBound;
  private final int maxBound;

  public static final int MAX_SUPPORTED_BITS = 16;
  public static final int MAX_SUPPORTED_RANGE = (1 << MAX_SUPPORTED_BITS) - 1;

  public static boolean isRangeSupported(int minVertexValue, int maxVertexValue) {
    return (maxVertexValue + getCoordinateShift(minVertexValue)) <= MAX_SUPPORTED_RANGE;
  }

  public SpaceFillingCurve(int minVertexValue, int maxVertexValue) {
    // TODO: fix tile buffer problem
    /* Each tile can have a buffer around, which means the coordinate values are extended beyond the specified tileExtent.
     * Currently we are extending size tile size be half of to the size into each direction, which works well for the test tilesets.
     * But this leads to problems if the tile coordinates are not within this boundaries.
     * */
    this.coordinateShift = getCoordinateShift(minVertexValue);
    this.tileExtent = maxVertexValue + coordinateShift;
    this.numBits = (int) Math.ceil((Math.log(this.tileExtent + 1) / Math.log(2)));
    this.minBound = minVertexValue;
    this.maxBound = maxVertexValue;

    if (numBits > MAX_SUPPORTED_BITS) {
      throw new IllegalArgumentException("Tile coordinate span " + this.tileExtent + " is too large");
    }
  }

  protected void validateCoordinates(Vertex vertex) {
    // TODO: also check of int overflow as we limiting the sfc ids to max int size
    if (vertex.x() < minBound
        || vertex.y() < minBound
        || vertex.x() > maxBound
        || vertex.y() > maxBound) {
      throw new IllegalArgumentException(
          "The specified tile buffer size is currently not supported.");
    }
  }

  private static int getCoordinateShift(int minVertexValue) {
    if (minVertexValue == Integer.MIN_VALUE) {
      return Integer.MAX_VALUE;
    }
    return (minVertexValue < 0) ? Math.abs(minVertexValue) : 0;
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
