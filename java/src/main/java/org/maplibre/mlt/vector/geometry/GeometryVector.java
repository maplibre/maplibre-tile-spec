package org.maplibre.mlt.vector.geometry;

import org.maplibre.mlt.converter.geometry.GeometryType;
import org.maplibre.mlt.decoder.GeometryDecoder;
import java.nio.IntBuffer;
import java.util.Iterator;
import java.util.Optional;
import org.locationtech.jts.geom.Geometry;

public class GeometryVector implements Iterable<Geometry> {

  public static class MortonSettings {
    public int numBits;
    public int coordinateShift;

    public MortonSettings(int numBits, int coordinateShift) {
      this.numBits = numBits;
      this.coordinateShift = coordinateShift;
    }
  }

  public enum VertexBufferType {
    MORTON,
    VEC_2,
    VEC_3
  }

  public final VertexBufferType vertexBufferType;
  private IntBuffer geometryTypes;
  private int geometryType;
  public final int numGeometries;
  public final TopologyVector topologyVector;
  public final IntBuffer vertexOffsets;
  public final IntBuffer vertexBuffer;
  public final Optional<MortonSettings> mortonSettings;

  public GeometryVector(
      VertexBufferType vertexBufferType,
      IntBuffer geometryTypes,
      TopologyVector topologyVector,
      IntBuffer vertexOffsets,
      IntBuffer vertexBuffer,
      Optional<MortonSettings> mortonSettings) {
    this.vertexBufferType = vertexBufferType;
    this.geometryTypes = geometryTypes;
    this.topologyVector = topologyVector;
    this.vertexOffsets = vertexOffsets;
    this.vertexBuffer = vertexBuffer;
    this.numGeometries = geometryTypes.capacity();
    this.mortonSettings = mortonSettings;
  }

  public GeometryVector(
      int numGeometries,
      int geometryType,
      VertexBufferType vertexBufferType,
      TopologyVector topologyVector,
      IntBuffer vertexOffsets,
      IntBuffer vertexBuffer,
      Optional<MortonSettings> mortonSettings) {
    this.vertexBufferType = vertexBufferType;
    this.topologyVector = topologyVector;
    this.vertexOffsets = vertexOffsets;
    this.vertexBuffer = vertexBuffer;
    this.numGeometries = numGeometries;
    this.geometryType = geometryType;
    this.mortonSettings = mortonSettings;
  }

  public static GeometryVector createMortonEncodedGeometryVector(
      IntBuffer geometryTypes,
      TopologyVector topologyVector,
      IntBuffer vertexOffsets,
      IntBuffer vertexBuffer,
      MortonSettings mortonInfo) {
    return new GeometryVector(
        VertexBufferType.MORTON,
        geometryTypes,
        topologyVector,
        vertexOffsets,
        vertexBuffer,
        Optional.of(mortonInfo));
  }

  public static GeometryVector createConstMortonEncodedGeometryVector(
      int numGeometries,
      int geometryType,
      TopologyVector topologyVector,
      IntBuffer vertexOffsets,
      IntBuffer vertexBuffer,
      MortonSettings mortonInfo) {
    return new GeometryVector(
        numGeometries,
        geometryType,
        VertexBufferType.MORTON,
        topologyVector,
        vertexOffsets,
        vertexBuffer,
        Optional.of(mortonInfo));
  }

  public static GeometryVector create2DGeometryVector(
      IntBuffer geometryTypes,
      TopologyVector topologyVector,
      IntBuffer vertexOffsets,
      IntBuffer vertexBuffer) {
    return new GeometryVector(
        VertexBufferType.VEC_2,
        geometryTypes,
        topologyVector,
        vertexOffsets,
        vertexBuffer,
        Optional.empty());
  }

  public static GeometryVector createConst2DGeometryVector(
      int numGeometries,
      int geometryType,
      TopologyVector topologyVector,
      IntBuffer vertexOffsets,
      IntBuffer vertexBuffer) {
    return new GeometryVector(
        numGeometries,
        geometryType,
        VertexBufferType.VEC_2,
        topologyVector,
        vertexOffsets,
        vertexBuffer,
        Optional.empty());
  }

  public int getGeometryType(int index) {
    return geometryTypes != null ? geometryTypes.get(index) : geometryType;
  }

  public boolean containsPolygonGeometry() {
    // TODO: get rid of this by only checking for the presence of partOffsets and ringOffsets?
    if (geometryTypes != null) {
      for (int i = 0; i < numGeometries; i++) {
        if (geometryTypes.get(i) == GeometryType.POLYGON.ordinal()
            || geometryTypes.get(i) == GeometryType.MULTIPOLYGON.ordinal()) {
          return true;
        }
      }
      return false;
    }

    return geometryType == GeometryType.POLYGON.ordinal()
        || geometryType == GeometryType.MULTIPOLYGON.ordinal();
  }

  @Override
  public Iterator<Geometry> iterator() {
    return new Iterator<>() {
      private int index = 0;
      private Geometry[] geometries;

      @Override
      public boolean hasNext() {
        return index < numGeometries;
      }

      @Override
      public Geometry next() {
        if (geometries == null) {
          // TODO: implement lazy materialization
          geometries = GeometryDecoder.decodeGeometryVectorized(GeometryVector.this);
        }
        return geometries[index++];
      }
    };
  }
}
