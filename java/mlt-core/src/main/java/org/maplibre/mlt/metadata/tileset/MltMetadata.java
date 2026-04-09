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

  public static final record FeatureTable(
      @NotNull String name, @NotNull SequencedCollection<Column> columns) {

    public FeatureTable {
      Objects.requireNonNull(name);
      columns = (columns != null) ? columns : new ArrayList<>();
    }

    public FeatureTable(String name) {
      this(name, null);
    }

    public FeatureTable(String name, int initialColumnCapacity) {
      this(name, new ArrayList<>(initialColumnCapacity));
    }
  }

  /// The type of data in a Field
  /// @param scalarType A scalar type, if set.  Mutually exclusive with complexType.
  /// @param complexType A complex type, if set.  Mutually exclusive with scalarType.
  /// @param isNullable Whether the field can be null
  public static final record FieldType(
      @Nullable ScalarField scalarType, @Nullable ComplexField complexType, boolean isNullable) {

    public FieldType {
      if ((scalarType == null) == (complexType == null)) {
        throw new IllegalStateException("FieldType must be either scalar or complex");
      }
    }

    /// Create a scalar type
    /// @param scalarType the type
    /// @param isNullable whether the field can be null
    public FieldType(@Nullable ScalarField scalarType, boolean isNullable) {
      this(scalarType, null, isNullable);
    }

    /// Create a complex type
    /// @param complexType the type
    /// @param isNullable whether the field can be null
    public FieldType(@Nullable ComplexField complexType, boolean isNullable) {
      this(null, complexType, isNullable);
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
    /// if this field is not a scalar type
    public Optional<ScalarType> getScalarType() {
      return (scalarType != null) ? Optional.ofNullable(scalarType.physicalType) : Optional.empty();
    }

    /// Get the logical scalar type of this field, if applicable
    /// @return an Optional containing the logical scalar type of this field, or an empty Optional
    /// if this field is not a scalar type
    public Optional<LogicalScalarType> getLogicalScalarType() {
      return (scalarType != null) ? Optional.ofNullable(scalarType.logicalType) : Optional.empty();
    }

    /// Get the physical complex type of this field, if applicable
    /// @return an Optional containing the physical complex type of this field, or an empty Optional
    /// if this field is not a complex type
    public Optional<ComplexType> getComplexType() {
      return (complexType != null)
          ? Optional.ofNullable(complexType.physicalType)
          : Optional.empty();
    }

    /// Get the logical complex type of this field, if applicable
    /// @return an Optional containing the logical complex type of this field, or an empty Optional
    /// if this field is not a complex type
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
  /// @param name The name of the field.  May be null for nested fields or implicit types (e.g, ID).
  /// @param type The type of the field.  Must be non-null.
  public static final record Field(@NotNull FieldType type, @Nullable String name) {
    public Field {
      Objects.requireNonNull(type);
    }

    public Field(@NotNull FieldType type) {
      this(type, null);
    }
  }

  /// Column are top-level types in the schema
  /// @param field The field associated with this column.  Must be non-null.
  /// @param columnScope The column scope, which must be either FEATURE or VERTEX
  public static final record Column(@NotNull Field field, @NotNull ColumnScope columnScope) {

    /// Create a column with the given field and column scope
    /// @param field the field associated with this column
    /// @param columnScope the column scope, which must be either FEATURE or VERTEX
    public Column {
      if (columnScope != ColumnScope.FEATURE && columnScope != ColumnScope.VERTEX) {
        throw new IllegalStateException("Column scope must be either FEATURE or VERTEX");
      }
    }

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
  /// @param physicalType The physical type, if applicable.  Mutually exclusive with logicalType
  /// @param logicalType The logical type, if applicable.  Mutually exclusive with physicalType
  /// @param hasLongId Whether the ID field has long ids (i.e., 64-bit integers)
  public static final record ScalarField(
      @Nullable ScalarType physicalType,
      @Nullable LogicalScalarType logicalType,
      boolean hasLongId) {

    public ScalarField {
      if ((physicalType == null) == (logicalType == null)) {
        throw new IllegalStateException(
            "ScalarField must have either a physical type or a logical type");
      }
    }

    /// Create a scalar field with the given physical type
    /// @param type the physical type of the field
    public ScalarField(@NotNull ScalarType type) {
      this(type, null, false);
    }

    /// Create a scalar field with the given logical type
    /// @param type the logical type of the field
    /// @param hasLongId whether the ID field has long ids (i.e., 64-bit integers)
    public ScalarField(@NotNull LogicalScalarType type, boolean hasLongId) {
      this(null, type, hasLongId);
    }
  }

  /// A complex field
  /// @param physicalType The physical type, if applicable.  Mutually exclusive with logicalType
  /// @param logicalType The logical type, if applicable.  Mutually exclusive with physicalType.
  /// @param children The child fields of this complex field, if applicable
  public static final record ComplexField(
      @Nullable ComplexType physicalType,
      @Nullable LogicalComplexType logicalType,
      @NotNull SequencedCollection<Field> children) {

    public ComplexField {
      if ((physicalType == null) == (logicalType == null)) {
        throw new IllegalStateException(
            "ComplexField must have either a physical type or a logical type");
      }
      children = (children != null) ? children : new ArrayList<>();
    }

    /// Create a complex field with the given physical type and no children
    /// @param type the physical type of the field
    public ComplexField(@NotNull ComplexType type) {
      this(type, null);
    }

    /// Create a complex field with the given physical type and children
    /// @param type the physical type of the field
    /// @param children the child fields of this complex field, or null if no children
    public ComplexField(@NotNull ComplexType type, @Nullable SequencedCollection<Field> children) {
      this(type, null, children);
    }
  }
}
