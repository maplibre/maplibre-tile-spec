
A feature table is a named collection of columns.

```
FeatureTableSchema {
  name: string
  columnCount: varint
  columns: Column[columnCount]
}
```

A column is either scalar or complex, and represents the values of some property for each feature or vertex.

```
Column {
  name: string
  options: ColumnOptions (u8)
  variant {
    scalarType: ScalarColumn
    complexType: ComplexColumn
  }
}

flags ColumnOptions {
  nullable = 1
  complex = 2
  vertexScope = 4 // For M-Values, a 1:1 Mapping for property and vertex
                  // otherwise, a 1:1 Mapping of property and feature -> id and geometry
}
```

A scalar column represents a collection of a scalar type.

```
ScalarColumn {
  options: ScalarColumnOptions
  variant {
    physicalType: ScalarType
    logicalType: LogicalScalarType
  }
}

flags ScalarColumnOptions {
  logical = 1
}
```

A complex column represents a collection of a nested type described by a tree of `Field`s.

The complex type tree is flattened in to a list via a pre-order traversal.  An instance represents a column if it is a root (top-level) type or, otherwise, a child of a nested type.

The complex type `Geometry` and the logical type `BINARY` have no children since there layout is implicit.

`RangeMap` has only one child specifying the type of the value since the key is always a `vec2<double>`.

```
ComplexColumn {
  options: ComplexColumnOptions
  variant {
    physicalType: ComplexType
    logicalType: LogicalComplexType
  }
  childrenCount: varint
  children: Field[childrenCount]
}

flags ComplexColumnOptions {
  logical = 1
}
```

Fields define nested or leaf types in the schema as part of a complex type definition.

Name and nullable are only used in combination with a struct not for vec, list and map

Map has the order key type, value type

```
Field {
  options: FieldOptions (u8)
  name: string?
  variant {
    scalarField: ScalarField
    complexField: ComplexField
  }
}

flags FieldOptions {
  named = 1
  nullable = 2
  complex = 4
}
```

A scalar field contains non-nested types.

```
ScalarField {
  options: ScalarFieldOptions (u8)
  variant {
    physicalType: ScalarType
    logicalType: LogicalScalarType
  }
}

flags ScalarFieldOptions {
  logical = 1
}

enum ScalarType {
  BOOLEAN = 0
  INT_8 = 1
  UINT_8 = 2
  INT_32 = 3
  UINT_32 = 4
  INT_64 = 5
  UINT_64 = 6
  FLOAT = 7
  DOUBLE = 8
  STRING = 9
  INT_128 = 10
  UINT_128 = 11
}

enum LogicalScalarType {
  TIMESTAMP = 0 // i64 number of milliseconds since Unix epoch
  DATE = 1  // i32 number of days since Unix epoch
  JSON = 2  // string
}

```

A complex field contains nested types.

```
ComplexField {
  options: ComplexFieldOptions
  variant {
    physicalType: ComplexType
    logicalType: LogicalComplexType
  }
  childCount: varint
  children: Field[childCount]
}

flags ComplexFieldOptions {
  logical = 1
}

enum ComplexType {
  VEC_2 = 0      // pair of any one of signed or unsigned 8-, 32-, 64-bit integers, Float or Double
  VEC_3 = 1      // triple of any one of the same
  GEOMETRY = 2   // vec2<i32> for the VertexBuffer stream with additional streams about the topology
  GEOMETRY_Z = 3 // vec3<i32> for the VertexBuffer stream with additional streams about the topology
  LIST = 4
  MAP = 5
  STRUCT = 6
}

enum LogicalComplexType {
  BINARY = 0    // vec<u8>
  RANGE_MAP = 1 // vec2<double> -> T
}
```
