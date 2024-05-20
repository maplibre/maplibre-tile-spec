package com.mlt.vector;

import com.mlt.converter.mvt.Feature;
import com.mlt.vector.geometry.GeometryVector;

import java.nio.charset.StandardCharsets;
import java.util.Iterator;
import java.util.Optional;

public class FeatureTable implements Iterable<Feature>{
    private final String name;
    private final Vector idColumn;
    private final GeometryVector geometryColumn;
    private final Vector[] propertyColumns;

    public FeatureTable(String name, GeometryVector geometryVector,
                       Vector[] properties) {
        this(name, null, geometryVector, properties);
    }

    public FeatureTable(String name, Vector idColumn, GeometryVector geometryVector,
                       Vector[] properties) {
        this.name = name;
        this.idColumn = idColumn;
        this.geometryColumn = geometryVector;
        this.propertyColumns = properties;
    }

    @Override
    public Iterator<Feature> iterator() {
        return new Iterator<>() {
            private int index = 0;

            @Override
            public boolean hasNext() {
                return index < idColumn.size();
            }

            @Override
            public Feature next() {
                var id = (long)idColumn.getValue(index++).get();
                //var geometry = geometryColumn
                return new Feature(id, null, null);
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
