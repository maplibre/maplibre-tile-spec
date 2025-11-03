#pragma once

#include <mlt/common.hpp>
#include <mlt/decode/int.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/properties.hpp>
#include <mlt/util/packed_bitset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/raw.hpp>

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
                    throw std::runtime_error("Present stream not supported for string columns");
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

        for (std::uint32_t i = 0; i < numValues; ++i) {
            if (presentStream.empty() || testBit(presentStream, i)) {
                out.push_back(dictionary[offsets[i]]);
            } else {
                // TODO: Differentiate between null and empty strings?
                out.emplace_back();
            }
        }
    }
};

} // namespace mlt::decoder
