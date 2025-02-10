package com.mlt.converter.triangulation;

import java.util.ArrayList;
import java.util.List;

public class TriangulatedPolygon {
  private final int numTrianglesPerPolygon;

  private final ArrayList<Integer> indexBuffer;

  TriangulatedPolygon(ArrayList<Integer> indexBuffer, int numTriangles) {
    this.numTrianglesPerPolygon = numTriangles;
    this.indexBuffer = indexBuffer;
  }

  public Integer getNumTrianglesPerPolygon() {
    return numTrianglesPerPolygon;
  }

  public List<Integer> getIndexBuffer() {
    return indexBuffer;
  }
}
