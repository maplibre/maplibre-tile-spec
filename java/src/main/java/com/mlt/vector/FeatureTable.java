package com.mlt.vector;

import com.mlt.converter.mvt.Feature;
import com.mlt.vector.flat.IntFlatVector;
import com.mlt.vector.geometry.GeometryVector;
import java.util.HashMap;
import java.util.Iterator;
import org.locationtech.jts.geom.Geometry;

/** In-Memory representation of MLT storage format for efficient processing */
public class FeatureTable implements Iterable<Feature> {
  private final String name;
  private final Vector idColumn;
  private final GeometryVector geometryColumn;
  private final Vector[] propertyColumns;

  public FeatureTable(String name, GeometryVector geometryVector, Vector[] properties) {
    this(name, null, geometryVector, properties);
  }

  public FeatureTable(
      String name, Vector idColumn, GeometryVector geometryVector, Vector[] properties) {
    this.name = name;
    this.idColumn = idColumn;
    this.geometryColumn = geometryVector;
    this.propertyColumns = properties;
  }

  @Override
  public Iterator<Feature> iterator() {
    return new Iterator<>() {
      private int index = 0;
      private Iterator<Geometry> geometryIterator = geometryColumn.iterator();

      @Override
      public boolean hasNext() {
        return index < idColumn.size();
      }

      @Override
      public Feature next() {
        var id =
            idColumn instanceof IntFlatVector
                ? ((Integer) idColumn.getValue(index).get()).longValue()
                : (Long) idColumn.getValue(index).get();
        var geometry = geometryIterator.next();

        var properties = new HashMap<String, Object>();
        for (var i = 0; i < propertyColumns.length; i++) {
          var propertyColumnVector = propertyColumns[i];
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

  public String getName() {
    return name;
  }

  public Vector getIdColumn() {
    return idColumn;
  }

  public GeometryVector getGeometryColumn() {
    return geometryColumn;
  }

  public Vector[] getPropertyColumns() {
    return propertyColumns;
  }
}
