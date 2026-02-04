#pragma once

#include <mlt/encode/int.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/util/encoding/varint.hpp>

#include <cstdint>
#include <span>
#include <string>
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

    /// @param values Non-null string values (nulls handled by the present stream in the caller)
    static EncodeResult encode(std::span<const std::string_view> values,
                               PhysicalLevelTechnique physicalTechnique,
                               IntegerEncoder& intEncoder) {
        auto plain = encodePlain(values, physicalTechnique, intEncoder);
        auto dict = encodeDictionary(values, physicalTechnique, intEncoder);

        if (dict.size() < plain.size()) {
            return {3, std::move(dict)};
        }
        return {2, std::move(plain)};
    }

    static EncodeResult encodeSharedDictionary(
        const std::vector<std::vector<const std::string_view*>>& columns,
        PhysicalLevelTechnique physicalTechnique,
        IntegerEncoder& intEncoder) {
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
            return {0, {}};
        }

        auto dictData = encodeDictionaryData(dictionary, physicalTechnique, intEncoder, true);

        std::vector<std::uint8_t> result;
        result.insert(result.end(), dictData.begin(), dictData.end());

        for (std::size_t col = 0; col < columns.size(); ++col) {
            if (dataStreams[col].empty()) {
                util::encoding::encodeVarint(static_cast<std::uint32_t>(0), result);
                continue;
            }

            util::encoding::encodeVarint(static_cast<std::uint32_t>(2), result);

            auto presentData = BooleanEncoder::encodeBooleanStream(
                presentStreams[col], PhysicalStreamType::PRESENT);
            result.insert(result.end(), presentData.begin(), presentData.end());

            auto offsetData = intEncoder.encodeIntStream(
                dataStreams[col], physicalTechnique, false,
                PhysicalStreamType::OFFSET, LogicalStreamType{OffsetType::STRING});
            result.insert(result.end(), offsetData.begin(), offsetData.end());
        }

        const auto numStreams = 3 + static_cast<std::uint32_t>(columns.size()) * 2;
        return {numStreams, std::move(result)};
    }

private:
    static std::vector<std::uint8_t> encodePlain(std::span<const std::string_view> values,
                                                  PhysicalLevelTechnique physicalTechnique,
                                                  IntegerEncoder& intEncoder) {
        using namespace metadata::stream;

        std::vector<std::int32_t> lengths;
        lengths.reserve(values.size());
        std::vector<std::uint8_t> rawData;
        for (const auto sv : values) {
            lengths.push_back(static_cast<std::int32_t>(sv.size()));
            rawData.insert(rawData.end(), sv.begin(), sv.end());
        }

        auto encodedLengths = intEncoder.encodeIntStream(
            lengths, physicalTechnique, false,
            PhysicalStreamType::LENGTH, LogicalStreamType{LengthType::VAR_BINARY});

        auto dataMetadata = StreamMetadata(
                                PhysicalStreamType::DATA, LogicalStreamType{DictionaryType::NONE},
                                LogicalLevelTechnique::NONE, LogicalLevelTechnique::NONE,
                                PhysicalLevelTechnique::NONE,
                                static_cast<std::uint32_t>(values.size()),
                                static_cast<std::uint32_t>(rawData.size()))
                                .encode();

        std::vector<std::uint8_t> result;
        result.reserve(encodedLengths.size() + dataMetadata.size() + rawData.size());
        result.insert(result.end(), encodedLengths.begin(), encodedLengths.end());
        result.insert(result.end(), dataMetadata.begin(), dataMetadata.end());
        result.insert(result.end(), rawData.begin(), rawData.end());
        return result;
    }

    static std::vector<std::uint8_t> encodeDictionary(std::span<const std::string_view> values,
                                                       PhysicalLevelTechnique physicalTechnique,
                                                       IntegerEncoder& intEncoder) {
        using namespace metadata::stream;

        std::vector<std::string_view> dictionary;
        std::unordered_map<std::string_view, std::uint32_t> dictIndex;
        std::vector<std::int32_t> offsets;
        offsets.reserve(values.size());

        for (const auto sv : values) {
            auto [it, inserted] = dictIndex.try_emplace(sv, static_cast<std::uint32_t>(dictionary.size()));
            if (inserted) {
                dictionary.push_back(sv);
            }
            offsets.push_back(static_cast<std::int32_t>(it->second));
        }

        auto dictData = encodeDictionaryData(dictionary, physicalTechnique, intEncoder, false);

        auto encodedOffsets = intEncoder.encodeIntStream(
            offsets, physicalTechnique, false,
            PhysicalStreamType::OFFSET, LogicalStreamType{OffsetType::STRING});

        std::vector<std::uint8_t> result;
        result.reserve(dictData.size() + encodedOffsets.size());
        result.insert(result.end(), dictData.begin(), dictData.end());
        result.insert(result.end(), encodedOffsets.begin(), encodedOffsets.end());
        return result;
    }

    static std::vector<std::uint8_t> encodeDictionaryData(
        std::span<const std::string_view> dictionary,
        PhysicalLevelTechnique physicalTechnique,
        IntegerEncoder& intEncoder,
        bool isShared) {
        using namespace metadata::stream;

        std::vector<std::int32_t> lengths;
        lengths.reserve(dictionary.size());
        std::vector<std::uint8_t> rawData;
        for (const auto sv : dictionary) {
            lengths.push_back(static_cast<std::int32_t>(sv.size()));
            rawData.insert(rawData.end(), sv.begin(), sv.end());
        }

        auto encodedLengths = intEncoder.encodeIntStream(
            lengths, physicalTechnique, false,
            PhysicalStreamType::LENGTH, LogicalStreamType{LengthType::DICTIONARY});

        const auto dictType = isShared ? DictionaryType::SHARED : DictionaryType::SINGLE;
        auto dataMetadata = StreamMetadata(
                                PhysicalStreamType::DATA, LogicalStreamType{dictType},
                                LogicalLevelTechnique::NONE, LogicalLevelTechnique::NONE,
                                PhysicalLevelTechnique::NONE,
                                static_cast<std::uint32_t>(dictionary.size()),
                                static_cast<std::uint32_t>(rawData.size()))
                                .encode();

        std::vector<std::uint8_t> result;
        result.reserve(encodedLengths.size() + dataMetadata.size() + rawData.size());
        result.insert(result.end(), encodedLengths.begin(), encodedLengths.end());
        result.insert(result.end(), dataMetadata.begin(), dataMetadata.end());
        result.insert(result.end(), rawData.begin(), rawData.end());
        return result;
    }
};

} // namespace mlt::encoder
