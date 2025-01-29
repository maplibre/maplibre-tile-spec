#pragma once

#include <common.hpp>
#include <decode/int.hpp>
#include <metadata/stream.hpp>
#include <stdexcept>
#include <util/buffer_stream.hpp>

#include <string>
#include "util/raw.hpp"

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

    void decode(BufferStream& tileData,
                count_t numStreams,
                count_t numValues,
                const std::vector<std::uint8_t>& presentStream,
                std::vector<std::string>& out) noexcept(false) {
        using namespace metadata::stream;

        assert(numValues <= 8 * presentStream.size());

        std::vector<std::uint8_t> symDataStream;
        std::vector<std::uint8_t> dictDataStream;
        std::vector<std::uint32_t> offsetStream;
        std::vector<std::uint32_t> symLengthStream;
        std::vector<std::uint32_t> dictLengthStream;
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
                    util::decoding::decodeRaw(tileData, target, streamMetadata->getByteLength(), /*consume=*/true);
                    break;
                }
            }
        }

        if (!symDataStream.empty()) {
            throw std::runtime_error("FSST decoding not implemented");
        } else if (!dictDataStream.empty()) {
            std::vector<std::string_view> out;
            out.reserve(numValues);
            decodeDictionary(presentStream, dictLengthStream, dictDataStream, offsetStream, out, numValues);
        } else {
            decodePlain(presentStream, dictLengthStream, dictDataStream, out, numValues);
        }
    }

private:
    IntegerDecoder& intDecoder;

    static void decodePlain(const std::vector<std::uint8_t>& presentStream,
                            const std::vector<std::uint32_t>& lengthStream,
                            const std::vector<std::uint8_t>& utf8bytes,
                            std::vector<std::string>& out,
                            count_t numValues) {
        for (count_t i = 0; i < numValues; ++i) {
            if (presentStream.empty() || presentStream[i / 8] & (1 << (i % 8))) {
                const auto length = lengthStream[i];
                const char* bytes = reinterpret_cast<std::string::const_pointer>(utf8bytes.data() + length);
                out.emplace_back(bytes, length);
            } else {
                out.emplace_back();
            }
        }
    }

    static void decodeDictionary(const std::vector<std::uint8_t>& presentStream,
                                 const std::vector<std::uint32_t>& lengthStream,
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

        auto offset = 0;
        for (count_t i = 0; i < numValues; ++i) {
            if (presentStream.empty() || presentStream[i / 8] & (1 << (i % 8))) {
                out.push_back(dictionary[offsets[offset++]]);
            } else {
                out.emplace_back();
            }
        }
    }
};

} // namespace mlt::decoder
