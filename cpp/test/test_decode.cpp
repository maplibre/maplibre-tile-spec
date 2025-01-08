#include <gtest/gtest.h>

#include <protozero/pbf_message.hpp>

#include <fstream>
#include <vector>

enum class TileSetMetadata : protozero::pbf_tag_type {
    required_int32_version =  1,
    repeated_featureTables =  2,
    optional_string_name = 3,
    optional_string_description = 4,
    optional_string_attribution = 5,
    optional_int32_minZoom = 6,
    optional_int32_maxZoom = 7,
    repeated_double_bounds = 8,
    repeated_double_center = 9,
};

// message FeatureTableSchema {
//   string name = 1;
//   repeated Column columns = 2;
// }
// message Column {
//   string name = 1;
//   // specifies if the values are optional in the column and a present stream should be used
//   bool nullable = 2;
//   ColumnScope columnScope = 3;
//   oneof type {
//     ScalarColumn scalarType = 4;
//     ComplexColumn complexType = 5;
//   }
// }

TEST(Decode, metadata) {
    std::ifstream file("../test/expected/simple/polygon-boolean.mlt.meta.pbf", std::ios::binary | std::ios::ate);
    EXPECT_TRUE(file.is_open());

    std::size_t size = file.tellg();
    file.seekg(0);

    std::vector<std::ifstream::char_type> buffer(size);
    EXPECT_TRUE(file.read(buffer.data(), size));

    protozero::pbf_message<TileSetMetadata> message{protozero::data_view{buffer.data(), buffer.size()}};

    while (message.next()) {
        switch (message.tag()) {
        case TileSetMetadata::required_int32_version:
            message.get_int32();
            break;
        case TileSetMetadata::repeated_featureTables:
            message.get_message();
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
}

