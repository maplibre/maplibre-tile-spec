package org.maplibre.mlt.metadata.tileset;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.SequencedCollection;

public final class MltMetadata {
  private MltMetadata() {}

  public enum ColumnScope {
    /** 1:1 Mapping of property and feature to id and geometry */
    FEATURE,
    /** For M-Values, 1:1 Mapping for property and vertex */
    VERTEX,
    UNRECOGNIZED
  }

  public enum ScalarType {
    BOOLEAN,
    INT_8,
    UINT_8,
    INT_32,
    UINT_32,
    INT_64,
    UINT_64,
    FLOAT,
    DOUBLE,
    STRING,
    UNRECOGNIZED
  }

  public enum ComplexType {
    GEOMETRY,
    STRUCT,
    MAP, // nested property map
    UNRECOGNIZED
  }

  public enum LogicalScalarType {
    ID,
    UNRECOGNIZED
  }

  public enum LogicalComplexType {
    /** physical type: list&lt;UInt8&gt; */
    BINARY,
    /**
     * physical type: map&lt;vec2&lt;double, T&gt;&gt; -&gt; special data structure which can be
     * used for an efficient representation of linear referencing
     */
    RANGE_MAP,
    UNRECOGNIZED
  }

  public static final class TileSetMetadata {
    public List<FeatureTable> featureTables = new ArrayList<>();
    public String name;
    public String description;
    public Object attribution;

    @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
    public Optional<Integer> minZoom = Optional.empty();

    @SuppressWarnings("OptionalUsedAsFieldOrParameterType")
    public Optional<Integer> maxZoom = Optional.empty();

    public List<Double> bounds = new ArrayList<>();
    public List<Double> center = new ArrayList<>();
  }

  public static final class FeatureTable {
    public FeatureTable(String name) {
      this.name = name;
      this.columns = new ArrayList<>();
    }
    public FeatureTable(String name, int initialColumnCapacity) {
      this.name = name;
      this.columns = new ArrayList<>(initialColumnCapacity);
    }

    public final String name;
    public final SequencedCollection<Column> columns;
  }

  public static class FieldType {
    public FieldType(ScalarField type, boolean isNullable) {
      this.scalarType = type;
      this.complexType = null;
      this.isNullable = isNullable;
    }

    public FieldType(ComplexField type, boolean isNullable) {
      this.scalarType = null;
      this.complexType = type;
      this.isNullable = isNullable;
    }

    public final boolean isNullable;
    public final ComplexField complexType;
    public final ScalarField scalarType;
  }

  public static class Field extends FieldType {
    public Field(String name, ScalarField type, boolean isNullable) {
      super(type, isNullable);
      this.name = name;
    }

    public Field(String name, ComplexField type, boolean isNullable) {
      super(type, isNullable);
      this.name = name;
    }

    public String name;
  }

  /** Column are top-level types in the schema */
  public static final class Column extends Field {
    public Column(String name, ScalarField type, boolean isNullable) {
      super(name, type, isNullable);
    }

    public Column(String name, ComplexField type, boolean isNullable) {
      super(name, type, isNullable);
    }

    public ColumnScope columnScope;
  }

  public static final class ScalarField {
    public ScalarField(ScalarType type) {
      physicalType = type;
      logicalType = null;
      hasLongId = false;
    }

    public ScalarField(LogicalScalarType type, boolean hasLongId) {
      physicalType = null;
      logicalType = type;
      this.hasLongId = hasLongId;
    }

    public final ScalarType physicalType;
    public final LogicalScalarType logicalType;
    public final boolean hasLongId;
  }

  public static final class ComplexField {
    public ComplexField(ComplexType type) {
      this(type, new ArrayList<>());
    }

    public ComplexField(ComplexType type, List<Field> children) {
      this.physicalType = type;
      this.children = children;
    }

    public ComplexType physicalType;
    public LogicalComplexType logicalType;

    public List<Field> children;
  }
}
