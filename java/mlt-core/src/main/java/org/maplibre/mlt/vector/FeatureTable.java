package org.maplibre.mlt.vector;

import java.util.HashMap;
import java.util.Iterator;
import org.jetbrains.annotations.NotNull;
import org.locationtech.jts.geom.Geometry;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.vector.constant.IntConstVector;
import org.maplibre.mlt.vector.flat.IntFlatVector;
import org.maplibre.mlt.vector.geometry.GeometryVector;
import org.maplibre.mlt.vector.sequence.IntSequenceVector;

/** In-Memory representation of MLT storage format for efficient processing */
public class FeatureTable implements Iterable<Feature> {
  private final String name;

  @SuppressWarnings("rawtypes")
  private final Vector idColumn;

  private final GeometryVector geometryColumn;

  @SuppressWarnings("rawtypes")
  private final Vector[] propertyColumns;

  public FeatureTable(
      String name,
      GeometryVector geometryVector,
      @SuppressWarnings("rawtypes") Vector[] properties) {
    this(name, null, geometryVector, properties);
  }

  public FeatureTable(
      String name,
      @SuppressWarnings("rawtypes") Vector idColumn,
      GeometryVector geometryVector,
      @SuppressWarnings("rawtypes") Vector[] properties) {
    this.name = name;
    this.idColumn = idColumn;
    this.geometryColumn = geometryVector;
    this.propertyColumns = properties;
  }

  @Override
  @NotNull
  public Iterator<Feature> iterator() {
    return new Iterator<>() {
      private int index = 0;
      private final Iterator<Geometry> geometryIterator = geometryColumn.iterator();

      @Override
      public boolean hasNext() {
        return index < idColumn.size();
      }

      @Override
      public Feature next() {
        var id =
            isIntVector(idColumn)
                ? ((Integer) idColumn.getValue(index).get()).longValue()
                : (Long) idColumn.getValue(index).get();

        var geometry = geometryIterator.next();

        var properties = new HashMap<String, Object>();
        for (var propertyColumnVector : propertyColumns) {
          var columnName = propertyColumnVector.getName();
          var propertyValue = propertyColumnVector.getValue(index);
          if (propertyValue.isPresent()) {
            var value = propertyValue.get();
            properties.put(columnName, value);
          }
        }

        index++;
        return new Feature(id, geometry, properties);
      }
    };
  }

  private boolean isIntVector(Vector<?, ?> vector) {
    return vector instanceof IntFlatVector
        || vector instanceof IntConstVector
        || vector instanceof IntSequenceVector;
  }

  public String getName() {
    return name;
  }

  @SuppressWarnings("rawtypes")
  public Vector getIdColumn() {
    return idColumn;
  }

  public GeometryVector getGeometryColumn() {
    return geometryColumn;
  }

  @SuppressWarnings("rawtypes")
  public Vector[] getPropertyColumns() {
    return propertyColumns;
  }
}
