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
#include "mlt/properties.hpp"

namespace mlt::decoder {

class PropertyDecoder {
public:
    PropertyDecoder(IntegerDecoder& intDecoder_, StringDecoder& stringDecoder_) noexcept(false)
        : intDecoder(intDecoder_),
          stringDecoder(stringDecoder_) {}

    PresentProperties decodePropertyColumn(BufferStream& tileData,
                                           const metadata::tileset::Column& column,
                                           std::uint32_t numStreams) noexcept(false) {
        using namespace metadata;
        using namespace metadata::stream;
        using namespace metadata::tileset;
        using namespace util::decoding;

        if (!std::holds_alternative<ScalarColumn>(column.type)) {
            throw std::runtime_error("Missing property type");
        }

        PackedBitset presentStream;
        count_t presentValueCount = 0;

        if ((numStreams > 1) != column.nullable) {
            throw std::runtime_error("Invalid stream count");
        }
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
                if (streamMetadata->getNumValues() > 0 &&
                    (streamMetadata->getNumValues() + 7) / 8 != byteBuffer.size()) {
                    throw std::runtime_error("column data incomplete");
                }
                return {byteBuffer, presentStream};
            }
            case ScalarType::INT_8:
            case ScalarType::UINT_8:
                throw std::runtime_error("8-bit integer type not implemented");
            case ScalarType::INT_32: {
                std::vector<std::uint32_t> intBuffer;
                intBuffer.reserve(streamMetadata->getNumValues());
                intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::uint32_t, /*isSigned=*/true>(
                    tileData, intBuffer, *streamMetadata);
                return {std::move(intBuffer), std::move(presentStream)};
            }
            case ScalarType::UINT_32: {
                std::vector<std::uint32_t> intBuffer;
                intBuffer.reserve(streamMetadata->getNumValues());
                intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::uint32_t, /*isSigned=*/false>(
                    tileData, intBuffer, *streamMetadata);
                return {std::move(intBuffer), std::move(presentStream)};
            }
            case ScalarType::INT_64: {
                std::vector<std::uint64_t> longBuffer;
                longBuffer.reserve(streamMetadata->getNumValues());
                intDecoder.decodeIntStream<std::uint64_t, std::uint64_t, std::uint64_t, /*isSigned=*/true>(
                    tileData, longBuffer, *streamMetadata);
                return {std::move(longBuffer), std::move(presentStream)};
            }
            case ScalarType::UINT_64: {
                std::vector<std::uint64_t> longBuffer;
                longBuffer.reserve(streamMetadata->getNumValues());
                intDecoder.decodeIntStream<std::uint64_t, std::uint64_t, std::uint64_t, /*isSigned=*/false>(
                    tileData, longBuffer, *streamMetadata);
                return {std::move(longBuffer), std::move(presentStream)};
            }
            case ScalarType::FLOAT: {
                std::vector<float> floatBuffer;
                floatBuffer.reserve(streamMetadata->getNumValues());
                decodeRaw(tileData, floatBuffer, *streamMetadata, /*consume=*/true);
                return {std::move(floatBuffer), std::move(presentStream)};
            }
            case ScalarType::DOUBLE: {
                std::vector<double> doubleBuffer;
                doubleBuffer.reserve(streamMetadata->getNumValues());
                decodeRaw(tileData, doubleBuffer, *streamMetadata, /*consume=*/true);
                return {std::move(doubleBuffer), std::move(presentStream)};
            }
            case ScalarType::STRING: {
                StringDictViews strings;
                stringDecoder.decode(
                    tileData, numStreams - 1, presentValueCount, presentStream, strings.first, strings.second);
                return {PropertyVec{std::move(strings)}, std::move(presentStream)};
            }
            default:
                throw std::runtime_error("Unknown scalar type: " + std::to_string(std::to_underlying(scalarType)));
        }
    }

private:
    IntegerDecoder& intDecoder;
    StringDecoder& stringDecoder;
};

} // namespace mlt::decoder
