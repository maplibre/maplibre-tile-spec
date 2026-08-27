#pragma once

#include <mlt/decode/int.hpp>
#include <mlt/decode/string.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/nested_value.hpp>
#include <mlt/properties.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/packed_bitset.hpp>
#include <mlt/util/raw.hpp>
#include <mlt/util/rle.hpp>
#include <mlt/util/varint.hpp>

#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace mlt::decoder {

class MapPropertyDecoder {
public:
    MapPropertyDecoder(IntegerDecoder& intDecoder_, StringDecoder& stringDecoder_)
        : intDecoder(intDecoder_),
          stringDecoder(stringDecoder_) {}

    MapPropertyVecMap decodeMapPropertyColumn(BufferStream& tileData,
                                              const metadata::tileset::Column& column,
                                              std::uint32_t numStreams) {
        auto columnNames = getMapColumnNames(column);

        if (numStreams == 0) {
            return {};
        }

        // The dictionary mask is a raw byte and is not counted in the stream count
        const auto mask = static_cast<std::uint8_t>(tileData.read());

        const auto lengthStreamMeta = metadata::stream::StreamMetadata::decode(tileData);
        std::vector<std::uint32_t> lengthStream;
        intDecoder.decodeIntStream<std::uint32_t>(tileData, lengthStream, *lengthStreamMeta);
        numStreams--;

        std::vector<NestedValue> dictionary;

        if (mask & MASK_STRING) {
            const auto stringStreamCount = static_cast<std::uint32_t>(tileData.read());
            const auto strings = stringDecoder.decode(tileData, stringStreamCount);
            for (const auto& sv : strings.getStrings()) {
                dictionary.emplace_back(std::string(sv));
            }
            numStreams -= stringStreamCount;
        }

        if (mask & MASK_INT32) {
            const auto meta = metadata::stream::StreamMetadata::decode(tileData);
            std::vector<std::int32_t> values;
            intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::int32_t>(tileData, values, *meta, true);
            for (auto v : values) {
                dictionary.emplace_back(static_cast<std::int64_t>(v));
            }
            numStreams--;
        } else if (mask & MASK_INT64) {
            const auto meta = metadata::stream::StreamMetadata::decode(tileData);
            std::vector<std::int64_t> values;
            intDecoder.decodeIntStream<std::uint64_t, std::uint64_t, std::int64_t>(tileData, values, *meta, true);
            for (auto v : values) {
                dictionary.emplace_back(v);
            }
            numStreams--;
        }

        if (mask & MASK_UINT32) {
            const auto meta = metadata::stream::StreamMetadata::decode(tileData);
            std::vector<std::uint32_t> values;
            intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::uint32_t>(tileData, values, *meta, false);
            for (auto v : values) {
                dictionary.emplace_back(static_cast<std::uint64_t>(v));
            }
            numStreams--;
        } else if (mask & MASK_UINT64) {
            const auto meta = metadata::stream::StreamMetadata::decode(tileData);
            std::vector<std::int64_t> raw;
            intDecoder.decodeIntStream<std::uint64_t, std::uint64_t, std::int64_t>(tileData, raw, *meta, false);
            for (auto v : raw) {
                dictionary.emplace_back(static_cast<std::uint64_t>(v));
            }
            numStreams--;
        }

        if (mask & MASK_FLOAT) {
            const auto meta = metadata::stream::StreamMetadata::decode(tileData);
            std::vector<float> values;
            util::decoding::decodeRaw(tileData, values, *meta, true);
            for (auto v : values) {
                dictionary.emplace_back(v);
            }
            numStreams--;
        }

        if (mask & MASK_DOUBLE) {
            const auto meta = metadata::stream::StreamMetadata::decode(tileData);
            std::vector<double> values;
            util::decoding::decodeRaw(tileData, values, *meta, true);
            for (auto v : values) {
                dictionary.emplace_back(v);
            }
            numStreams--;
        }

        PackedBitset presentStream;
        std::uint32_t presentCount = 0;
        if (mask & MASK_PRESENCE) {
            const auto meta = metadata::stream::StreamMetadata::decode(tileData);
            presentCount = meta->getNumValues();
            util::decoding::rle::decodeBoolean(tileData, presentStream, *meta, true);
            numStreams--;
        }

        std::vector<std::uint32_t> flattenedValues;
        if (numStreams > 0) {
            const auto meta = metadata::stream::StreamMetadata::decode(tileData);
            intDecoder.decodeIntStream<std::uint32_t>(tileData, flattenedValues, *meta);
            numStreams--;
        }

        if (numStreams != 0) {
            throw std::runtime_error("Unexpected remaining streams in map column: " + std::to_string(numStreams));
        }

        const auto numColumns = columnNames.size();
        const auto featureCount = (!presentStream.empty() ? presentCount
                                                          : static_cast<std::uint32_t>(lengthStream.size())) /
                                  static_cast<std::uint32_t>(numColumns);

        MapPropertyVecMap result;
        std::uint32_t countsCursor = 0;
        std::uint32_t valuesCursor = 0;

        for (std::size_t childIndex = 0; childIndex < numColumns; ++childIndex) {
            std::uint32_t childPresentCount = 0;
            std::vector<bool> childPresent;

            // Shared columns are laid out child-major, all features of child 0 then all of child 1
            if (!presentStream.empty()) {
                childPresent.resize(featureCount);
                const auto childPresentOffset = static_cast<std::uint32_t>(childIndex) * featureCount;
                for (std::uint32_t fi = 0; fi < featureCount; ++fi) {
                    if (testBit(presentStream, childPresentOffset + fi)) {
                        childPresent[fi] = true;
                        childPresentCount++;
                    }
                }
            } else {
                childPresentCount = featureCount;
            }

            const auto childCountsEnd = countsCursor + childPresentCount;
            if (childCountsEnd > lengthStream.size()) {
                throw std::runtime_error("Merged map counts underflow while decoding child streams");
            }

            std::vector<std::optional<NestedValue>> decodedProperties(featureCount);

            auto flattenedIndex = valuesCursor;
            auto countCursor = countsCursor;
            for (std::uint32_t fi = 0; fi < featureCount; ++fi) {
                const bool isPresent = childPresent.empty() || childPresent[fi];
                if (!isPresent) {
                    continue;
                }

                if (countCursor >= childCountsEnd) {
                    throw std::runtime_error("Map count stream underflow");
                }

                const auto featureValueCount = lengthStream[countCursor++];
                const auto endIndex = flattenedIndex + featureValueCount;
                if (endIndex > flattenedValues.size()) {
                    throw std::runtime_error("Map value stream underflow");
                }

                if (featureValueCount == 1) {
                    auto [value, next] = decodeValue(flattenedValues, flattenedIndex, endIndex, dictionary);
                    decodedProperties[fi] = std::move(value);
                    flattenedIndex = next;
                } else if (flattenedIndex < endIndex && flattenedValues[flattenedIndex] == CV_START_LIST) {
                    auto [value, next] = decodeValue(flattenedValues, flattenedIndex, endIndex, dictionary);
                    decodedProperties[fi] = std::move(value);
                    flattenedIndex = next;
                } else {
                    // Root maps have no START_MAP header, only nested ones do
                    auto [value, next] = decodeMapEntries(flattenedValues, flattenedIndex, endIndex, dictionary);
                    decodedProperties[fi] = std::move(value);
                    flattenedIndex = next;
                }
            }

            std::uint32_t childValueCount = 0;
            for (std::uint32_t i = countsCursor; i < childCountsEnd; ++i) {
                childValueCount += lengthStream[i];
            }
            valuesCursor += childValueCount;
            countsCursor = childCountsEnd;

            result.emplace(std::move(columnNames[childIndex]), MapProperties(std::move(decodedProperties)));
        }

        return result;
    }

private:
    static constexpr std::uint32_t CV_FALSE = 0;
    static constexpr std::uint32_t CV_TRUE = 1;
    static constexpr std::uint32_t CV_START_MAP = 2;
    static constexpr std::uint32_t CV_START_LIST = 3;
    static constexpr std::uint32_t CV_COUNT = 4;

    static constexpr std::uint8_t MASK_STRING = 1;
    static constexpr std::uint8_t MASK_INT32 = 1 << 1;
    static constexpr std::uint8_t MASK_UINT32 = 1 << 2;
    static constexpr std::uint8_t MASK_INT64 = 1 << 3;
    static constexpr std::uint8_t MASK_UINT64 = 1 << 4;
    static constexpr std::uint8_t MASK_FLOAT = 1 << 5;
    static constexpr std::uint8_t MASK_DOUBLE = 1 << 6;
    static constexpr std::uint8_t MASK_PRESENCE = 1 << 7;

    struct Decoded {
        NestedValue value;
        std::uint32_t nextIndex;
    };

    static Decoded decodeValue(const std::vector<std::uint32_t>& tokens,
                               std::uint32_t start,
                               std::uint32_t end,
                               const std::vector<NestedValue>& dictionary) {
        if (start >= end) {
            throw std::runtime_error("Unexpected end of map value stream");
        }

        const auto token = tokens[start];
        switch (token) {
            case CV_FALSE:
                return {NestedValue(false), start + 1};
            case CV_TRUE:
                return {NestedValue(true), start + 1};
            case CV_START_MAP: {
                const auto valueEnd = getValueEndIndex(tokens, start, end);
                const auto payloadStart = start + 2;
                return {decodeMapEntries(tokens, payloadStart, valueEnd, dictionary).value, valueEnd};
            }
            case CV_START_LIST: {
                const auto valueEnd = getValueEndIndex(tokens, start, end);
                const auto payloadStart = start + 2;
                NestedArray listValues;
                auto idx = payloadStart;
                while (idx < valueEnd) {
                    auto [val, next] = decodeValue(tokens, idx, valueEnd, dictionary);
                    listValues.push_back(std::move(val));
                    idx = next;
                }
                return {NestedValue(std::move(listValues)), valueEnd};
            }
            default:
                return {decodeScalarByIndex(token, dictionary), start + 1};
        }
    }

    static Decoded decodeMapEntries(const std::vector<std::uint32_t>& tokens,
                                    std::uint32_t start,
                                    std::uint32_t end,
                                    const std::vector<NestedValue>& dictionary) {
        NestedObject obj;
        auto idx = start;
        while (idx < end) {
            const auto keyVal = decodeScalarByIndex(tokens[idx++], dictionary);
            if (!keyVal.isString()) {
                throw std::runtime_error("Map key is not a string");
            }
            auto keyStr = std::string(keyVal.getString());
            auto [val, next] = decodeValue(tokens, idx, end, dictionary);
            obj.push_back(NestedObjectEntry{std::move(keyStr), std::move(val)});
            idx = next;
        }
        return {NestedValue(std::move(obj)), idx};
    }

    static std::uint32_t getValueEndIndex(const std::vector<std::uint32_t>& tokens,
                                          std::uint32_t start,
                                          std::uint32_t end) {
        if (start + 1 >= end) {
            throw std::runtime_error("Missing length for nested map/list payload");
        }
        // The length counts the two header tokens as well as the payload
        const auto encodedLength = tokens[start + 1];
        if (encodedLength < 2) {
            throw std::runtime_error("Invalid nested payload length");
        }
        const auto valueEnd = start + encodedLength;
        if (valueEnd > end) {
            throw std::runtime_error("Nested payload exceeds containing bounds");
        }
        return valueEnd;
    }

    static NestedValue decodeScalarByIndex(std::uint32_t token, const std::vector<NestedValue>& dictionary) {
        if (token < CV_COUNT) {
            throw std::runtime_error("Invalid scalar dictionary index: " + std::to_string(token));
        }
        const auto offset = token - CV_COUNT;
        if (offset >= dictionary.size()) {
            throw std::runtime_error("Scalar dictionary index out of range: " + std::to_string(token));
        }
        return dictionary[offset];
    }

    static std::vector<std::string> getMapColumnNames(const metadata::tileset::Column& column) {
        if (column.hasComplexType()) {
            const auto& complex = column.getComplexType();
            if (!complex.children.empty()) {
                std::vector<std::string> names;
                names.reserve(complex.children.size());
                for (const auto& child : complex.children) {
                    names.push_back(column.name + child.name);
                }
                return names;
            }
        }
        return {column.name};
    }

    IntegerDecoder& intDecoder;
    StringDecoder& stringDecoder;
};

} // namespace mlt::decoder
