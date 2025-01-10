#include <gtest/gtest.h>

#include <protozero/pbf_message.hpp>

#include <filesystem>
#include <fstream>
#include <regex>
#include <vector>

namespace mlt::tileset_metadata {

// Based on maplibre-tile-spec:spec/schema/mlt_tileset_metadata.proto commit 4b92e986b5c5c1f4c3772a5cf7033fe5822deb5c
namespace schema {

enum class TileSetMetadata : protozero::pbf_tag_type {
    implicit_int32_version =  1,
    repeated_FeatureTableSchema_featureTables =  2,
    optional_string_name = 3,
    optional_string_description = 4,
    optional_string_attribution = 5,
    optional_int32_minZoom = 6,
    optional_int32_maxZoom = 7,
    repeated_double_bounds = 8, // order left, bottom, right, top in WGS84
    repeated_double_center = 9, // order longitude, latitude in WGS84
};

enum class FeatureTableSchema : protozero::pbf_tag_type {
    implicit_string_name = 1,
    repeated_Column_columns = 2,
};

// Column are top-level types in the schema
enum class Column : protozero::pbf_tag_type {
    implicit_string_name = 1,
    // specifies if the values are optional in the column and a present stream should be used
    implicit_bool_nullable = 2,
    implicit_ColumnScope_columnScope = 3,
 
    oneof_ScalarColumn_scalarType = 4,
    oneof_ComplexColumn_complexType = 5,
};

enum class ScalarColumn : protozero::pbf_tag_type {
    oneof_ScalarType_physicalType = 4,
    oneof_LogicalScalarType_logicalType = 5,
};

// The type tree is flattened in to a list via a pre-order traversal
// Represents a column if it is a root (top-level) type or a child of a nested type
enum class ComplexColumn : protozero::pbf_tag_type {
    oneof_ComplexType_physicalType = 4,
    oneof_LogicalComplexType_logicalType = 5,

    // The complex type Geometry and the logical type BINARY have no children since there layout is implicit known.
    // RangeMap has only one child specifying the type of the value since the key is always a vec2<double>.
    repeated_Field_children = 6,
};

// Fields define nested or leaf types in the schema as part of a complex type definition
enum class Field : protozero::pbf_tag_type {
    // name and nullable are only needed in combination with a struct not for vec, list and map
    // Map -> has the order key type, value type
    optional_string_name = 1,
    optional_bool_nullable = 2,

    oneof_ScalarField_scalarField = 3,
    oneof_ComplexField_complexField = 4,
};

enum class ScalarField : protozero::pbf_tag_type {
    oneof_ScalarType_physicalType = 1,
    oneof_LogicalScalarType_logicalType = 2,
};

enum class ComplexField : protozero::pbf_tag_type {
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
  // fixed size binary with 2 values of the same type either signed or unsigned Int8, Int32, Int64 as well as Float or Double
  VEC_2 = 0,
  // fixed size binary with 2 values of the same type either signed or unsigned Int8, Int32, Int64 as well as Float or Double
  VEC_3 = 1,
  // vec2<Int32> for the VertexBuffer stream with additional information (streams) about the topology
  GEOMETRY = 2,
  // vec3<Int32> for the VertexBuffer stream with additional information (streams) about the topology
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
  // physical type: map<vec2<double, T>> -> special data structure which can be used for a efficient representation of linear referencing
  RANGE_MAP = 1,
};

} // schema

bool readField(protozero::pbf_message<schema::Field>);

bool readScalarField(protozero::pbf_message<schema::ScalarField> message) {
    bool hasPhysical = false;
    bool hasLogical = false;

    while (message.next()) {
        switch (message.tag()) {
        case schema::ScalarField::oneof_ScalarType_physicalType:
            static_cast<schema::ScalarType>(message.get_enum());
            hasPhysical = true;
        case schema::ScalarField::oneof_LogicalScalarType_logicalType:
            static_cast<schema::LogicalScalarType>(message.get_enum());
            hasLogical = true;
        default:
            message.skip();
            break;
        }
    }
    return (hasLogical != hasPhysical);
}

bool readComplexField(protozero::pbf_message<schema::ComplexField> message) {
    bool hasPhysical = false;
    bool hasLogical = false;

    while (message.next()) {
        switch (message.tag()) {
        case schema::ComplexField::oneof_ComplexType_physicalType:
            static_cast<schema::ScalarType>(message.get_enum());
            hasPhysical = true;
        case schema::ComplexField::oneof_LogicalComplexType_logicalType:
            static_cast<schema::LogicalScalarType>(message.get_enum());
            hasLogical = true;
        case schema::ComplexField::repeated_Field_children:
            if (!readField(message.get_message())) {
                return false;
            }
            break;
        default:
            message.skip();
            break;
        }
    }
    return (hasLogical != hasPhysical);
}

bool readScalarColumn(protozero::pbf_message<schema::ScalarColumn> message) {
    bool hasPhysical = false;
    bool hasLogical = false;

    while (message.next()) {
        switch (message.tag()) {
        case schema::ScalarColumn::oneof_ScalarType_physicalType:
            static_cast<schema::ScalarType>(message.get_enum());
            hasPhysical = true;
            break;
        case schema::ScalarColumn::oneof_LogicalScalarType_logicalType:
            static_cast<schema::LogicalScalarType>(message.get_enum());
            hasLogical = true;
            break;
        default:
            message.skip();
            break;
        }
    }
    return (hasLogical != hasPhysical);
}

bool readField(protozero::pbf_message<schema::Field> message) {
    bool hasScalar = false;
    bool hasComplex = false;

    while (message.next()) {
        switch (message.tag()) {
        case schema::Field::optional_string_name:
            message.get_string();
            break;
        case schema::Field::optional_bool_nullable:
            message.get_bool();
            break;
        case schema::Field::oneof_ScalarField_scalarField:
            if (!readScalarField(message.get_message())) {
                return false;
            }
            hasScalar = true;
            break;
        case schema::Field::oneof_ComplexField_complexField:
            if (!readComplexField(message.get_message())) {
                return false;
            }
            hasComplex = true;
            break;
        default:
            message.skip();
            break;
        }
    }
    return (hasScalar != hasComplex);
}

bool readComplexColumn(protozero::pbf_message<schema::ComplexColumn> message) {
    bool hasPhysical = false;
    bool hasLogical = false;

    while (message.next()) {
        switch (message.tag()) {
        case schema::ComplexColumn::oneof_ComplexType_physicalType:
            static_cast<schema::ComplexType>(message.get_enum());
            hasPhysical = true;
            break;
        case schema::ComplexColumn::oneof_LogicalComplexType_logicalType:
            static_cast<schema::LogicalComplexType>(message.get_enum());
            hasLogical = true;
            break;
        case schema::ComplexColumn::repeated_Field_children:
            if (!readField(message.get_message())) {
                return false;
            }
            break;
        default:
            message.skip();
            break;
        }
    }
    return (hasLogical != hasPhysical);
}

bool readColumn(protozero::pbf_message<schema::Column> message) {
    bool hasName = false;
    bool hasScalar = false;
    bool hasComplex = false;

    while (message.next()) {
        switch (message.tag()) {
        case schema::Column::implicit_string_name:  // treated as required
            std::cerr << "  Column: " << message.get_string() << std::endl;
            hasName = true;
            break;
        case schema::Column::implicit_bool_nullable:
            std::cerr << "  Nullable: " << message.get_bool() << std::endl;
            break;
        case schema::Column::implicit_ColumnScope_columnScope:
            std::cerr << "  Scope: " << message.get_enum() << std::endl;
            //static_cast<ColumnScope>(message.get_enum())
            break;
        case schema::Column::oneof_ScalarColumn_scalarType:
            std::cerr << "  ScalarColumn" << std::endl;
            if (!readScalarColumn(message.get_message())) {
                return false;
            }
            hasScalar = true;
            break;
        case schema::Column::oneof_ComplexColumn_complexType:
            std::cerr << "  ComplexColumn" << std::endl;
            if (!readComplexColumn(message.get_message())) {
                return false;
            }
            hasComplex = true;
            break;
        default:
            message.skip();
            break;
        }
    }

    return hasName && (hasScalar != hasComplex);
}

bool readFeatureTableSchema(protozero::pbf_message<schema::FeatureTableSchema> message) {
    bool hasName = false;
    int columnCount = 0;

    while (message.next()) {
        switch (message.tag()) {
        case schema::FeatureTableSchema::implicit_string_name:  // treated as required
            std::cerr << " Table: " << message.get_string() << std::endl;
            hasName = true;
            break;
        case schema::FeatureTableSchema::repeated_Column_columns:
            if (!readColumn(message.get_message())) {
                return false;
            }
            columnCount++;
            break;
        default:
            message.skip();
            break;
        }
    }
    return hasName && (columnCount > 0);
}

bool readTileSetMetadata(protozero::pbf_message<schema::TileSetMetadata> message) {
    int featureTableCount = 0;

    while (message.next()) {
        switch (message.tag()) {
        case schema::TileSetMetadata::implicit_int32_version: {
            const auto version = message.get_int32();
            constexpr auto minVersion = 1;
            if (version < minVersion) {
                return false;
            }
            break;
        }
        case schema::TileSetMetadata::repeated_FeatureTableSchema_featureTables:
            if (!readFeatureTableSchema(message.get_message())) {
                return false;
            }
            featureTableCount++;
            break;
        case schema::TileSetMetadata::optional_string_name:
            message.get_string();
            break;
        case schema::TileSetMetadata::optional_string_description:
            message.get_string();
            break;
        case schema::TileSetMetadata::optional_string_attribution:
            message.get_string();
            break;
        case schema::TileSetMetadata::optional_int32_minZoom:
            message.get_int32();
            break;
        case schema::TileSetMetadata::optional_int32_maxZoom:
            message.get_int32();
            break;
        case schema::TileSetMetadata::repeated_double_bounds:
            message.get_double();
            break;
        case schema::TileSetMetadata::repeated_double_center:
            message.get_double();
            break;
        default:
            message.skip();
            break;
        }
    }
    return (featureTableCount > 0);
}

} // mlt::tileset_metadata

TEST(Decode, metadata) {
    constexpr auto path = "../test/expected/simple";
    const std::regex metadataFilePattern{".*\\.mlt.meta.pbf"};
    for (const auto& entry : std::filesystem::directory_iterator(path)) {
        std::smatch match;
        const auto fileName = entry.path().filename().string();
        if (!entry.is_regular_file() || !std::regex_match(fileName, match, metadataFilePattern)) {
            continue;
        }

        std::cerr << "Loading " << fileName << std::endl;

        std::ifstream file(entry.path(), std::ios::binary | std::ios::ate);
        EXPECT_TRUE(file.is_open());

        std::size_t size = file.tellg();
        file.seekg(0);

        std::vector<std::ifstream::char_type> buffer(size);
        EXPECT_TRUE(file.read(buffer.data(), size));

        EXPECT_TRUE(mlt::tileset_metadata::readTileSetMetadata({protozero::data_view{buffer.data(), buffer.size()}}));
    }
}

