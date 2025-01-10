#include <gtest/gtest.h>

#include <protozero/pbf_message.hpp>

#include <filesystem>
#include <fstream>
#include <regex>
#include <vector>

enum class TileSetMetadata : protozero::pbf_tag_type {
    implicit_int32_version =  1,
    repeated_FeatureTableSchema_featureTables =  2,
    optional_string_name = 3,
    optional_string_description = 4,
    optional_string_attribution = 5,
    optional_int32_minZoom = 6,
    optional_int32_maxZoom = 7,
    repeated_double_bounds = 8,
    repeated_double_center = 9,
};

enum class FeatureTableSchema : protozero::pbf_tag_type {
    implicit_string_name = 1,
    repeated_Column_columns = 2,
};

enum class Column : protozero::pbf_tag_type {
    implicit_string_name = 1,
    implicit_bool_nullable = 2,
    implicit_ColumnScope_columnScope = 3,
 
    // oneof_type
    optional_ScalarColumn_scalarType = 4,
    optional_ComplexColumn_complexType = 5,
};

bool readColumn(protozero::pbf_message<Column> message) {
    bool hasName = false;
    bool hasScalar = false;
    bool hasComplex = false;

    while (message.next()) {
        switch (message.tag()) {
        case Column::implicit_string_name:  // treated as required
            std::cerr << "  Column: " << message.get_string() << std::endl;
            hasName = true;
            break;
        case Column::implicit_bool_nullable:
            std::cerr << "  Nullable: " << message.get_bool() << std::endl;
            break;
        case Column::implicit_ColumnScope_columnScope:
            std::cerr << "  ColumnScope" << std::endl;
            message.get_message();
            break;
        case Column::optional_ScalarColumn_scalarType:
            std::cerr << "  ScalarColumn" << std::endl;
            message.get_message();
            hasScalar = true;
            break;
        case Column::optional_ComplexColumn_complexType:
            std::cerr << "  ComplexColumn" << std::endl;
            message.get_message();
            hasComplex = true;
            break;
        default:
            message.skip();
            break;
        }
    }

    return hasName && (hasScalar != hasComplex);
}

bool readFeatureTableSchema(protozero::pbf_message<FeatureTableSchema> message) {
    bool hasName = false;
    int columnCount = 0;

    while (message.next()) {
        switch (message.tag()) {
        case FeatureTableSchema::implicit_string_name:  // treated as required
            std::cerr << " Table: " << message.get_string() << std::endl;
            hasName = true;
            break;
        case FeatureTableSchema::repeated_Column_columns:
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

bool readTileSetMetadata(protozero::pbf_message<TileSetMetadata> message) {
    int featureTableCount = 0;

    while (message.next()) {
        switch (message.tag()) {
        case TileSetMetadata::implicit_int32_version: {
            const auto version = message.get_int32();
            constexpr auto minVersion = 1;
            if (version < minVersion) {
                return false;
            }
            break;
        }
        case TileSetMetadata::repeated_FeatureTableSchema_featureTables:
            if (!readFeatureTableSchema(message.get_message())) {
                return false;
            }
            featureTableCount++;
            break;
        case TileSetMetadata::optional_string_name:
            message.get_string();
            break;
        case TileSetMetadata::optional_string_description:
            message.get_string();
            break;
        case TileSetMetadata::optional_string_attribution:
            message.get_string();
            break;
        case TileSetMetadata::optional_int32_minZoom:
            message.get_int32();
            break;
        case TileSetMetadata::optional_int32_maxZoom:
            message.get_int32();
            break;
        case TileSetMetadata::repeated_double_bounds:
            message.get_double();
            break;
        case TileSetMetadata::repeated_double_center:
            message.get_double();
            break;
        default:
            message.skip();
            break;
        }
    }
    return (featureTableCount > 0);
}

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

        EXPECT_TRUE(readTileSetMetadata({protozero::data_view{buffer.data(), buffer.size()}}));
    }
}

