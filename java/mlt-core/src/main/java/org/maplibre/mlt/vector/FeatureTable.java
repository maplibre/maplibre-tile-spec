package org.maplibre.mlt.vector;

import org.maplibre.mlt.vector.constant.IntConstVector;
import org.maplibre.mlt.vector.flat.IntFlatVector;
import org.maplibre.mlt.vector.geometry.GeometryVector;
import org.maplibre.mlt.vector.sequence.IntSequenceVector;

/** In-Memory representation of MLT storage format for efficient processing */
public class FeatureTable {
  private final String name;

  private final Vector<?, ?> idColumn;

  private final GeometryVector geometryColumn;

  private final Vector<?, ?>[] propertyColumns;

  public FeatureTable(String name, GeometryVector geometryVector, Vector<?, ?>[] properties) {
    this(name, null, geometryVector, properties);
  }

  public FeatureTable(
      String name,
      Vector<?, ?> idColumn,
      GeometryVector geometryVector,
      Vector<?, ?>[] properties) {
    this.name = name;
    this.idColumn = idColumn;
    this.geometryColumn = geometryVector;
    this.propertyColumns = properties;
  }

  private boolean isIntVector(Vector<?, ?> vector) {
    return vector instanceof IntFlatVector
        || vector instanceof IntConstVector
        || vector instanceof IntSequenceVector;
  }

  public String getName() {
    return name;
  }

  public Vector<?, ?> getIdColumn() {
    return idColumn;
  }

  public GeometryVector getGeometryColumn() {
    return geometryColumn;
  }

  public Vector<?, ?>[] getPropertyColumns() {
    return propertyColumns;
  }
}
