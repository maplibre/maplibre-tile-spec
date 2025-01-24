#pragma once

#include <decode/int.hpp>
#include <decode/string.hpp>
#include <feature.hpp>
#include <metadata/stream.hpp>
#include <metadata/tileset.hpp>
#include <util/buffer_stream.hpp>
#include <util/rle.hpp>
#include <util/raw.hpp>

#include <string>
#include <variant>
#include <vector>

namespace mlt::decoder {

class PropertyDecoder {
public:
    using PropertyMap = Feature::PropertyMap;

    PropertyDecoder(IntegerDecoder& intDecoder_, StringDecoder& stringDecoder_) noexcept(false)
        : intDecoder(intDecoder_),
          stringDecoder(stringDecoder_) {}

    PropertyMap decodePropertyColumn(BufferStream& tileData,
                                     const metadata::tileset::Column& column,
                                     std::uint32_t numStreams) noexcept(false) {
        using namespace metadata;
        using namespace metadata::stream;
        using namespace metadata::tileset;
        using namespace util::decoding;

        if (std::holds_alternative<ScalarColumn>(column.type)) {
            std::vector<std::uint8_t> presentStream;
            if (numStreams > 1) {
                const auto presentStreamMetadata = StreamMetadata::decode(tileData);
                rle::decodeBoolean(tileData, presentStream, *presentStreamMetadata, /*consume=*/true);
            }

            const auto scalarColumn = std::get<ScalarColumn>(column.type);
            if (!std::holds_alternative<ScalarType>(scalarColumn.type)) {
                throw std::runtime_error("property column ('" + column.name + "') must be scalar and physical");
            }
            const auto scalarType = std::get<ScalarType>(scalarColumn.type);

            // Everything but string has a metadata
            std::unique_ptr<StreamMetadata> streamMetadata;
            switch (scalarType) {
                case ScalarType::BOOLEAN:
                case ScalarType::INT_8:
                case ScalarType::UINT_8:
                case ScalarType::INT_32:
                case ScalarType::UINT_32:
                case ScalarType::INT_64:
                case ScalarType::UINT_64:
                case ScalarType::FLOAT:
                case ScalarType::DOUBLE:
                    streamMetadata = StreamMetadata::decode(tileData);
                    break;
                case ScalarType::STRING:
                    break;
            }

            std::vector<std::uint8_t> byteBuffer;
            std::vector<std::uint32_t> intBuffer;
            std::vector<std::uint64_t> longBuffer;
            std::vector<float> floatBuffer;
            std::vector<double> doubleBuffer;
            switch (scalarType) {
                case ScalarType::BOOLEAN: {
                    rle::decodeBoolean(tileData, byteBuffer, *streamMetadata, /*consume=*/true);
                    byteBuffer.clear();
                    break;
                }
                case ScalarType::INT_8:
                case ScalarType::UINT_8:
                    throw std::runtime_error("8-bit integer type not implemented");
                case ScalarType::INT_32:
                case ScalarType::UINT_32: {
                    const bool isSigned = (scalarType == ScalarType::INT_32);
                    intDecoder.decodeIntStream<std::uint32_t>(tileData, intBuffer, *streamMetadata, isSigned);
                    intBuffer.clear();
                    break;
                }
                case ScalarType::INT_64:
                case ScalarType::UINT_64: {
                    const bool isSigned = (scalarType == ScalarType::INT_64);
                    intDecoder.decodeIntStream<std::uint64_t>(tileData, longBuffer, *streamMetadata, isSigned);
                    longBuffer.clear();
                }
                case ScalarType::FLOAT:
                    decodeRaw(tileData, floatBuffer, *streamMetadata, /*consume=*/true);
                    floatBuffer.clear();
                    break;
                case ScalarType::DOUBLE:
                    decodeRaw(tileData, doubleBuffer, *streamMetadata, /*consume=*/true);
                    doubleBuffer.clear();
                    break;
                case ScalarType::STRING: {
                    std::vector<std::string> strings;
                    stringDecoder.decode(tileData, numStreams, presentStream, streamMetadata->getNumValues(), strings);
                    break;
                }
                default:
                    throw std::runtime_error("Unknown scalar type: " + std::to_string(std::to_underlying(scalarType)));
            }
        }
        return {};
    }

private:
    IntegerDecoder& intDecoder;
    StringDecoder& stringDecoder;
};

} // namespace mlt::decoder
