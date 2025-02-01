#pragma once

#include <mlt/decode/int.hpp>
#include <mlt/decode/string.hpp>
#include <mlt/feature.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/rle.hpp>
#include <mlt/util/raw.hpp>

#include <string>
#include <variant>
#include <vector>

namespace mlt::decoder {

class PropertyDecoder {
public:
    PropertyDecoder(IntegerDecoder& intDecoder_, StringDecoder& stringDecoder_) noexcept(false)
        : intDecoder(intDecoder_),
          stringDecoder(stringDecoder_) {}

    Feature::Property decodePropertyColumn(BufferStream& tileData,
                                           const metadata::tileset::Column& column,
                                           std::uint32_t numStreams) noexcept(false) {
        using namespace metadata;
        using namespace metadata::stream;
        using namespace metadata::tileset;
        using namespace util::decoding;

        if (std::holds_alternative<ScalarColumn>(column.type)) {
            std::vector<std::uint8_t> presentStream;
            count_t presentValueCount = 0;
            if (numStreams > 1) {
                const auto presentStreamMetadata = StreamMetadata::decode(tileData);
                presentValueCount = presentStreamMetadata->getNumValues();
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

            switch (scalarType) {
                case ScalarType::BOOLEAN: {
                    std::vector<std::uint8_t> byteBuffer;
                    rle::decodeBoolean(tileData, byteBuffer, *streamMetadata, /*consume=*/true);
                    break;
                }
                case ScalarType::INT_8:
                case ScalarType::UINT_8:
                    throw std::runtime_error("8-bit integer type not implemented");
                case ScalarType::INT_32: {
                    std::vector<std::uint32_t> intBuffer;
                    intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::uint32_t, /*isSigned=*/true>(
                        tileData, intBuffer, *streamMetadata);
                    return intBuffer;
                }
                case ScalarType::UINT_32: {
                    std::vector<std::uint32_t> intBuffer;
                    intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::uint32_t, /*isSigned=*/false>(
                        tileData, intBuffer, *streamMetadata);
                    return intBuffer;
                }
                case ScalarType::INT_64: {
                    std::vector<std::uint64_t> longBuffer;
                    intDecoder.decodeIntStream<std::uint64_t, std::uint64_t, std::uint64_t, /*isSigned=*/true>(
                        tileData, longBuffer, *streamMetadata);
                    return longBuffer;
                }
                case ScalarType::UINT_64: {
                    std::vector<std::uint64_t> longBuffer;
                    intDecoder.decodeIntStream<std::uint64_t, std::uint64_t, std::uint64_t, /*isSigned=*/false>(
                        tileData, longBuffer, *streamMetadata);
                    return longBuffer;
                }
                case ScalarType::FLOAT: {
                    std::vector<float> floatBuffer;
                    decodeRaw(tileData, floatBuffer, *streamMetadata, /*consume=*/true);
                    return floatBuffer;
                }
                case ScalarType::DOUBLE: {
                    std::vector<double> doubleBuffer;
                    decodeRaw(tileData, doubleBuffer, *streamMetadata, /*consume=*/true);
                    return doubleBuffer;
                }
                case ScalarType::STRING: {
                    std::vector<std::string> strings;
                    stringDecoder.decode(tileData, numStreams - 1, presentValueCount, presentStream, strings);
                    return strings;
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
