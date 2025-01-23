#pragma once

#include <feature.hpp>
#include <metadata/stream.hpp>
#include <metadata/tileset.hpp>
#include <util/buffer_stream.hpp>
#include <util/rle.hpp>

#include <string>
#include <variant>

namespace mlt::decoder {

class PropertyDecoder {
public:
    using PropertyMap = Feature::PropertyMap;

    static PropertyMap decodePropertyColumn(BufferStream& tileData,
                                            const metadata::tileset::Column& column,
                                            std::uint32_t numStreams) {
        using namespace util::decoding;

        if (std::holds_alternative<metadata::tileset::ScalarColumn>(column.type)) {
            if (numStreams > 1) {
                const auto presentStreamMetadata = metadata::stream::decode(tileData);
                // values ignored, do not decode
                tileData.consume(presentStreamMetadata->getByteLength());
            }

            const auto scalarColumn = std::get<metadata::tileset::ScalarColumn>(column.type);
            if (!std::holds_alternative<metadata::tileset::schema::ScalarType>(scalarColumn.type)) {
                throw std::runtime_error("property column ('" + column.name + "') must be scalar and physical");
            }
            const auto scalarType = std::get<metadata::tileset::schema::ScalarType>(scalarColumn.type);

            // Everything but string has a metadata
            std::unique_ptr<metadata::stream::StreamMetadata> streamMetadata;
            switch (scalarType) {
                case metadata::tileset::schema::ScalarType::BOOLEAN:
                case metadata::tileset::schema::ScalarType::INT_8:
                case metadata::tileset::schema::ScalarType::UINT_8:
                case metadata::tileset::schema::ScalarType::INT_32:
                case metadata::tileset::schema::ScalarType::UINT_32:
                case metadata::tileset::schema::ScalarType::INT_64:
                case metadata::tileset::schema::ScalarType::UINT_64:
                case metadata::tileset::schema::ScalarType::FLOAT:
                case metadata::tileset::schema::ScalarType::DOUBLE:
                    streamMetadata = metadata::stream::decode(tileData);
                    break;
                case metadata::tileset::schema::ScalarType::STRING:
                    break;
            }

            std::vector<std::uint8_t> buffer;
            switch (scalarType) {
                case metadata::tileset::schema::ScalarType::BOOLEAN: {
                    rle::decodeBoolean(tileData, buffer, *streamMetadata, /*consume=*/true);
                    break;
                }
                case metadata::tileset::schema::ScalarType::INT_8:
                case metadata::tileset::schema::ScalarType::UINT_8:
                    throw std::runtime_error("8-bit integer type not implemented");
                case metadata::tileset::schema::ScalarType::INT_32:
                case metadata::tileset::schema::ScalarType::UINT_32:
                    throw std::runtime_error("32-bit integer type not implemented");
                case metadata::tileset::schema::ScalarType::INT_64:
                case metadata::tileset::schema::ScalarType::UINT_64:
                    throw std::runtime_error("64-bit integer type not implemented");
                case metadata::tileset::schema::ScalarType::FLOAT:
                case metadata::tileset::schema::ScalarType::DOUBLE:
                    throw std::runtime_error("Floating point type not implemented");
                case metadata::tileset::schema::ScalarType::STRING:
                    throw std::runtime_error("String type not implemented");
                default:
                    throw std::runtime_error("Unknown scalar type: " + std::to_string(std::to_underlying(scalarType)));
            }
        }
        return {};
    }
};

} // namespace mlt::decoder
