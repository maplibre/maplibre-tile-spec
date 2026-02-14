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

  @Override
  @NotNull
  public Iterator<Feature> iterator() {
    return new Iterator<>() {
      private int index = 0;
      private final Iterator<Geometry> geometryIterator = geometryColumn.iterator();

      @Override
      public boolean hasNext() {
        return index < geometryColumn.numGeometries;
      }

      @Override
      public Feature next() {
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

        Feature feature;
        if (idColumn != null) {
          var idValue = idColumn.getValue(index);
          if (idValue.isPresent()) {
            long id =
                isIntVector(idColumn)
                    ? ((Integer) idValue.get()).longValue()
                    : (Long) idValue.get();
            feature = new Feature(id, geometry, properties);
          } else {
            feature = new Feature(geometry, properties);
          }
        } else {
          feature = new Feature(geometry, properties);
        }

        index++;
        return feature;
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
