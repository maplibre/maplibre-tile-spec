package org.maplibre.mlt.metadata.tileset;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

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
     * used for a efficient representation of linear referencing
     */
    RANGE_MAP,
    UNRECOGNIZED
  }

  public static final class TileSetMetadata {
    public List<FeatureTable> featureTables = new ArrayList<>();
    public String name;
    public String description;
    public java.lang.Object attribution;

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

    public String name;
    public List<Column> columns;
  }

  public static class Field {
    public Field(String name, ScalarField type) {
      this.name = name;
      this.scalarType = type;
    }

    public Field(String name, ComplexField type) {
      this.name = name;
      this.complexType = type;
    }

    public String name;
    public boolean isNullable;
    public ComplexField complexType;
    public ScalarField scalarType;
  }

  /** Column are top-level types in the schema */
  public static final class Column extends Field {
    public Column(String name, ScalarField type) {
      super(name, type);
    }

    public Column(String name, ComplexField type) {
      super(name, type);
    }

    public ColumnScope columnScope;
  }

  public static final class ScalarField {
    public ScalarField(ScalarType type) {
      physicalType = type;
    }

    public ScalarField(LogicalScalarType type) {
      logicalType = type;
    }

    public ScalarType physicalType;
    public LogicalScalarType logicalType;
    public boolean hasLongId;
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
