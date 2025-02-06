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
    StringDecoder(IntegerDecoder& intDecoder_) noexcept(false)
        : intDecoder(intDecoder_) {}

    /*
     * String column layouts:
     * -> plain -> present, length, data
     * -> dictionary -> present, length, dictionary, data
     * -> fsst dictionary -> symbolTable, symbolLength, dictionary, length, present, data
     * */

    StringDictViews decode(BufferStream& tileData, count_t numStreams, count_t numValues) noexcept(false) {
        using namespace metadata::stream;
        using namespace util::decoding;

        std::vector<std::uint8_t> symDataStream;
        std::vector<std::uint8_t> dictDataStream;
        std::vector<std::uint32_t> offsetStream;
        std::vector<std::uint32_t> symLengthStream;
        std::vector<std::uint32_t> dictLengthStream;
        std::vector<std::string_view> views;
        views.reserve(numValues);
        for (count_t i = 0; i < numStreams; ++i) {
            auto streamMetadata = StreamMetadata::decode(tileData);
            switch (streamMetadata->getPhysicalStreamType()) {
                case PhysicalStreamType::PRESENT:
                    throw std::runtime_error("Present stream not supported for string columns");
                case PhysicalStreamType::OFFSET:
                    intDecoder.decodeIntStream<std::uint32_t>(tileData, offsetStream, *streamMetadata);
                    break;
                case PhysicalStreamType::LENGTH: {
                    if (!streamMetadata->getLogicalStreamType() ||
                        !streamMetadata->getLogicalStreamType()->getLengthType()) {
                        throw std::runtime_error("Length stream missing logical type");
                    }
                    const auto type = *streamMetadata->getLogicalStreamType()->getLengthType();
                    auto& target = (type == LengthType::DICTIONARY) ? dictLengthStream : symLengthStream;
                    intDecoder.decodeIntStream<std::uint32_t>(tileData, target, *streamMetadata);
                    break;
                }
                case PhysicalStreamType::DATA: {
                    if (!streamMetadata->getLogicalStreamType() ||
                        !streamMetadata->getLogicalStreamType()->getDictionaryType()) {
                        throw std::runtime_error("Data stream missing logical type");
                    }
                    const auto type = *streamMetadata->getLogicalStreamType()->getDictionaryType();
                    auto& target = (type == DictionaryType::SINGLE) ? dictDataStream : symDataStream;
                    decodeRaw(tileData, target, streamMetadata->getByteLength(), /*consume=*/true);
                    break;
                }
            }
        }

        if (!symDataStream.empty()) {
            throw std::runtime_error("FSST decoding not implemented");
        } else if (!dictDataStream.empty()) {
            decodeDictionary(dictLengthStream, dictDataStream, offsetStream, views, numValues);
            return {std::move(dictDataStream), std::move(views)};
        } else {
            decodePlain(dictLengthStream, dictDataStream, views, numValues);
            return {std::move(dictDataStream), std::move(views)};
        }
    }

private:
    IntegerDecoder& intDecoder;

    static void decodePlain(const std::vector<std::uint32_t>& lengthStream,
                            const std::vector<std::uint8_t>& utf8bytes,
                            std::vector<std::string_view>& out,
                            count_t numValues) {
        for (count_t i = 0; i < numValues; ++i) {
            const auto length = lengthStream[i];
            const char* bytes = reinterpret_cast<std::string::const_pointer>(utf8bytes.data() + length);
            out.emplace_back(bytes, length);
        }
    }

    static void decodeDictionary(const std::vector<std::uint32_t>& lengthStream,
                                 const std::vector<std::uint8_t>& utf8bytes,
                                 const std::vector<std::uint32_t>& offsets,
                                 std::vector<std::string_view>& out,
                                 count_t numValues) {
        const auto* const utf8Ptr = reinterpret_cast<const char*>(utf8bytes.data());

        std::vector<std::string_view> dictionary;
        dictionary.reserve(lengthStream.size());

        count_t dictionaryOffset = 0;
        for (const auto length : lengthStream) {
            dictionary.emplace_back(utf8Ptr + dictionaryOffset, length);
            dictionaryOffset += length;
        }

        for (count_t i = 0; i < numValues; ++i) {
            out.push_back(dictionary[offsets[i]]);
        }
    }
};

} // namespace mlt::decoder
