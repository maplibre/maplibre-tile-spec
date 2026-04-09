package org.maplibre.mlt.metadata.tileset;

import jakarta.annotation.Nullable;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.SequencedCollection;
import org.jetbrains.annotations.NotNull;

public final class MltMetadata {
  private MltMetadata() {}

  public static FieldType scalarFieldType(@NotNull ScalarType type, boolean isNullable) {
    return new FieldType(new ScalarField(type), isNullable);
  }

  public static FieldType idFieldType(boolean hasLongId, boolean isNullable) {
    return new FieldType(new ScalarField(LogicalScalarType.ID, hasLongId), isNullable);
  }

  public static FieldType structFieldType(@Nullable SequencedCollection<Field> children) {
    return new FieldType(new ComplexField(ComplexType.STRUCT, children), false);
  }

  public static FieldType geometryFieldType() {
    return new FieldType(new ComplexField(ComplexType.GEOMETRY), false);
  }

  public static FieldType complexFieldType(@NotNull ComplexType type, boolean isNullable) {
    return new FieldType(new ComplexField(type, null), isNullable);
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

  /// The type of data in a Field
  public static class FieldType {
    /// Whether the field can be null
    public final boolean isNullable;
    /// A complex type, if set.  Mutually exclusive with scalarType.
    @Nullable public final ComplexField complexType;
    /// A scalar type, if set.  Mutually exclusive with complexType.
    @Nullable public final ScalarField scalarType;

    /// Create a scalar type
    /// @param scalarType the type
    /// @param isNullable whether the field can be null
    public FieldType(@Nullable ScalarField scalarType, boolean isNullable) {
      this.isNullable = isNullable;
      this.complexType = null;
      this.scalarType = scalarType;
    }

    /// Create a complex type
    /// @param complexType the type
    /// @param isNullable whether the field can be null
    public FieldType(@Nullable ComplexField complexType, boolean isNullable) {
      this.isNullable = isNullable;
      this.complexType = complexType;
      this.scalarType = null;
    }

    /// Check if this field is of the given scalar type
    /// @param type the scalar type to check against
    /// @return true if this field is of the given scalar type
    public boolean is(ScalarType type) {
      return (scalarType != null && scalarType.physicalType == type);
    }

    /// Check if this field is of the given logical scalar type
    /// @param type the logical scalar type to check against
    /// @return true if this field is of the given logical scalar type
    public boolean is(LogicalScalarType type) {
      return (scalarType != null && scalarType.logicalType == type);
    }

    /// Check if this field is of the given complex type
    /// @param type the complex type to check against
    /// @return true if this field is of the given complex type
    public boolean is(ComplexType type) {
      return (complexType != null && complexType.physicalType == type);
    }

    /// Check if this field is of the given logical complex type
    /// @param type the logical complex type to check against
    /// @return true if this field is of the given logical complex type
    public boolean is(LogicalComplexType type) {
      return (complexType != null && complexType.logicalType == type);
    }

    /// Get the physical scalar type of this field, if applicable
    /// @return an Optional containing the physical scalar type of this field, or an empty Optional
    // if this field is not a scalar type
    public Optional<ScalarType> getScalarType() {
      return (scalarType != null) ? Optional.ofNullable(scalarType.physicalType) : Optional.empty();
    }

    /// Get the logical scalar type of this field, if applicable
    /// @return an Optional containing the logical scalar type of this field, or an empty Optional
    // if this field is not a scalar type
    public Optional<LogicalScalarType> getLogicalScalarType() {
      return (scalarType != null) ? Optional.ofNullable(scalarType.logicalType) : Optional.empty();
    }

    /// Get the physical complex type of this field, if applicable
    /// @return an Optional containing the physical complex type of this field, or an empty Optional
    // if this field is not a complex type
    public Optional<ComplexType> getComplexType() {
      return (complexType != null)
          ? Optional.ofNullable(complexType.physicalType)
          : Optional.empty();
    }

    /// Get the logical complex type of this field, if applicable
    /// @return an Optional containing the logical complex type of this field, or an empty Optional
    // if this field is not a complex type
    public Optional<LogicalComplexType> getLogicalComplexType() {
      return (complexType != null)
          ? Optional.ofNullable(complexType.logicalType)
          : Optional.empty();
    }

    /// Get the child fields of this field, if applicable (e.g., for STRUCT or MAP types)
    /// @return an Optional containing a SequencedCollection of child fields, or an empty Optional
    public Optional<SequencedCollection<Field>> getChildren() {
      return (complexType != null) ? Optional.ofNullable(complexType.children) : Optional.empty();
    }
  }

  /// A field may be a column or nested as a child of a complex type
  public static class Field {
    /// The name of the field.  May be null for nested fields or implicit types (e.g, ID).
    @Nullable public final String name;
    /// The type of the field.  Must be non-null.
    @NotNull public final FieldType type;

    /// Create a field with the given type
    /// @param type the type of the field
    public Field(@NotNull FieldType type) {
      this(type, null);
    }

    /// Create a field with the given type and name
    /// @param type the type of the field
    /// @param name the name of the field, or null if unnamed
    public Field(@NotNull FieldType type, @Nullable String name) {
      this.type = Objects.requireNonNull(type);
      this.name = name;
    }
  }

  /** Column are top-level types in the schema */
  public static final class Column {
    /// The field associated with this column.  Must be non-null.
    @NotNull public final Field field;
    /// The column scope.  Must be non-null.
    public final @NotNull ColumnScope columnScope;

    /// Create a column with the given field and default scope of FEATURE
    /// @param field the field associated with this column
    public Column(Field field) {
      this(field, ColumnScope.FEATURE);
    }

    /// Create a column with the given type and default scope of FEATURE
    /// @param type the type of the field associated with this column
    public Column(FieldType type) {
      this(new Field(type), ColumnScope.FEATURE);
    }

    /// Create a column with the given field and column scope
    /// @param field the field associated with this column
    /// @param columnScope the column scope, which must be either FEATURE or VERTEX
    public Column(Field field, ColumnScope columnScope) {
      this.field = Objects.requireNonNull(field);
      this.columnScope = columnScope;
      if (this.columnScope != ColumnScope.FEATURE && this.columnScope != ColumnScope.VERTEX) {
        throw new IllegalStateException("Column scope must be either FEATURE or VERTEX");
      }
    }

    /// Get the name of this column
    /// @return the name of this column, or null if unnamed
    public String getName() {
      return field.name;
    }

    /// Check if this column is nullable
    /// @return true if this column is nullable, false otherwise
    public boolean isNullable() {
      return field.type.isNullable;
    }

    /// Check if this column is a scalar type
    /// @return true if this column is a scalar type, false otherwise
    public boolean isScalar() {
      return field.type.scalarType != null;
    }

    /// Check if this column is a complex type
    /// @return true if this column is a complex type, false otherwise
    public boolean isComplex() {
      return field.type.complexType != null;
    }

    /// Check if this column is of the given scalar type
    /// @param type the scalar type to check against
    /// @return true if this column is of the given scalar type
    public boolean is(ScalarType type) {
      return field.type.is(type);
    }

    /// Check if this column is of the given logical scalar type
    /// @param type the logical scalar type to check against
    /// @return true if this column is of the given logical scalar type
    public boolean is(LogicalScalarType type) {
      return field.type.is(type);
    }

    /// Check if this column is of the given complex type
    /// @param type the complex type to check against
    /// @return true if this column is of the given complex type
    public boolean is(ComplexType type) {
      return field.type.is(type);
    }

    /// Check if this column is of the given logical complex type
    /// @param type the logical complex type to check against
    /// @return true if this column is of the given logical complex type
    public boolean is(LogicalComplexType type) {
      return field.type.is(type);
    }

    /// Get the physical scalar type of this column, if applicable
    /// @return an Optional containing the physical scalar type of this column, or an empty Optional
    // if this column is not a scalar type
    public Optional<ScalarType> getScalarType() {
      return field.type.getScalarType();
    }

    /// Get the logical scalar type of this column, if applicable
    /// @return an Optional containing the logical scalar type of this column, or an empty Optional
    // if this column is not a scalar type
    public Optional<LogicalScalarType> getLogicalScalarType() {
      return field.type.getLogicalScalarType();
    }

    /// Get the physical complex type of this column, if applicable
    /// @return an Optional containing the physical complex type of this column, or an empty
    // Optional if this column is not a complex type
    public Optional<ComplexType> getComplexType() {
      return field.type.getComplexType();
    }

    /// Get the logical complex type of this column, if applicable
    /// @return an Optional containing the logical complex type of this column, or an empty Optional
    // if this column is not a complex type
    public Optional<LogicalComplexType> getLogicalComplexType() {
      return field.type.getLogicalComplexType();
    }

    /// Get the child fields of this column, if applicable (e.g., for STRUCT or MAP types)
    /// @return an Optional containing a SequencedCollection of child fields, or an empty Optional
    public Optional<SequencedCollection<Field>> getChildren() {
      return field.type.getChildren();
    }
  }

  /// A scalar field, which may be a physical type or a simple logical type (e.g., ID)
  public static final class ScalarField {
    /// The physical type of the field, if applicable.  Mutually exclusive with logicalType.
    public final ScalarType physicalType;
    /// The logical type of the field, if applicable.  Mutually exclusive with physicalType.
    public final LogicalScalarType logicalType;
    /// Whether the field has long ids (i.e., 64-bit integers) - only applicable for ID logical type
    public final boolean hasLongId;

    /// Create a scalar field with the given physical type
    /// @param type the physical type of the field
    public ScalarField(@NotNull ScalarType type) {
      Objects.requireNonNull(type);
      physicalType = type;
      logicalType = null;
      hasLongId = false;
    }

    /// Create a scalar field with the given logical type
    /// @param type the logical type of the field
    /// @param hasLongId whether the field has long ids (i.e., 64-bit integers) - only applicable
    // for ID logical type
    public ScalarField(@NotNull LogicalScalarType type, boolean hasLongId) {
      Objects.requireNonNull(type);
      physicalType = null;
      logicalType = type;
      this.hasLongId = hasLongId;
    }
  }

  /// A complex field
  public static final class ComplexField {
    /// The physical type of the field, if applicable.  Mutually exclusive with logicalType.
    public @Nullable ComplexType physicalType;
    /// The logical type of the field, if applicable.  Mutually exclusive with physicalType.
    public @Nullable LogicalComplexType logicalType;
    /// The child fields of this complex field, if applicable (e.g., for STRUCT or MAP types)
    public @NotNull SequencedCollection<Field> children;

    /// Create a complex field with the given physical type and no children
    /// @param type the physical type of the field
    public ComplexField(@NotNull ComplexType type) {
      this(type, new ArrayList<>());
    }

    /// Create a complex field with the given physical type and children
    /// @param type the physical type of the field
    /// @param children the child fields of this complex field, or null if no children
    public ComplexField(@NotNull ComplexType type, @Nullable SequencedCollection<Field> children) {
      Objects.requireNonNull(type);
      this.physicalType = type;
      this.children = (children != null) ? children : List.of();
    }
  }
}
