#pragma once

#if MLT_WITH_PROTOZERO
#include <protozero/pbf_message.hpp>
#endif

#include <optional>
#include <variant>
#include <vector>

namespace mlt::metadata::tileset {

// Based on maplibre-tile-spec:spec/schema/mlt_tileset_metadata.proto commit
// 4b92e986b5c5c1f4c3772a5cf7033fe5822deb5c
namespace schema {

enum class TileSetMetadata : std::uint32_t {
    implicit_int32_version = 1,
    repeated_FeatureTableSchema_featureTables = 2,
    optional_string_name = 3,
    optional_string_description = 4,
    optional_string_attribution = 5,
    optional_int32_minZoom = 6,
    optional_int32_maxZoom = 7,
    repeated_double_bounds = 8, // order left, bottom, right, top in WGS84
    repeated_double_center = 9, // order longitude, latitude in WGS84
};

enum class FeatureTableSchema : std::uint32_t {
    implicit_string_name = 1,
    repeated_Column_columns = 2,
};

// Column are top-level types in the schema
enum class Column : std::uint32_t {
    implicit_string_name = 1,
    // specifies if the values are optional in the column and a present stream
    // should be used
    implicit_bool_nullable = 2,
    implicit_ColumnScope_columnScope = 3,

    oneof_ScalarColumn_scalarType = 4,
    oneof_ComplexColumn_complexType = 5,
};

enum class ScalarColumn : std::uint32_t {
    oneof_ScalarType_physicalType = 4,
    oneof_LogicalScalarType_logicalType = 5,
};

// The type tree is flattened in to a list via a pre-order traversal
// Represents a column if it is a root (top-level) type or a child of a nested
// type
enum class ComplexColumn : std::uint32_t {
    oneof_ComplexType_physicalType = 4,
    oneof_LogicalComplexType_logicalType = 5,

    // The complex type Geometry and the logical type BINARY have no children
    // since there layout is implicit known. RangeMap has only one child
    // specifying the type of the value since the key is always a vec2<double>.
    repeated_Field_children = 6,
};

// Fields define nested or leaf types in the schema as part of a complex type
// definition
enum class Field : std::uint32_t {
    // name and nullable are only needed in combination with a struct not for vec,
    // list and map
    // Map -> has the order key type, value type
    optional_string_name = 1,
    optional_bool_nullable = 2,

    oneof_ScalarField_scalarField = 3,
    oneof_ComplexField_complexField = 4,
};

enum class ScalarField : std::uint32_t {
    oneof_ScalarType_physicalType = 1,
    oneof_LogicalScalarType_logicalType = 2,
};

enum class ComplexField : std::uint32_t {
    oneof_ComplexType_physicalType = 1,
    oneof_LogicalComplexType_logicalType = 2,
    repeated_Field_children = 3,
};

enum class ColumnScope {
    // 1:1 Mapping of property and feature -> id and geometry
    FEATURE = 0,
    // For M-Values -> 1:1 Mapping for property and vertex
    VERTEX = 1,
};

enum class ScalarType {
    BOOLEAN = 0,
    INT_8 = 1,
    UINT_8 = 2,
    INT_32 = 3,
    UINT_32 = 4,
    INT_64 = 5,
    UINT_64 = 6,
    FLOAT = 7,
    DOUBLE = 8,
    STRING = 9,
};

enum class ComplexType {
    // fixed size binary with 2 values of the same type either signed or unsigned
    // Int8, Int32, Int64 as well as Float or Double
    VEC_2 = 0,
    // fixed size binary with 2 values of the same type either signed or unsigned
    // Int8, Int32, Int64 as well as Float or Double
    VEC_3 = 1,
    // vec2<Int32> for the VertexBuffer stream with additional information
    // (streams) about the topology
    GEOMETRY = 2,
    // vec3<Int32> for the VertexBuffer stream with additional information
    // (streams) about the topology
    GEOMETRY_Z = 3,
    LIST = 4,
    MAP = 5,
    STRUCT = 6,
};

enum class LogicalScalarType {
    // physical type: Int64 -> number of milliseconds since Unix epoch
    TIMESTAMP = 0,
    // physical type: Int32 -> number of days since Unix epoch
    DATE = 1,
    // physical type: String
    JSON = 2,
};

enum class LogicalComplexType {
    // physical type: list<UInt8>
    BINARY = 0,
    // physical type: map<vec2<double, T>> -> special data structure which can be
    // used for a efficient representation of linear referencing
    RANGE_MAP = 1,
};

} // namespace schema

using schema::ColumnScope;
using schema::ComplexType;
using schema::LogicalComplexType;
using schema::LogicalScalarType;
using schema::ScalarType;

enum class GeometryType {
    POINT = 0,
    LINESTRING = 1,
    POLYGON = 2,
    MULTIPOINT = 3,
    MULTILINESTRING = 4,
    MULTIPOLYGON = 5,
};

struct ScalarField {
    std::variant<ScalarType, LogicalScalarType> type;
};

struct Field;

struct ComplexField {
    std::variant<ComplexType, LogicalComplexType> type;
    std::vector<Field> children;
};

// Fields define nested or leaf types in the schema as part of a complex type
// definition
struct Field {
    // name and nullable are only needed in combination with a struct not for vec,
    // list and map Map -> has the order key type, value type
    std::string name;
    bool nullable = false;
    std::variant<ScalarField, ComplexField> field;
};

struct ScalarColumn {
    std::variant<ScalarType, LogicalScalarType> type;
};

// The type tree is flattened in to a list via a pre-order traversal
// Represents a column if it is a root (top-level) type or a child of a nested
// type
struct ComplexColumn {
    std::variant<ComplexType, LogicalComplexType> type;

    // The complex type Geometry and the logical type BINARY have no children
    // since there layout is implicit known. RangeMap has only one child
    // specifying the type of the value since the key is always a vec2<double>.
    std::vector<Field> children;
};

// Column are top-level types in the schema
struct Column {
    std::string name;
    bool nullable = false;
    ColumnScope columnScope = ColumnScope::FEATURE;
    std::variant<ScalarColumn, ComplexColumn> type;
};

struct FeatureTableSchema {
    std::string name;
    std::vector<Column> columns;
};

struct TileSetMetadata {
    std::int32_t version = 0;
    std::vector<FeatureTableSchema> featureTables;
    std::string name;
    std::string description;
    std::string attribution;
    std::optional<std::int32_t> minZoom;
    std::optional<std::int32_t> maxZoom;
    std::optional<double> boundLeft;
    std::optional<double> boundBottom;
    std::optional<double> boundRight;
    std::optional<double> boundTop;
    std::optional<double> centerLon;
    std::optional<double> centerLat;
};

#if MLT_WITH_PROTOZERO
/// Decode the tileset metadata from a protobuf message
/// @throws protobuf::exception corrupted/truncated input data
std::optional<TileSetMetadata> read(protozero::pbf_message<schema::TileSetMetadata>);
#endif

} // namespace mlt::metadata::tileset
