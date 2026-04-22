#pragma once

#include <algorithm>
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

    static std::vector<std::uint8_t> encodeBooleanColumn(std::span<const std::optional<bool>> values,
                                                         bool columnNullable = false) {
        const auto [presentValues, dataValues, hasNull] = separateNulls<bool>(values);

        std::vector<std::uint8_t> result;
        appendPresentStream(result, presentValues, hasNull, columnNullable);

        const auto dataStream = BooleanEncoder::encodeBooleanStream(dataValues,
                                                                    metadata::stream::PhysicalStreamType::DATA);
        result.insert(result.end(), dataStream.begin(), dataStream.end());
        return result;
    }

    /// Unsigned integers are encoded separately to avoid undefined behvior when casting unsigned values outside the
    /// range of signed integers.

    static std::vector<std::uint8_t> encodeInt32Column(std::span<const std::optional<std::int32_t>> values,
                                                       PhysicalLevelTechnique physicalTechnique,
                                                       bool isSigned,
                                                       IntegerEncoder& intEncoder,
                                                       bool columnNullable = false) {
        return encodeIntColumn<std::int32_t>(values, physicalTechnique, isSigned, intEncoder, columnNullable);
    }

    static std::vector<std::uint8_t> encodeUint32Column(std::span<const std::uint32_t> values,
                                                        PhysicalLevelTechnique physicalTechnique,
                                                        IntegerEncoder& intEncoder) {
        return intEncoder.encodeUint32Stream(
            values, physicalTechnique, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
    }

    static std::vector<std::uint8_t> encodeUint32Column(std::span<const std::optional<std::uint32_t>> values,
                                                        PhysicalLevelTechnique physicalTechnique,
                                                        IntegerEncoder& intEncoder,
                                                        bool columnNullable = false) {
        return encodeOptionalDataColumn<std::uint32_t>(
            values, columnNullable, [&](std::span<const std::uint32_t> dataValues) {
                return intEncoder.encodeUint32Stream(
                    dataValues, physicalTechnique, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
            });
    }

    static std::vector<std::uint8_t> encodeUint64Column(std::span<const std::uint64_t> values,
                                                        IntegerEncoder& intEncoder) {
        return intEncoder.encodeUint64Stream(values, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
    }

    static std::vector<std::uint8_t> encodeUint64Column(std::span<const std::optional<std::uint64_t>> values,
                                                        IntegerEncoder& intEncoder,
                                                        bool columnNullable = false) {
        return encodeOptionalDataColumn<std::uint64_t>(
            values, columnNullable, [&](std::span<const std::uint64_t> dataValues) {
                return intEncoder.encodeUint64Stream(
                    dataValues, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
            });
    }

    static std::vector<std::uint8_t> encodeInt64Column(std::span<const std::optional<std::int64_t>> values,
                                                       bool isSigned,
                                                       IntegerEncoder& intEncoder,
                                                       bool columnNullable = false) {
        return encodeOptionalDataColumn<std::int64_t>(
            values, columnNullable, [&](std::span<const std::int64_t> dataValues) {
                return intEncoder.encodeLongStream(
                    dataValues, isSigned, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
            });
    }

    template <typename T>
        requires(std::same_as<T, float> || std::same_as<T, double>)
    static std::vector<std::uint8_t> encodeFloatingPointColumn(std::span<const std::optional<T>> values,
                                                               bool columnNullable = false) {
        return encodeOptionalDataColumn<T>(values, columnNullable, [](std::span<const T> dataValues) {
            return FloatEncoder::encodeStream(dataValues);
        });
    }

    static std::vector<std::uint8_t> encodeFloatColumn(std::span<const std::optional<float>> values,
                                                       bool columnNullable = false) {
        return encodeFloatingPointColumn(values, columnNullable);
    }

    static std::vector<std::uint8_t> encodeDoubleColumn(std::span<const std::optional<double>> values,
                                                        bool columnNullable = false) {
        return encodeFloatingPointColumn(values, columnNullable);
    }

    static std::vector<std::uint8_t> encodeStringColumn(std::span<const std::optional<std::string_view>> values,
                                                        PhysicalLevelTechnique physicalTechnique,
                                                        IntegerEncoder& intEncoder,
                                                        bool useFsst = true,
                                                        bool columnNullable = false) {
        std::vector<bool> presentValues;
        std::vector<std::string_view> dataValues;
        for (const auto& v : values) {
            if (columnNullable) {
                presentValues.push_back(v.has_value());
            }
            if (!columnNullable || v.has_value()) {
                dataValues.push_back(v.has_value() ? *v : std::string_view{});
            }
        }

        const auto stringResult = StringEncoder::encode(dataValues, physicalTechnique, intEncoder, useFsst);

        const auto streamCount = stringResult.numStreams + (columnNullable ? 1 : 0);

        std::vector<std::uint8_t> result;
        util::encoding::encodeVarint(streamCount, result);
        if (columnNullable) {
            const auto presentStream = BooleanEncoder::encodeBooleanStream(presentValues, PhysicalStreamType::PRESENT);
            result.insert(result.end(), presentStream.begin(), presentStream.end());
        }
        result.insert(result.end(), stringResult.data.begin(), stringResult.data.end());
        return result;
    }

private:
    template <typename T>
    struct SeparatedNulls {
        std::vector<bool> present;
        std::vector<T> data;
        bool hasNull;
    };

    template <typename T>
    static SeparatedNulls<T> separateNulls(std::span<const std::optional<T>> values) {
        SeparatedNulls<T> result;
        result.hasNull = false;
        for (const auto& v : values) {
            if (v.has_value()) {
                result.present.push_back(true);
                result.data.push_back(*v);
            } else {
                result.present.push_back(false);
                result.hasNull = true;
            }
        }
        return result;
    }

    static void appendPresentStream(std::vector<std::uint8_t>& result,
                                    const std::vector<bool>& presentValues,
                                    bool hasNull,
                                    bool columnNullable = false) {
        if (hasNull || columnNullable) {
            auto presentStream = BooleanEncoder::encodeBooleanStream(presentValues, PhysicalStreamType::PRESENT);
            result.insert(result.end(), presentStream.begin(), presentStream.end());
        }
    }

    template <typename T, typename EncodeDataFn>
    static std::vector<std::uint8_t> encodeOptionalDataColumn(std::span<const std::optional<T>> values,
                                                              bool columnNullable,
                                                              EncodeDataFn&& encodeData) {
        const auto [presentValues, dataValues, hasNull] = separateNulls<T>(values);

        std::vector<std::uint8_t> result;
        appendPresentStream(result, presentValues, hasNull, columnNullable);

        const auto dataStream = encodeData(std::span<const T>{dataValues});
        result.insert(result.end(), dataStream.begin(), dataStream.end());
        return result;
    }

    template <typename T>
    static std::vector<std::uint8_t> encodeIntColumn(std::span<const std::optional<T>> values,
                                                     PhysicalLevelTechnique physicalTechnique,
                                                     bool isSigned,
                                                     IntegerEncoder& intEncoder,
                                                     bool columnNullable = false) {
        return encodeOptionalDataColumn<T>(values, columnNullable, [&](std::span<const T> rawValues) {
            std::vector<std::int32_t> dataValues(rawValues.size());
            std::transform(rawValues.begin(), rawValues.end(), dataValues.begin(), [](auto v) {
                return static_cast<std::int32_t>(v);
            });
            return intEncoder.encodeIntStream(
                dataValues, physicalTechnique, isSigned, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
        });
    }
};

} // namespace mlt::encoder
