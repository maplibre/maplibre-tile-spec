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
    protected FieldType(
        @Nullable ScalarField scalar, @Nullable ComplexField complex, boolean isNullable) {
      this.scalarType = scalar;
      this.complexType = complex;
      this.isNullable = isNullable;
    }

    public final boolean isNullable;
    public final @Nullable ComplexField complexType;
    public final @Nullable ScalarField scalarType;
  }

  public static class Field extends FieldType {
    public final String name;

    protected Field(
        @Nullable String name,
        @Nullable ScalarField scalar,
        @Nullable ComplexField complex,
        boolean isNullable) {
      super(scalar, complex, isNullable);
      this.name = name;
    }

    protected Field(@NotNull Field other) {
      super(other.scalarType, other.complexType, other.isNullable);
      this.name = other.name;
    }

    public FieldBuilder asFieldBuilder() {
      return new FieldBuilder()
          .name(this.name)
          .nullable(this.isNullable)
          .scalar(this.scalarType)
          .complex(this.complexType);
    }

    public static class BuilderRoot {}

    @SuppressWarnings("unchecked")
    public static class BuilderBase<B extends BuilderRoot> extends BuilderRoot {
      private @Nullable String name;
      private @Nullable ScalarField scalarField;
      private @Nullable ComplexField complexField;
      private boolean isNullable = false;

      private BuilderBase() {}

      public B name(@Nullable String name) {
        this.name = name;
        return (B) this;
      }

      public B nullable(boolean nullable) {
        this.isNullable = nullable;
        return (B) this;
      }

      public B scalar(@Nullable ScalarField scalarField) {
        this.scalarField = scalarField;
        return (B) this;
      }

      public B scalar(@Nullable ScalarType type) {
        return scalar(new ScalarField(type));
      }

      public B id(boolean hasLongId) {
        return scalar(new ScalarField(LogicalScalarType.ID, hasLongId));
      }

      public B geometry() {
        return complex(new ComplexField(ComplexType.GEOMETRY));
      }

      public B complex(@Nullable ComplexField complexField) {
        this.complexField = complexField;
        return (B) this;
      }

      public B complex(@Nullable ComplexType type) {
        return complex(new ComplexField(type));
      }

      public B struct() {
        return struct(null);
      }

      public B struct(@Nullable List<Field> children) {
        return complex(
            new ComplexField(
                ComplexType.STRUCT, (children != null) ? children : new ArrayList<>()));
      }

      public Field build() {
        if (scalarField == null && complexField == null) {
          throw new IllegalStateException(
              "Either scalar or complex type must be provided for Field");
        }
        if (scalarField != null && complexField != null) {
          throw new IllegalStateException("Field cannot have both scalar and complex types");
        }
        return new Field(this.name, this.scalarField, this.complexField, this.isNullable);
      }
    }

    public static class FieldBuilder extends Field.BuilderBase<FieldBuilder> {
      private FieldBuilder() {}
    }
  }

  /** Column are top-level types in the schema */
  public static final class Column extends Field {
    public Column(@NotNull Field field, @NotNull MltMetadata.ColumnScope scope) {
      super(field);
      this.columnScope = scope;
    }

    public final @NotNull ColumnScope columnScope;

    public ColumnBuilder asColumnBuilder() {
      return new ColumnBuilder()
          .name(this.name)
          .nullable(this.isNullable)
          .scalar(this.scalarType)
          .complex(this.complexType)
          .scope(this.columnScope);
    }

    public static final class ColumnBuilder extends BuilderBase<ColumnBuilder> {
      private @NotNull ColumnScope columnScope = MltMetadata.ColumnScope.FEATURE;

      private ColumnBuilder() {}

      public ColumnBuilder scope(@NotNull ColumnScope scope) {
        this.columnScope = scope;
        return this;
      }

      public Column build() {
        Objects.requireNonNull(this.columnScope);
        if (this.columnScope != ColumnScope.FEATURE && this.columnScope != ColumnScope.VERTEX) {
          throw new IllegalStateException("Column scope must be either FEATURE or VERTEX");
        }
        return new Column(super.build(), this.columnScope);
      }
    }
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

    public ComplexField(@NotNull ComplexType type, @Nullable List<Field> children) {
      Objects.requireNonNull(type);
      this.physicalType = type;
      this.children = (children != null) ? children : new ArrayList<>();
    }

    public @Nullable ComplexType physicalType;
    public @Nullable LogicalComplexType logicalType;

    public @NotNull List<Field> children;
  }

  public static @NotNull Field.FieldBuilder fieldBuilder() {
    return new Field.FieldBuilder();
  }

  public static @NotNull Column.ColumnBuilder columnBuilder() {
    return new Column.ColumnBuilder();
  }
}
