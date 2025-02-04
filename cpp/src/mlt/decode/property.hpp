#pragma once

#include <mlt/decode/int.hpp>
#include <mlt/decode/string.hpp>
#include <mlt/feature.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/rle.hpp>
#include <mlt/util/raw.hpp>

#include <stdexcept>
#include <string>
#include <variant>
#include <vector>

namespace mlt::decoder {

namespace {

/// Use the "present" bitmask stream to expand a stream of values into a larger buffer.
/// The actual number of values present in a data stream is the number of bits set in the present stream.
/// We expand backward from the end so that we don't need to use a second buffer.
template <typename T>
void populateColumn(std::vector<T>& buffer, const std::vector<std::uint8_t>& presentStream, const count_t numValues) {
    if (presentStream.empty() || buffer.empty() || numValues <= buffer.size()) {
        return;
    }

    auto sourceIndex = buffer.size() - 1;
    auto targetIndex = numValues - 1;
    auto byteIndex = (numValues - 1) / 8;
    auto mask = 1 << ((numValues - 1) % 8);

    buffer.resize(numValues);
    for (auto value = presentStream[byteIndex];;) {
        if (value & mask) {
            buffer[targetIndex] = buffer[sourceIndex];
            if (sourceIndex == 0) {
                break;
            }
            --sourceIndex;
        }
        --targetIndex;

        mask >>= 1;
        if (!mask) {
            if (byteIndex) {
                mask = 1 << 7;
                value = presentStream[--byteIndex];
            } else {
                break;
            }
        }
    }
}
}

class PropertyDecoder {
public:
    PropertyDecoder(IntegerDecoder& intDecoder_, StringDecoder& stringDecoder_) noexcept(false)
        : intDecoder(intDecoder_),
          stringDecoder(stringDecoder_) {}

    PropertyVec decodePropertyColumn(BufferStream& tileData,
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
                if ((presentValueCount + 7) / 8 != presentStream.size()) {
                    throw std::runtime_error("invalid present stream");
                }
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
                default:
                case ScalarType::STRING:
                    break;
            }

            switch (scalarType) {
                case ScalarType::BOOLEAN: {
                    std::vector<std::uint8_t> byteBuffer;
                    rle::decodeBoolean(tileData, byteBuffer, *streamMetadata, /*consume=*/true);
                    if (streamMetadata->getNumValues() > 0 && (streamMetadata->getNumValues() + 7) / 8 != byteBuffer.size()) {
                        throw std::runtime_error("column data incomplete");
                    }
                    return byteBuffer;
                }
                case ScalarType::INT_8:
                case ScalarType::UINT_8:
                    throw std::runtime_error("8-bit integer type not implemented");
                case ScalarType::INT_32: {
                    std::vector<std::uint32_t> intBuffer;
                    intBuffer.reserve(streamMetadata->getNumValues());
                    intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::uint32_t, /*isSigned=*/true>(
                        tileData, intBuffer, *streamMetadata);
                    populateColumn(intBuffer, presentStream, presentValueCount);
                    return intBuffer;
                }
                case ScalarType::UINT_32: {
                    std::vector<std::uint32_t> intBuffer;
                    intBuffer.reserve(streamMetadata->getNumValues());
                    intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::uint32_t, /*isSigned=*/false>(
                        tileData, intBuffer, *streamMetadata);
                    populateColumn(intBuffer, presentStream, presentValueCount);
                    return intBuffer;
                }
                case ScalarType::INT_64: {
                    std::vector<std::uint64_t> longBuffer;
                    longBuffer.reserve(streamMetadata->getNumValues());
                    intDecoder.decodeIntStream<std::uint64_t, std::uint64_t, std::uint64_t, /*isSigned=*/true>(
                        tileData, longBuffer, *streamMetadata);
                    populateColumn(longBuffer, presentStream, presentValueCount);
                    return longBuffer;
                }
                case ScalarType::UINT_64: {
                    std::vector<std::uint64_t> longBuffer;
                    longBuffer.reserve(streamMetadata->getNumValues());
                    intDecoder.decodeIntStream<std::uint64_t, std::uint64_t, std::uint64_t, /*isSigned=*/false>(
                        tileData, longBuffer, *streamMetadata);
                    populateColumn(longBuffer, presentStream, presentValueCount);
                    return longBuffer;
                }
                case ScalarType::FLOAT: {
                    std::vector<float> floatBuffer;
                    floatBuffer.reserve(streamMetadata->getNumValues());
                    decodeRaw(tileData, floatBuffer, *streamMetadata, /*consume=*/true);
                    populateColumn(floatBuffer, presentStream, presentValueCount);
                    return floatBuffer;
                }
                case ScalarType::DOUBLE: {
                    std::vector<double> doubleBuffer;
                    doubleBuffer.reserve(streamMetadata->getNumValues());
                    decodeRaw(tileData, doubleBuffer, *streamMetadata, /*consume=*/true);
                    populateColumn(doubleBuffer, presentStream, presentValueCount);
                    return doubleBuffer;
                }
                case ScalarType::STRING: {
                    std::vector<std::string_view> strings;
                    std::vector<std::uint8_t> stringData;
                    stringDecoder.decode(tileData, numStreams - 1, presentValueCount, presentStream, stringData, strings);
                    return std::make_pair(std::move(stringData), std::move(strings));
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
