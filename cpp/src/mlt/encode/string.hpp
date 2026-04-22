#pragma once

#include <mlt/encode/boolean.hpp>
#include <mlt/encode/int.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/util/encoding/fsst.hpp>
#include <mlt/util/encoding/varint.hpp>

#include <cstdint>
#include <span>
#include <string_view>
#include <unordered_map>
#include <vector>

namespace mlt::encoder {

class StringEncoder {
public:
    using PhysicalLevelTechnique = metadata::stream::PhysicalLevelTechnique;
    using PhysicalStreamType = metadata::stream::PhysicalStreamType;
    using LogicalStreamType = metadata::stream::LogicalStreamType;
    using StreamMetadata = metadata::stream::StreamMetadata;

    struct EncodeResult {
        std::uint32_t numStreams;
        std::vector<std::uint8_t> data;
    };

    static EncodeResult encode(std::span<const std::string_view> values,
                               PhysicalLevelTechnique physicalTechnique,
                               IntegerEncoder& intEncoder,
                               bool useFsst = true) {
        auto plain = encodeStringBytes(values,
                                       physicalTechnique,
                                       intEncoder,
                                       metadata::stream::LengthType::VAR_BINARY,
                                       metadata::stream::DictionaryType::NONE);
        auto dict = encodeDictionary(values, physicalTechnique, intEncoder);

        std::uint32_t bestStreams = 2;
        std::vector<std::uint8_t>* best = &plain;
        if (dict.size() < best->size()) {
            bestStreams = 3;
            best = &dict;
        }

        if (useFsst) {
            auto fsstDict = encodeFsstDictionary(values, physicalTechnique, intEncoder, true, false);
            if (fsstDict.size() < best->size()) {
                return {.numStreams = 5, .data = std::move(fsstDict)};
            }
        }

        return {.numStreams = bestStreams, .data = std::move(*best)};
    }

    static EncodeResult encodeSharedDictionary(const std::vector<std::vector<const std::string_view*>>& columns,
                                               PhysicalLevelTechnique physicalTechnique,
                                               IntegerEncoder& intEncoder,
                                               bool useFsst = true) {
        using namespace metadata::stream;

        std::vector<std::string_view> dictionary;
        std::unordered_map<std::string_view, std::uint32_t> dictIndex;
        std::vector<std::vector<std::int32_t>> dataStreams(columns.size());
        std::vector<std::vector<bool>> presentStreams(columns.size());

        for (std::size_t col = 0; col < columns.size(); ++col) {
            for (const auto* sv : columns[col]) {
                if (!sv) {
                    presentStreams[col].push_back(false);
                } else {
                    presentStreams[col].push_back(true);
                    auto [it, inserted] = dictIndex.try_emplace(*sv, static_cast<std::uint32_t>(dictionary.size()));
                    if (inserted) {
                        dictionary.push_back(*sv);
                    }
                    dataStreams[col].push_back(static_cast<std::int32_t>(it->second));
                }
            }
        }

        if (dictionary.empty()) {
            return {.numStreams = 0, .data = {}};
        }

        auto dictData = encodeStringBytes(dictionary,
                                          physicalTechnique,
                                          intEncoder,
                                          metadata::stream::LengthType::DICTIONARY,
                                          metadata::stream::DictionaryType::SHARED);
        // Non-FSST shared dictionary emits only LENGTH + DATA streams.
        std::uint32_t dictStreams = 2;

        if (useFsst) {
            auto fsstData = encodeFsst(dictionary, physicalTechnique, intEncoder, true);
            if (fsstData.size() < dictData.size()) {
                dictData = std::move(fsstData);
                // FSST shared dictionary emits LENGTH(symbol), DATA(symbol table),
                // LENGTH(dictionary), DATA(compressed dictionary).
                dictStreams = 4;
            }
        }

        std::vector<std::uint8_t> result;
        result.insert(result.end(), dictData.begin(), dictData.end());

        for (std::size_t col = 0; col < columns.size(); ++col) {
            if (dataStreams[col].empty()) {
                util::encoding::encodeVarint(static_cast<std::uint32_t>(0), result);
                continue;
            }

            util::encoding::encodeVarint(static_cast<std::uint32_t>(2), result);

            auto presentData = BooleanEncoder::encodeBooleanStream(presentStreams[col], PhysicalStreamType::PRESENT);
            result.insert(result.end(), presentData.begin(), presentData.end());

            auto offsetData = intEncoder.encodeIntStream(dataStreams[col],
                                                         physicalTechnique,
                                                         false,
                                                         PhysicalStreamType::OFFSET,
                                                         LogicalStreamType{OffsetType::STRING});
            result.insert(result.end(), offsetData.begin(), offsetData.end());
        }

        const auto numStreams = dictStreams + (static_cast<std::uint32_t>(columns.size()) * 2);
        return {.numStreams = numStreams, .data = std::move(result)};
    }

private:
    struct EncodedStringStreams {
        std::vector<std::uint8_t> firstStream;
        std::vector<std::uint8_t> remainingStreams;
    };

    struct DictionaryIndex {
        std::vector<std::string_view> dictionary;
        std::vector<std::int32_t> offsets;
    };

    static DictionaryIndex buildDictIndex(std::span<const std::string_view> values) {
        DictionaryIndex result;
        result.offsets.reserve(values.size());
        std::unordered_map<std::string_view, std::uint32_t> indexMap;
        indexMap.reserve(values.size());
        for (const auto sv : values) {
            auto [it, inserted] = indexMap.try_emplace(sv, static_cast<std::uint32_t>(result.dictionary.size()));
            if (inserted) {
                result.dictionary.push_back(sv);
            }
            result.offsets.push_back(static_cast<std::int32_t>(it->second));
        }
        return result;
    }

    static std::vector<std::uint8_t> encodeOffsetsAndData(const std::vector<std::uint8_t>& firstStream,
                                                          const std::vector<std::uint8_t>& remainingStreams,
                                                          std::span<const std::int32_t> offsets,
                                                          PhysicalLevelTechnique physicalTechnique,
                                                          IntegerEncoder& intEncoder) {
        using namespace metadata::stream;

        auto encodedOffsets = intEncoder.encodeIntStream(
            offsets, physicalTechnique, false, PhysicalStreamType::OFFSET, LogicalStreamType{OffsetType::STRING});

        std::vector<std::uint8_t> result;
        result.reserve(firstStream.size() + encodedOffsets.size() + remainingStreams.size());
        result.insert(result.end(), firstStream.begin(), firstStream.end());
        result.insert(result.end(), encodedOffsets.begin(), encodedOffsets.end());
        result.insert(result.end(), remainingStreams.begin(), remainingStreams.end());
        return result;
    }

    static std::vector<std::uint8_t> encodeDictionary(std::span<const std::string_view> values,
                                                      PhysicalLevelTechnique physicalTechnique,
                                                      IntegerEncoder& intEncoder) {
        const auto [dictionary, offsets] = buildDictIndex(values);
        auto dictStreams = encodeStringStreams(dictionary,
                                               physicalTechnique,
                                               intEncoder,
                                               metadata::stream::LengthType::DICTIONARY,
                                               metadata::stream::DictionaryType::SINGLE);
        return encodeOffsetsAndData(
            dictStreams.firstStream, dictStreams.remainingStreams, offsets, physicalTechnique, intEncoder);
    }

    static std::vector<std::uint8_t> encodeFsstDictionary(std::span<const std::string_view> values,
                                                          PhysicalLevelTechnique physicalTechnique,
                                                          IntegerEncoder& intEncoder,
                                                          bool encodeOffsets,
                                                          bool isShared) {
        const auto [dictionary, offsets] = buildDictIndex(values);
        auto fsstData = encodeFsst(dictionary, physicalTechnique, intEncoder, isShared);

        if (!encodeOffsets) {
            return fsstData;
        }

        auto fsstStreams = encodeFsstStreams(dictionary, physicalTechnique, intEncoder, isShared);
        return encodeOffsetsAndData(
            fsstStreams.firstStream, fsstStreams.remainingStreams, offsets, physicalTechnique, intEncoder);
    }

    static std::vector<std::uint8_t> encodeFsst(std::span<const std::string_view> values,
                                                PhysicalLevelTechnique physicalTechnique,
                                                IntegerEncoder& intEncoder,
                                                bool isShared) {
        auto streams = encodeFsstStreams(values, physicalTechnique, intEncoder, isShared);
        std::vector<std::uint8_t> out;
        out.reserve(streams.firstStream.size() + streams.remainingStreams.size());
        out.insert(out.end(), streams.firstStream.begin(), streams.firstStream.end());
        out.insert(out.end(), streams.remainingStreams.begin(), streams.remainingStreams.end());
        return out;
    }

    static EncodedStringStreams encodeFsstStreams(std::span<const std::string_view> values,
                                                  PhysicalLevelTechnique physicalTechnique,
                                                  IntegerEncoder& intEncoder,
                                                  bool isShared) {
        using namespace metadata::stream;
        namespace fsst = util::encoding::fsst;

        std::vector<std::uint8_t> joined;
        std::vector<std::int32_t> valueLengths;
        valueLengths.reserve(values.size());
        for (const auto sv : values) {
            valueLengths.push_back(static_cast<std::int32_t>(sv.size()));
            joined.insert(joined.end(), sv.begin(), sv.end());
        }

        const auto result = fsst::encode(joined);

        std::vector<std::int32_t> symLens(result.symbolLengths.begin(), result.symbolLengths.end());
        const auto encodedSymbolLengths = intEncoder.encodeIntStream(
            symLens, physicalTechnique, false, PhysicalStreamType::LENGTH, LogicalStreamType{LengthType::SYMBOL});

        const auto symbolTableMetadata = StreamMetadata(PhysicalStreamType::DATA,
                                                        LogicalStreamType{DictionaryType::FSST},
                                                        LogicalLevelTechnique::NONE,
                                                        LogicalLevelTechnique::NONE,
                                                        PhysicalLevelTechnique::NONE,
                                                        static_cast<std::uint32_t>(result.symbolLengths.size()),
                                                        static_cast<std::uint32_t>(result.symbols.size()))
                                             .encode();

        const auto encodedDictLengths = intEncoder.encodeIntStream(valueLengths,
                                                                   physicalTechnique,
                                                                   false,
                                                                   PhysicalStreamType::LENGTH,
                                                                   LogicalStreamType{LengthType::DICTIONARY});

        const auto dictType = isShared ? DictionaryType::SHARED : DictionaryType::SINGLE;
        const auto corpusMetadata = StreamMetadata(PhysicalStreamType::DATA,
                                                   LogicalStreamType{dictType},
                                                   LogicalLevelTechnique::NONE,
                                                   LogicalLevelTechnique::NONE,
                                                   PhysicalLevelTechnique::NONE,
                                                   static_cast<std::uint32_t>(values.size()),
                                                   static_cast<std::uint32_t>(result.compressedData.size()))
                                        .encode();

        std::vector<std::uint8_t> remainder;
        remainder.reserve(symbolTableMetadata.size() + result.symbols.size() + encodedDictLengths.size() +
                          corpusMetadata.size() + result.compressedData.size());
        remainder.insert(remainder.end(), symbolTableMetadata.begin(), symbolTableMetadata.end());
        remainder.insert(remainder.end(), result.symbols.begin(), result.symbols.end());
        remainder.insert(remainder.end(), encodedDictLengths.begin(), encodedDictLengths.end());
        remainder.insert(remainder.end(), corpusMetadata.begin(), corpusMetadata.end());
        remainder.insert(remainder.end(), result.compressedData.begin(), result.compressedData.end());

        return {
            .firstStream = std::move(encodedSymbolLengths),
            .remainingStreams = std::move(remainder),
        };
    }

    static EncodedStringStreams encodeStringStreams(std::span<const std::string_view> values,
                                                    PhysicalLevelTechnique physicalTechnique,
                                                    IntegerEncoder& intEncoder,
                                                    metadata::stream::LengthType lengthType,
                                                    metadata::stream::DictionaryType dictType) {
        using namespace metadata::stream;

        std::vector<std::int32_t> lengths;
        lengths.reserve(values.size());
        std::vector<std::uint8_t> rawData;
        for (const auto sv : values) {
            lengths.push_back(static_cast<std::int32_t>(sv.size()));
            rawData.insert(rawData.end(), sv.begin(), sv.end());
        }

        const auto encodedLengths = intEncoder.encodeIntStream(
            lengths, physicalTechnique, false, PhysicalStreamType::LENGTH, LogicalStreamType{lengthType});

        const auto dataMetadata = StreamMetadata(PhysicalStreamType::DATA,
                                                 LogicalStreamType{dictType},
                                                 LogicalLevelTechnique::NONE,
                                                 LogicalLevelTechnique::NONE,
                                                 PhysicalLevelTechnique::NONE,
                                                 static_cast<std::uint32_t>(values.size()),
                                                 static_cast<std::uint32_t>(rawData.size()))
                                      .encode();

        std::vector<std::uint8_t> remaining;
        remaining.reserve(dataMetadata.size() + rawData.size());
        remaining.insert(remaining.end(), dataMetadata.begin(), dataMetadata.end());
        remaining.insert(remaining.end(), rawData.begin(), rawData.end());

        return {
            .firstStream = std::move(encodedLengths),
            .remainingStreams = std::move(remaining),
        };
    }

    static std::vector<std::uint8_t> encodeStringBytes(std::span<const std::string_view> values,
                                                       PhysicalLevelTechnique physicalTechnique,
                                                       IntegerEncoder& intEncoder,
                                                       metadata::stream::LengthType lengthType,
                                                       metadata::stream::DictionaryType dictType) {
        auto streams = encodeStringStreams(values, physicalTechnique, intEncoder, lengthType, dictType);
        std::vector<std::uint8_t> result;
        result.reserve(streams.firstStream.size() + streams.remainingStreams.size());
        result.insert(result.end(), streams.firstStream.begin(), streams.firstStream.end());
        result.insert(result.end(), streams.remainingStreams.begin(), streams.remainingStreams.end());
        return result;
    }
};

} // namespace mlt::encoder
