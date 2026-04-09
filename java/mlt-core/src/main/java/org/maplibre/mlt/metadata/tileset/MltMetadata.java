package org.maplibre.mlt.metadata.tileset;

import jakarta.annotation.Nullable;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.SequencedCollection;
import lombok.Builder;
import lombok.experimental.SuperBuilder;
import org.jetbrains.annotations.NotNull;

public final class MltMetadata {
  private MltMetadata() {}

  /// Create a builder for a scalar field type
  /// @param type the physical scalar type of the field
  /// @return a builder for a scalar field type with the specified physical type
  public static FieldType.FieldTypeBuilder scalarFieldTypeBuilder(@NotNull ScalarType type) {
    return FieldType.builder().scalarType(new ScalarField(type));
  }

  /// Create a builder with the type already set to ID
  /// @param hasLongId indicates whether the ID is a 64-bit integer (long) or a 32-bit integer (int)
  /// @return a builder with the type already set to ID
  public static FieldType.FieldTypeBuilder idFieldTypeBuilder(boolean hasLongId) {
    return FieldType.builder().scalarType(new ScalarField(LogicalScalarType.ID, hasLongId));
  }

  /// Create a builder for a struct field type
   /// @param children the child fields of the struct, or null if there are no children
   /// @return a builder for a struct field type with the specified children
  public static FieldType.FieldTypeBuilder structFieldTypeBuilder(@Nullable SequencedCollection<Field> children) {
    return FieldType.builder().complexType(new ComplexField(ComplexType.STRUCT, children));
  }

  public static FieldType.FieldTypeBuilder geometryFieldTypeBuilder() {
    return FieldType.builder().complexType(new ComplexField(ComplexType.GEOMETRY));
  }

  public static FieldType.FieldTypeBuilder complexFieldTypeBuilder(@NotNull ComplexType type) {
    return FieldType.builder().complexType(new ComplexField(type, null));
  }

  public static FieldType.FieldTypeBuilder complexFieldTypeBuilder(@Nullable List<Field> children) {
    return FieldType.builder().complexType(new ComplexField(ComplexType.STRUCT, children));
  }

  /// Describes how the column's values are associated with features and geometries
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

  @Builder(toBuilder = true)
  public static class FieldType {
    @Builder.Default public final boolean isNullable = false;
    public final @Nullable ComplexField complexType;
    public final @Nullable ScalarField scalarType;

    FieldType(boolean isNullable, @Nullable ComplexField complexType, @Nullable ScalarField scalarType) {
      this.isNullable = isNullable;
      this.complexType = complexType;
      this.scalarType = scalarType;
      if ((complexType != null) == (scalarType != null)) {
        throw new IllegalStateException(
                "Field type must be either a complex type or a scalar type");
      }
    }

    public boolean is(ScalarType type) {
      return (scalarType != null && scalarType.physicalType == type);
    }
  }

  @SuperBuilder(toBuilder = true)
  public static class Field {
    public final @Nullable String name;
    public final @NotNull FieldType type;

    // Declare the generated builder class so that Javadoc can link to it.
    // The actual implementation will be generated by Lombok.
    public abstract static class FieldBuilder<C extends Field, B extends FieldBuilder<C, B>> {
      public FieldType type;
    }
  }

  /** Column are top-level types in the schema */
  @SuperBuilder(toBuilder = true)
  public static final class Column extends Field {
    protected Column(ColumnBuilder<?, ?> builder) {
      super(builder);
      this.columnScope = builder.columnScope$set ? builder.columnScope$value : ColumnScope.FEATURE;
      if (this.columnScope != ColumnScope.FEATURE && this.columnScope != ColumnScope.VERTEX) {
        throw new IllegalStateException("Column scope must be either FEATURE or VERTEX");
      }
    }

    @Builder.Default public final @NotNull ColumnScope columnScope = ColumnScope.FEATURE;

    // Declare the generated builder class so that Javadoc can link to it.
    // The actual implementation will be generated by Lombok.
    public abstract static class ColumnBuilder<C extends Column, B extends ColumnBuilder<C, B>>
        extends Field.FieldBuilder<C, B> {}
  }

  public static final class ScalarField {
    public ScalarField(@NotNull ScalarType type) {
      Objects.requireNonNull(type);
      physicalType = type;
      logicalType = null;
      hasLongId = false;
    }

    public ScalarField(@NotNull LogicalScalarType type, boolean hasLongId) {
      Objects.requireNonNull(type);
      physicalType = null;
      logicalType = type;
      this.hasLongId = hasLongId;
    }

    public final ScalarType physicalType;
    public final LogicalScalarType logicalType;
    public final boolean hasLongId;
  }

  public static final class ComplexField {
    public ComplexField(@NotNull ComplexType type) {
      this(type, new ArrayList<>());
    }

    public ComplexField(@NotNull ComplexType type, @Nullable SequencedCollection<Field> children) {
      Objects.requireNonNull(type);
      this.physicalType = type;
      this.children = (children != null) ? children : new ArrayList<>();
    }

    public @Nullable ComplexType physicalType;
    public @Nullable LogicalComplexType logicalType;

    public @NotNull SequencedCollection<Field> children;
  }
}
