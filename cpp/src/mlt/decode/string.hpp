#pragma once

#include <mlt/common.hpp>
#include <mlt/decode/int.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/properties.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/packed_bitset.hpp>
#include <mlt/util/raw.hpp>
#include <mlt/util/varint.hpp>

#include <string>
#include <stdexcept>
#include <string_view>

namespace mlt::decoder {

class StringDecoder {
public:
    StringDecoder(IntegerDecoder& intDecoder_)
        : intDecoder(intDecoder_) {}

    /*
     * String column layouts:
     * -> plain -> present, length, data
     * -> dictionary -> present, length, dictionary, data
     * -> fsst dictionary -> symbolTable, symbolLength, dictionary, length, present, data
     * */

    StringDictViews decode(BufferStream& tileData, std::uint32_t numStreams, std::uint32_t numValues) {
        using namespace metadata::stream;
        using namespace util::decoding;

        std::optional<DictionaryType> dictType;
        std::optional<LengthType> lengthType;
        std::vector<std::uint8_t> dataStream;
        std::vector<std::uint32_t> offsetStream;
        std::vector<std::uint32_t> lengthStream;
        std::vector<std::string_view> views;
        PackedBitset presentStream;
        std::optional<std::uint32_t> presentValueCount;
        views.reserve(numValues);
        for (std::uint32_t i = 0; i < numStreams; ++i) {
            const auto streamMetadata = StreamMetadata::decode(tileData);
            switch (streamMetadata->getPhysicalStreamType()) {
                default:
                    throw std::runtime_error("Unsupported stream type");
                case PhysicalStreamType::PRESENT:
                    presentValueCount = streamMetadata->getNumValues();
                    rle::decodeBoolean(tileData, presentStream, *streamMetadata, /*consume=*/true);
                    if ((*presentValueCount + 7) / 8 != presentStream.size()) {
                        throw std::runtime_error("invalid present stream");
                    }
                    break;
                case PhysicalStreamType::OFFSET:
                    intDecoder.decodeIntStream<std::uint32_t>(tileData, offsetStream, *streamMetadata);
                    break;
                case PhysicalStreamType::LENGTH: {
                    if (!streamMetadata->getLogicalStreamType() ||
                        !streamMetadata->getLogicalStreamType()->getLengthType()) {
                        throw std::runtime_error("Length stream missing logical type");
                    }
                    lengthType = streamMetadata->getLogicalStreamType()->getLengthType();
                    intDecoder.decodeIntStream<std::uint32_t>(tileData, lengthStream, *streamMetadata);
                    break;
                }
                case PhysicalStreamType::DATA: {
                    if (!streamMetadata->getLogicalStreamType() ||
                        !streamMetadata->getLogicalStreamType()->getDictionaryType()) {
                        throw std::runtime_error("Data stream missing logical type");
                    }
                    dictType = streamMetadata->getLogicalStreamType()->getDictionaryType();
                    decodeRaw(tileData, dataStream, streamMetadata->getByteLength(), /*consume=*/true);
                    break;
                }
            }
        }

        if (presentValueCount && *presentValueCount != numValues) {
            throw std::runtime_error("Unexpected present value count for string column");
        }

        if (dictType == DictionaryType::FSST) {
            throw std::runtime_error("FSST decoding not implemented");
        } else if (dictType == DictionaryType::SINGLE) {
            decodeDictionary(lengthStream, dataStream, offsetStream, presentStream, views, numValues);
            return {std::move(dataStream), std::move(views)};
        } else {
            decodePlain(lengthStream, dataStream, presentStream, views, numValues);
            return {std::move(dataStream), std::move(views)};
        }
    }

    /// Multiple string columns sharing a dictionary
    PropertyVecMap decodeSharedDictionary(BufferStream& tileData,
                                          const metadata::tileset::Column& column,
                                          std::uint32_t numStreams) {
        using namespace metadata::stream;
        using namespace metadata::tileset::schema;
        using namespace util::decoding;

        if (!column.hasComplexType() || !column.getComplexType().hasChildren() || numStreams < 3) {
            throw std::runtime_error("Expected struct column for shared dictionary decoding");
        }

        std::vector<std::uint32_t> dictionaryLengthStream;
        const auto dictionaryStream = std::make_shared<std::vector<std::uint8_t>>();
        std::vector<std::uint32_t> symbolLengthStream;
        std::vector<std::uint8_t> symbolTableStream;
        PropertyVecMap results;

        bool dictionaryStreamDecoded = false;
        while (!dictionaryStreamDecoded && numStreams--) {
            const auto streamMetadata = metadata::stream::StreamMetadata::decode(tileData);
            switch (streamMetadata->getPhysicalStreamType()) {
                case metadata::stream::PhysicalStreamType::LENGTH: {
                    const bool isDict = streamMetadata->getLogicalStreamType()->getLengthType() ==
                                        LengthType::DICTIONARY;
                    auto& target = isDict ? dictionaryLengthStream : symbolLengthStream;
                    intDecoder.decodeIntStream<std::uint32_t>(tileData, target, *streamMetadata);
                    break;
                }
                case metadata::stream::PhysicalStreamType::DATA: {
                    const bool isShared = streamMetadata->getLogicalStreamType()->getDictionaryType() ==
                                          DictionaryType::SHARED;
                    auto& targetStream = isShared ? *dictionaryStream : symbolTableStream;
                    decodeRaw(tileData, targetStream, streamMetadata->getByteLength(), /*consume=*/true);
                    dictionaryStreamDecoded = isShared;
                    break;
                }
                default:
                    throw std::runtime_error("Unsupported stream type");
            }
        }

        std::vector<std::string_view> dictionaryViews;
        if (!symbolLengthStream.empty() && !symbolTableStream.empty() && !dictionaryLengthStream.empty()) {
            throw std::runtime_error("FSST decoding not implemented");
        } else if (!dictionaryLengthStream.empty() && !dictionaryStream->empty()) {
            decodeDictionary(dictionaryLengthStream, *dictionaryStream, dictionaryViews);
        } else {
            throw std::runtime_error("Expected streams missing in shared dictionary decoding");
        }

        for (const auto& child : column.getComplexType().children) {
            const auto childStreams = decodeVarint<std::uint32_t>(tileData);
            // TODO: We don't really need this stream count until the present stream is optional.
            // We could infer it from the nullable bit in the column, but only if that applies to all children.
            if (childStreams != 2 || !child.hasScalarType() ||
                child.getScalarType().getPhysicalType() != ScalarType::STRING) {
                throw std::runtime_error("Currently only optional string fields are implemented for a struct");
            }

            const auto presentStreamMetadata = metadata::stream::StreamMetadata::decode(tileData);
            const auto presentValueCount = presentStreamMetadata->getNumValues();
            PackedBitset presentStream;
            rle::decodeBoolean(tileData, presentStream, *presentStreamMetadata, /*consume=*/true);
            if ((presentValueCount + 7) / 8 != presentStream.size()) {
                throw std::runtime_error("invalid present stream");
            }

            const auto dataStreamMetadata = metadata::stream::StreamMetadata::decode(tileData);
            std::vector<std::uint32_t> dataReferenceStream;
            intDecoder.decodeIntStream<std::uint32_t>(tileData, dataReferenceStream, *dataStreamMetadata);

            std::vector<std::string_view> propertyValues;
            propertyValues.reserve(dataStreamMetadata->getNumValues());

            std::uint32_t counter = 0;
            for (std::uint32_t i = 0; i < presentValueCount; ++i) {
                if (testBit(presentStream, i)) {
                    if (counter >= dataReferenceStream.size()) {
                        throw std::runtime_error("StringDecoder: dataReferenceStream out of bounds");
                    }
                    auto dictIndex = dataReferenceStream[counter++];
                    if (dictIndex >= dictionaryViews.size()) {
                        throw std::runtime_error("StringDecoder: dictionaryViews index out of bounds");
                    }
                    propertyValues.push_back(dictionaryViews[dictIndex]);
                }
            }

            results.emplace(column.name + child.name,
                            PresentProperties{child.getScalarType().getPhysicalType(),
                                              StringDictViews(dictionaryStream, propertyValues),
                                              std::move(presentStream)});
        }

        return results;
    }

private:
    IntegerDecoder& intDecoder;

    /// Drop the useless codepoint produced when a UTF-16 Byte-Order-Mark is included in the conversion to UTF-8
    // https://en.wikipedia.org/wiki/Byte_order_mark#UTF-8
    // TODO: Can we force this on the encoding side instead?
    static std::string_view view(const char* bytes, std::size_t length) {
        if (length >= 3 && std::equal(bytes, bytes + 3, "\xEF\xBB\xBF")) {
            bytes += 3;
            length -= 3;
        }
        return {bytes, length};
    }

    static void decodePlain(const std::vector<std::uint32_t>& lengthStream,
                            const std::vector<std::uint8_t>& utf8bytes,
                            const PackedBitset& presentStream,
                            std::vector<std::string_view>& out,
                            std::uint32_t numValues) {
        std::size_t dataOffset = 0;
        std::size_t lengthOffset = 0;
        for (std::uint32_t i = 0; i < numValues; ++i) {
            if (presentStream.empty() || testBit(presentStream, i)) {
                const auto length = lengthStream[lengthOffset++];
                const char* bytes = reinterpret_cast<std::string::const_pointer>(utf8bytes.data() + dataOffset);
                out.push_back(view(bytes, length));
                dataOffset += length;
            } else {
                // TODO: Differentiate between null and empty strings?
                out.emplace_back();
            }
        }
    }

    static void decodeDictionary(const std::vector<std::uint32_t>& lengthStream,
                                 const std::vector<std::uint8_t>& utf8bytes,
                                 std::vector<std::string_view>& out) {
        const auto* const utf8Ptr = reinterpret_cast<const char*>(utf8bytes.data());

        std::size_t offset = 0;
        for (auto length : lengthStream) {
            out.emplace_back(utf8Ptr + offset, length);
            offset += length;
        }
    }

    static void decodeDictionary(const std::vector<std::uint32_t>& lengthStream,
                                 const std::vector<std::uint8_t>& utf8bytes,
                                 const std::vector<std::uint32_t>& offsets,
                                 const PackedBitset& presentStream,
                                 std::vector<std::string_view>& out,
                                 std::uint32_t numValues) {
        const auto* const utf8Ptr = reinterpret_cast<const char*>(utf8bytes.data());

        std::vector<std::string_view> dictionary;
        dictionary.reserve(lengthStream.size());

        std::uint32_t dictionaryOffset = 0;
        for (const auto length : lengthStream) {
            dictionary.push_back(view(utf8Ptr + dictionaryOffset, length));
            dictionaryOffset += length;
        }

        std::size_t offsetIndex = 0;
        for (std::uint32_t i = 0; i < numValues; ++i) {
            if (presentStream.empty() || testBit(presentStream, i)) {
                out.push_back(dictionary[offsets[offsetIndex++]]);
            } else {
                // TODO: Differentiate between null and empty strings?
                out.emplace_back();
            }
        }
    }
};

} // namespace mlt::decoder
