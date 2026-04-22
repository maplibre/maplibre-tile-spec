#pragma once

#include <algorithm>
#include <mlt/encode/boolean.hpp>
#include <mlt/encode/float.hpp>
#include <mlt/encode/int.hpp>
#include <mlt/encode/string.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/encoding/varint.hpp>
#include <mlt/util/stl.hpp>

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
    using EncodedChunks = std::vector<std::vector<std::uint8_t>>;

    struct EncodeChunkedResult {
        EncodedChunks chunks;
    };

    template <typename T, typename EncodeDataFn>
    static EncodeChunkedResult encodeSeparatedDataColumnChunked(std::span<const T> dataValues,
                                                                const std::vector<bool>& presentValues,
                                                                bool hasNull,
                                                                bool columnNullable,
                                                                EncodeDataFn&& encodeData) {
        EncodedChunks chunks;
        chunks.reserve(2);
        appendPresentStream(chunks, presentValues, hasNull, columnNullable);

        auto dataStream = encodeData(dataValues);
        chunks.push_back(std::move(dataStream));
        return {.chunks = std::move(chunks)};
    }

    static std::vector<std::uint8_t> encodeBooleanColumn(std::span<const std::optional<bool>> values,
                                                         bool columnNullable = false) {
        return flattenChunks(encodeBooleanColumnChunked(values, columnNullable).chunks);
    }

    static EncodeChunkedResult encodeBooleanColumnChunked(std::span<const std::optional<bool>> values,
                                                          bool columnNullable = false) {
        const auto [presentValues, dataValues, hasNull] = separateNulls<bool>(values);

        EncodedChunks chunks;
        chunks.reserve(2);
        appendPresentStream(chunks, presentValues, hasNull, columnNullable);

        auto dataStream = BooleanEncoder::encodeBooleanStream(dataValues, metadata::stream::PhysicalStreamType::DATA);
        chunks.push_back(std::move(dataStream));
        return {.chunks = std::move(chunks)};
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

    static EncodeChunkedResult encodeInt32ColumnChunked(std::span<const std::optional<std::int32_t>> values,
                                                        PhysicalLevelTechnique physicalTechnique,
                                                        bool isSigned,
                                                        IntegerEncoder& intEncoder,
                                                        bool columnNullable = false) {
        return encodeOptionalDataColumnChunked<std::int32_t>(
            values, columnNullable, [&](std::span<const std::int32_t> dataValues) {
                return intEncoder.encodeIntStream(
                    dataValues, physicalTechnique, isSigned, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
            });
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
        return flattenChunks(encodeUint32ColumnChunked(values, physicalTechnique, intEncoder, columnNullable).chunks);
    }

    static EncodeChunkedResult encodeUint32ColumnChunked(std::span<const std::optional<std::uint32_t>> values,
                                                         PhysicalLevelTechnique physicalTechnique,
                                                         IntegerEncoder& intEncoder,
                                                         bool columnNullable = false) {
        return encodeOptionalDataColumnChunked<std::uint32_t>(
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
        return flattenChunks(encodeUint64ColumnChunked(values, intEncoder, columnNullable).chunks);
    }

    static EncodeChunkedResult encodeUint64ColumnChunked(std::span<const std::optional<std::uint64_t>> values,
                                                         IntegerEncoder& intEncoder,
                                                         bool columnNullable = false) {
        return encodeOptionalDataColumnChunked<std::uint64_t>(
            values, columnNullable, [&](std::span<const std::uint64_t> dataValues) {
                return intEncoder.encodeUint64Stream(
                    dataValues, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
            });
    }

    static std::vector<std::uint8_t> encodeInt64Column(std::span<const std::optional<std::int64_t>> values,
                                                       bool isSigned,
                                                       IntegerEncoder& intEncoder,
                                                       bool columnNullable = false) {
        return flattenChunks(encodeInt64ColumnChunked(values, isSigned, intEncoder, columnNullable).chunks);
    }

    static EncodeChunkedResult encodeInt64ColumnChunked(std::span<const std::optional<std::int64_t>> values,
                                                        bool isSigned,
                                                        IntegerEncoder& intEncoder,
                                                        bool columnNullable = false) {
        return encodeOptionalDataColumnChunked<std::int64_t>(
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

    static EncodeChunkedResult encodeFloatColumnChunked(std::span<const std::optional<float>> values,
                                                        bool columnNullable = false) {
        return encodeOptionalDataColumnChunked<float>(values, columnNullable, [](std::span<const float> dataValues) {
            return FloatEncoder::encodeStream(dataValues);
        });
    }

    static std::vector<std::uint8_t> encodeDoubleColumn(std::span<const std::optional<double>> values,
                                                        bool columnNullable = false) {
        return encodeFloatingPointColumn(values, columnNullable);
    }

    static EncodeChunkedResult encodeDoubleColumnChunked(std::span<const std::optional<double>> values,
                                                         bool columnNullable = false) {
        return encodeOptionalDataColumnChunked<double>(values, columnNullable, [](std::span<const double> dataValues) {
            return FloatEncoder::encodeStream(dataValues);
        });
    }

    static std::vector<std::uint8_t> encodeStringColumn(std::span<const std::optional<std::string_view>> values,
                                                        PhysicalLevelTechnique physicalTechnique,
                                                        IntegerEncoder& intEncoder,
                                                        bool useFsst = true,
                                                        bool columnNullable = false) {
        return flattenChunks(
            encodeStringColumnChunked(values, physicalTechnique, intEncoder, useFsst, columnNullable).chunks);
    }

    static EncodeChunkedResult encodeStringColumnChunked(std::span<const std::optional<std::string_view>> values,
                                                         PhysicalLevelTechnique physicalTechnique,
                                                         IntegerEncoder& intEncoder,
                                                         bool useFsst = true,
                                                         bool columnNullable = false) {
        std::vector<bool> presentValues;
        std::vector<std::string_view> dataValues;
        if (columnNullable) {
            presentValues.reserve(values.size());
        }
        dataValues.reserve(values.size());
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

        EncodedChunks chunks;
        chunks.reserve(columnNullable ? 3 : 2);
        std::vector<std::uint8_t> streamCountBytes;
        util::encoding::encodeVarint(streamCount, streamCountBytes);
        chunks.push_back(std::move(streamCountBytes));
        if (columnNullable) {
            auto presentStream = BooleanEncoder::encodeBooleanStream(presentValues, PhysicalStreamType::PRESENT);
            chunks.push_back(std::move(presentStream));
        }
        chunks.push_back(std::move(stringResult.data));
        return {.chunks = std::move(chunks)};
    }

    static EncodeChunkedResult encodeStringColumnChunkedFromSeparated(std::span<const std::string_view> dataValues,
                                                                      const std::vector<bool>& presentValues,
                                                                      PhysicalLevelTechnique physicalTechnique,
                                                                      IntegerEncoder& intEncoder,
                                                                      bool useFsst = true,
                                                                      bool columnNullable = false) {
        const auto stringResult = StringEncoder::encode(dataValues, physicalTechnique, intEncoder, useFsst);
        const auto streamCount = stringResult.numStreams + (columnNullable ? 1 : 0);

        EncodedChunks chunks;
        chunks.reserve(3);
        std::vector<std::uint8_t> streamCountBytes;
        util::encoding::encodeVarint(streamCount, streamCountBytes);
        chunks.push_back(std::move(streamCountBytes));
        if (columnNullable) {
            auto presentStream = BooleanEncoder::encodeBooleanStream(presentValues, PhysicalStreamType::PRESENT);
            chunks.push_back(std::move(presentStream));
        }
        chunks.push_back(std::move(stringResult.data));
        return {.chunks = std::move(chunks)};
    }

private:
    template <typename T>
    // Represents an optional column split into a present bitmap, compacted non-null values,
    // and a fast flag indicating whether any null was observed.
    struct SeparatedNulls {
        std::vector<bool> present;
        std::vector<T> data;
        bool hasNull = false;
    };

    template <typename T>
    // Converts optional values into SeparatedNulls so downstream encoders can process
    // present and data streams independently without re-scanning the input.
    static SeparatedNulls<T> separateNulls(std::span<const std::optional<T>> values) {
        SeparatedNulls<T> result;
        result.present.reserve(values.size());
        result.data.reserve(values.size());
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

    static void appendPresentStream(EncodedChunks& chunks,
                                    const std::vector<bool>& presentValues,
                                    bool hasNull,
                                    bool columnNullable = false) {
        if (hasNull || columnNullable) {
            auto presentStream = BooleanEncoder::encodeBooleanStream(presentValues, PhysicalStreamType::PRESENT);
            chunks.push_back(std::move(presentStream));
        }
    }

    template <typename T, typename EncodeDataFn>
    static std::vector<std::uint8_t> encodeOptionalDataColumn(std::span<const std::optional<T>> values,
                                                              bool columnNullable,
                                                              EncodeDataFn&& encodeData) {
        return flattenChunks(
            encodeOptionalDataColumnChunked(values, columnNullable, std::forward<EncodeDataFn>(encodeData)).chunks);
    }

    template <typename T, typename EncodeDataFn>
    static EncodeChunkedResult encodeOptionalDataColumnChunked(std::span<const std::optional<T>> values,
                                                               bool columnNullable,
                                                               EncodeDataFn&& encodeData) {
        const auto [presentValues, dataValues, hasNull] = separateNulls<T>(values);
        return encodeSeparatedDataColumnChunked(
            std::span<const T>{dataValues}, presentValues, hasNull, columnNullable, encodeData);
    }

    static std::vector<std::uint8_t> flattenChunks(EncodedChunks chunks) {
        const auto totalSize = util::sum(chunks, [](const auto& chunk) { return chunk.size(); });

        std::vector<std::uint8_t> result;
        result.reserve(totalSize);
        for (const auto& chunk : chunks) {
            result.insert(result.end(), chunk.begin(), chunk.end());
        }
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
