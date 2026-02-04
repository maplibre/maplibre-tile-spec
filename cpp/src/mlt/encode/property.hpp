#pragma once

#include <mlt/encode/boolean.hpp>
#include <mlt/encode/float.hpp>
#include <mlt/encode/int.hpp>
#include <mlt/encode/string.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/encoding/varint.hpp>

#include <cstdint>
#include <optional>
#include <span>
#include <string_view>
#include <vector>

namespace mlt::encoder {

class PropertyEncoder {
public:
    using ScalarType = metadata::tileset::ScalarType;
    using PhysicalLevelTechnique = metadata::stream::PhysicalLevelTechnique;
    using PhysicalStreamType = metadata::stream::PhysicalStreamType;

    static std::vector<std::uint8_t> encodeBooleanColumn(std::span<const std::optional<bool>> values) {
        std::vector<bool> presentValues;
        std::vector<bool> dataValues;
        bool hasNull = false;
        for (const auto& v : values) {
            if (v.has_value()) {
                presentValues.push_back(true);
                dataValues.push_back(*v);
            } else {
                presentValues.push_back(false);
                hasNull = true;
            }
        }

        std::vector<std::uint8_t> result;
        if (hasNull) {
            auto presentStream = BooleanEncoder::encodeBooleanStream(presentValues, PhysicalStreamType::PRESENT);
            result.insert(result.end(), presentStream.begin(), presentStream.end());
        }

        auto dataStream = BooleanEncoder::encodeBooleanStream(dataValues, metadata::stream::PhysicalStreamType::DATA);
        result.insert(result.end(), dataStream.begin(), dataStream.end());
        return result;
    }

    static std::vector<std::uint8_t> encodeInt32Column(std::span<const std::optional<std::int32_t>> values,
                                                       PhysicalLevelTechnique physicalTechnique,
                                                       bool isSigned,
                                                       IntegerEncoder& intEncoder) {
        return encodeIntColumn<std::int32_t>(values, physicalTechnique, isSigned, intEncoder);
    }

    static std::vector<std::uint8_t> encodeUint32Column(std::span<const std::uint32_t> values,
                                                        PhysicalLevelTechnique physicalTechnique,
                                                        IntegerEncoder& intEncoder) {
        std::vector<std::int32_t> signedValues(values.size());
        std::transform(
            values.begin(), values.end(), signedValues.begin(), [](auto v) { return static_cast<std::int32_t>(v); });
        return intEncoder.encodeIntStream(
            signedValues, physicalTechnique, false, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
    }

    static std::vector<std::uint8_t> encodeUint64Column(std::span<const std::uint64_t> values,
                                                        IntegerEncoder& intEncoder) {
        std::vector<std::int64_t> signedValues(values.size());
        std::transform(
            values.begin(), values.end(), signedValues.begin(), [](auto v) { return static_cast<std::int64_t>(v); });
        return intEncoder.encodeLongStream(
            signedValues, false, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
    }

    static std::vector<std::uint8_t> encodeInt64Column(std::span<const std::optional<std::int64_t>> values,
                                                       bool isSigned,
                                                       IntegerEncoder& intEncoder) {
        std::vector<bool> presentValues;
        std::vector<std::int64_t> dataValues;
        bool hasNull = false;
        for (const auto& v : values) {
            if (v.has_value()) {
                presentValues.push_back(true);
                dataValues.push_back(*v);
            } else {
                presentValues.push_back(false);
                hasNull = true;
            }
        }

        std::vector<std::uint8_t> result;
        if (hasNull) {
            auto presentStream = BooleanEncoder::encodeBooleanStream(presentValues, PhysicalStreamType::PRESENT);
            result.insert(result.end(), presentStream.begin(), presentStream.end());
        }

        auto dataStream = intEncoder.encodeLongStream(
            dataValues, isSigned, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
        result.insert(result.end(), dataStream.begin(), dataStream.end());
        return result;
    }

    template <typename T>
        requires(std::same_as<T, float> || std::same_as<T, double>)
    static std::vector<std::uint8_t> encodeFloatingPointColumn(std::span<const std::optional<T>> values) {
        std::vector<bool> presentValues;
        std::vector<T> dataValues;
        bool hasNull = false;
        for (const auto& v : values) {
            if (v.has_value()) {
                presentValues.push_back(true);
                dataValues.push_back(*v);
            } else {
                presentValues.push_back(false);
                hasNull = true;
            }
        }

        std::vector<std::uint8_t> result;
        if (hasNull) {
            auto presentStream = BooleanEncoder::encodeBooleanStream(presentValues, PhysicalStreamType::PRESENT);
            result.insert(result.end(), presentStream.begin(), presentStream.end());
        }

        auto dataStream = FloatEncoder::encodeStream(std::span<const T>{dataValues});
        result.insert(result.end(), dataStream.begin(), dataStream.end());
        return result;
    }

    static std::vector<std::uint8_t> encodeFloatColumn(std::span<const std::optional<float>> values) {
        return encodeFloatingPointColumn(values);
    }

    static std::vector<std::uint8_t> encodeDoubleColumn(std::span<const std::optional<double>> values) {
        return encodeFloatingPointColumn(values);
    }

    static std::vector<std::uint8_t> encodeStringColumn(std::span<const std::optional<std::string_view>> values,
                                                        PhysicalLevelTechnique physicalTechnique,
                                                        IntegerEncoder& intEncoder) {
        std::vector<bool> presentValues;
        std::vector<std::string_view> dataValues;
        for (const auto& v : values) {
            if (v.has_value()) {
                presentValues.push_back(true);
                dataValues.push_back(*v);
            } else {
                presentValues.push_back(false);
            }
        }

        auto presentStream = BooleanEncoder::encodeBooleanStream(presentValues, PhysicalStreamType::PRESENT);

        auto stringResult = StringEncoder::encode(dataValues, physicalTechnique, intEncoder);

        const auto streamCount = stringResult.numStreams + 1;

        std::vector<std::uint8_t> result;
        util::encoding::encodeVarint(streamCount, result);
        result.insert(result.end(), presentStream.begin(), presentStream.end());
        result.insert(result.end(), stringResult.data.begin(), stringResult.data.end());
        return result;
    }

private:
    template <typename T>
    static std::vector<std::uint8_t> encodeIntColumn(std::span<const std::optional<T>> values,
                                                     PhysicalLevelTechnique physicalTechnique,
                                                     bool isSigned,
                                                     IntegerEncoder& intEncoder) {
        std::vector<bool> presentValues;
        std::vector<std::int32_t> dataValues;
        bool hasNull = false;
        for (const auto& v : values) {
            if (v.has_value()) {
                presentValues.push_back(true);
                dataValues.push_back(static_cast<std::int32_t>(*v));
            } else {
                presentValues.push_back(false);
                hasNull = true;
            }
        }

        std::vector<std::uint8_t> result;
        if (hasNull) {
            auto presentStream = BooleanEncoder::encodeBooleanStream(presentValues, PhysicalStreamType::PRESENT);
            result.insert(result.end(), presentStream.begin(), presentStream.end());
        }

        auto dataStream = intEncoder.encodeIntStream(
            dataValues, physicalTechnique, isSigned, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
        result.insert(result.end(), dataStream.begin(), dataStream.end());
        return result;
    }
};

} // namespace mlt::encoder
