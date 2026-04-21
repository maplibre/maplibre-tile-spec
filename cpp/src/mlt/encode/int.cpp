#include <algorithm>
#include <mlt/encoder.hpp>
#include <mlt/encode/int.hpp>

#if MLT_WITH_FASTPFOR
#include <compositecodec.h>
#include <fastpfor.h>
#include <variablebyte.h>
#include <stdexcept>
#endif

#include <bit>
#include <cassert>

namespace mlt::encoder {

using namespace metadata::stream;

struct IntegerEncoder::Impl {
#if MLT_WITH_FASTPFOR
    FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte> codec;
#endif
    IntegerEncodingOption encodingOption = IntegerEncodingOption::AUTO;
};

IntegerEncoder::IntegerEncoder()
    : impl(std::make_unique<Impl>()) {}

IntegerEncoder::~IntegerEncoder() noexcept = default;

void IntegerEncoder::setDefaultEncodingOption(IntegerEncodingOption option) {
    impl->encodingOption = option;
}

std::vector<std::uint8_t> IntegerEncoder::encodeVarints32(std::span<const std::int32_t> values, bool zigZag) {
    std::vector<std::uint8_t> result;
    result.reserve(values.size() * 2);
    for (const auto v : values) {
        const auto encoded = zigZag ? util::encoding::encodeZigZag(v) : static_cast<std::uint32_t>(v);
        util::encoding::encodeVarint(encoded, result);
    }
    return result;
}

std::vector<std::uint8_t> IntegerEncoder::encodeVarints64(std::span<const std::int64_t> values, bool zigZag) {
    std::vector<std::uint8_t> result;
    result.reserve(values.size() * 3);
    for (const auto v : values) {
        const auto encoded = zigZag ? util::encoding::encodeZigZag(v) : static_cast<std::uint64_t>(v);
        util::encoding::encodeVarint(encoded, result);
    }
    return result;
}

std::vector<std::uint8_t> IntegerEncoder::encodeFastPfor([[maybe_unused]] std::span<const std::int32_t> values,
                                                         [[maybe_unused]] bool zigZag) {
#if MLT_WITH_FASTPFOR
    std::vector<std::uint32_t> input(values.size());
    if (zigZag) {
        std::ranges::transform(values, input.begin(), [](auto v) { return util::encoding::encodeZigZag(v); });
    } else {
        std::ranges::transform(values, input.begin(), [](auto v) { return static_cast<std::uint32_t>(v); });
    }

    // FastPFor can throw NotEnoughStorage for incompressible/alignment-sensitive inputs.
    // Start with the common-case size and retry with larger buffers as needed.
    std::vector<std::uint32_t> compressed(input.size() + 1024);
    std::size_t compressedSize = 0;
    bool encoded = false;
    for (int attempt = 0; attempt < 4 && !encoded; ++attempt) {
        compressedSize = compressed.size();
        try {
            impl->codec.encodeArray(input.data(), input.size(), compressed.data(), compressedSize);
            encoded = true;
        } catch (const FastPForLib::NotEnoughStorage&) {
            compressed.resize(compressed.size() * 2);
        }
    }

    if (!encoded) {
        throw std::runtime_error("FastPFOR encoding failed: insufficient output buffer");
    }

    std::vector<std::uint8_t> result(compressedSize * sizeof(std::uint32_t));
    auto* outWords = reinterpret_cast<std::uint32_t*>(result.data());
    for (std::size_t i = 0; i < compressedSize; ++i) {
        outWords[i] = std::byteswap(compressed[i]);
    }
    return result;
#else
    throw std::runtime_error("FastPFOR encoding is not enabled. Configure with MLT_WITH_FASTPFOR=ON");
#endif
}

IntegerEncodingResult IntegerEncoder::encodeInt(std::span<const std::int32_t> values,
                                                [[maybe_unused]] PhysicalLevelTechnique physicalTechnique,
                                                bool isSigned) {
    return encodeInt(values, physicalTechnique, isSigned, impl->encodingOption);
}

IntegerEncodingResult IntegerEncoder::encodeInt(std::span<const std::int32_t> values,
                                                [[maybe_unused]] PhysicalLevelTechnique physicalTechnique,
                                                bool isSigned,
                                                IntegerEncodingOption option) {
#if MLT_WITH_FASTPFOR
    const auto encode = (physicalTechnique == PhysicalLevelTechnique::FAST_PFOR) ? &IntegerEncoder::encodeFastPfor
                                                                                 : &IntegerEncoder::encodeVarints32;
#else
    const auto encode = &IntegerEncoder::encodeVarints32;
#endif

    auto plainEncoded = (this->*encode)(values, isSigned);

    std::vector<std::int32_t> deltaValues(values.size());
    std::int32_t previousValue = 0;
    std::int32_t previousDelta = 0;
    std::uint32_t runs = 1;
    std::uint32_t deltaRuns = 1;
    for (std::size_t i = 0; i < values.size(); ++i) {
        const auto value = values[i];
        const auto delta = value - previousValue;
        deltaValues[i] = delta;

        if (i != 0 && value != previousValue) {
            ++runs;
        }
        if (i != 0 && delta != previousDelta) {
            ++deltaRuns;
        }
        previousValue = value;
        previousDelta = delta;
    }

    auto deltaEncoded = (this->*encode)(deltaValues, /*zigZag=*/true);

    struct Candidate {
        LogicalLevelTechnique t1;
        LogicalLevelTechnique t2;
        std::vector<std::uint8_t>* data;
        std::uint32_t numRuns;
        std::uint32_t physicalLength;
    };

    Candidate best{.t1 = LogicalLevelTechnique::NONE,
                   .t2 = LogicalLevelTechnique::NONE,
                   .data = &plainEncoded,
                   .numRuns = 0,
                   .physicalLength = static_cast<std::uint32_t>(values.size())};

    if (deltaEncoded.size() < best.data->size()) {
        best = {.t1 = LogicalLevelTechnique::DELTA,
                .t2 = LogicalLevelTechnique::NONE,
                .data = &deltaEncoded,
                .numRuns = 0,
                .physicalLength = static_cast<std::uint32_t>(values.size())};
    }

    std::vector<std::uint8_t> rleEncoded;
    std::uint32_t rlePhysicalLength = 0;
    bool isConstStream = false;
    const bool shouldTryRle = option != IntegerEncodingOption::AUTO || values.size() / runs >= 2;
    if (shouldTryRle) {
        auto rle = util::encoding::rle::encodeIntRle<std::int32_t>(values);
        isConstStream = (rle.runs.size() == 1);

        std::vector<std::int32_t> flattened;
        flattened.reserve(rle.runs.size() + rle.values.size());
        flattened.insert(flattened.end(), rle.runs.begin(), rle.runs.end());
        if (isSigned) {
            for (auto v : rle.values) {
                flattened.push_back(static_cast<std::int32_t>(util::encoding::encodeZigZag(v)));
            }
        } else {
            flattened.insert(flattened.end(), rle.values.begin(), rle.values.end());
        }
        rleEncoded = (this->*encode)(flattened, /*zigZag=*/false);

        rlePhysicalLength = static_cast<std::uint32_t>(rle.runs.size() + rle.values.size());
        if (isConstStream || rleEncoded.size() < best.data->size()) {
            best = {.t1 = LogicalLevelTechnique::RLE,
                    .t2 = LogicalLevelTechnique::NONE,
                    .data = &rleEncoded,
                    .numRuns = runs,
                    .physicalLength = rlePhysicalLength};
        }
    }

    std::vector<std::uint8_t> deltaRleEncoded;
    std::uint32_t deltaRlePhysicalLength = 0;
    const bool shouldTryDeltaRle = option != IntegerEncodingOption::AUTO ||
                                   (deltaValues.size() / deltaRuns >= 2 && !isConstStream);
    if (shouldTryDeltaRle) {
        auto deltaRle = util::encoding::rle::encodeIntRle<std::int32_t>(deltaValues);

        std::vector<std::int32_t> flattened;
        flattened.reserve(deltaRle.runs.size() + deltaRle.values.size());
        flattened.insert(flattened.end(), deltaRle.runs.begin(), deltaRle.runs.end());
        for (auto v : deltaRle.values) {
            flattened.push_back(static_cast<std::int32_t>(util::encoding::encodeZigZag(v)));
        }
        deltaRleEncoded = (this->*encode)(flattened, /*zigZag=*/false);

        deltaRlePhysicalLength = static_cast<std::uint32_t>(deltaRle.runs.size() + deltaRle.values.size());
        if (deltaRleEncoded.size() < best.data->size()) {
            best = {.t1 = LogicalLevelTechnique::DELTA,
                    .t2 = LogicalLevelTechnique::RLE,
                    .data = &deltaRleEncoded,
                    .numRuns = deltaRuns,
                    .physicalLength = deltaRlePhysicalLength};
        }
    }

    switch (option) {
        case IntegerEncodingOption::AUTO:
            break;
        case IntegerEncodingOption::PLAIN:
            best = {.t1 = LogicalLevelTechnique::NONE,
                    .t2 = LogicalLevelTechnique::NONE,
                    .data = &plainEncoded,
                    .numRuns = 0,
                    .physicalLength = static_cast<std::uint32_t>(values.size())};
            break;
        case IntegerEncodingOption::DELTA:
            best = {.t1 = LogicalLevelTechnique::DELTA,
                    .t2 = LogicalLevelTechnique::NONE,
                    .data = &deltaEncoded,
                    .numRuns = 0,
                    .physicalLength = static_cast<std::uint32_t>(values.size())};
            break;
        case IntegerEncodingOption::RLE:
            best = {.t1 = LogicalLevelTechnique::RLE,
                    .t2 = LogicalLevelTechnique::NONE,
                    .data = &rleEncoded,
                    .numRuns = runs,
                    .physicalLength = rlePhysicalLength};
            break;
        case IntegerEncodingOption::DELTA_RLE:
            best = {.t1 = LogicalLevelTechnique::DELTA,
                    .t2 = LogicalLevelTechnique::RLE,
                    .data = &deltaRleEncoded,
                    .numRuns = deltaRuns,
                    .physicalLength = deltaRlePhysicalLength};
            break;
    }

    return {.logicalLevelTechnique1 = best.t1,
            .logicalLevelTechnique2 = best.t2,
            .encodedValues = std::move(*best.data),
            .numRuns = best.numRuns,
            .physicalLevelEncodedValuesLength = best.physicalLength};
}

IntegerEncodingResult IntegerEncoder::encodeLong(std::span<const std::int64_t> values, bool isSigned) {
    return encodeLong(values, isSigned, impl->encodingOption);
}

IntegerEncodingResult IntegerEncoder::encodeLong(std::span<const std::int64_t> values,
                                                 bool isSigned,
                                                 IntegerEncodingOption option) {
    auto plainEncoded = encodeVarints64(values, isSigned);

    std::vector<std::int64_t> deltaValues(values.size());
    std::int64_t previousValue = 0;
    std::int64_t previousDelta = 0;
    std::uint32_t runs = 1;
    std::uint32_t deltaRuns = 1;
    for (std::size_t i = 0; i < values.size(); ++i) {
        const auto value = values[i];
        const auto delta = value - previousValue;
        deltaValues[i] = delta;

        if (i != 0 && value != previousValue) {
            ++runs;
        }
        if (i != 0 && delta != previousDelta) {
            ++deltaRuns;
        }
        previousValue = value;
        previousDelta = delta;
    }

    auto deltaEncoded = encodeVarints64(deltaValues, /*zigZag=*/true);

    struct Candidate {
        LogicalLevelTechnique t1;
        LogicalLevelTechnique t2;
        std::vector<std::uint8_t>* data;
        std::uint32_t numRuns;
        std::uint32_t physicalLength;
    };

    Candidate best{.t1 = LogicalLevelTechnique::NONE,
                   .t2 = LogicalLevelTechnique::NONE,
                   .data = &plainEncoded,
                   .numRuns = 0,
                   .physicalLength = static_cast<std::uint32_t>(values.size())};

    if (deltaEncoded.size() < best.data->size()) {
        best = {.t1 = LogicalLevelTechnique::DELTA,
                .t2 = LogicalLevelTechnique::NONE,
                .data = &deltaEncoded,
                .numRuns = 0,
                .physicalLength = static_cast<std::uint32_t>(values.size())};
    }

    std::vector<std::uint8_t> rleEncoded;
    std::uint32_t rlePhysicalLength = 0;
    const bool shouldTryRle = option != IntegerEncodingOption::AUTO || values.size() / runs >= 2;
    if (shouldTryRle) {
        auto rle = util::encoding::rle::encodeIntRle<std::int64_t>(values);

        std::vector<std::int64_t> flattened;
        flattened.reserve(rle.runs.size() + rle.values.size());
        for (auto r : rle.runs) {
            flattened.push_back(r);
        }
        if (isSigned) {
            for (auto v : rle.values) {
                flattened.push_back(static_cast<std::int64_t>(util::encoding::encodeZigZag(v)));
            }
        } else {
            flattened.insert(flattened.end(), rle.values.begin(), rle.values.end());
        }
        rleEncoded = encodeVarints64(flattened, /*zigZag=*/false);

        rlePhysicalLength = static_cast<std::uint32_t>(rle.runs.size() + rle.values.size());
        if (rleEncoded.size() < best.data->size()) {
            best = {.t1 = LogicalLevelTechnique::RLE,
                    .t2 = LogicalLevelTechnique::NONE,
                    .data = &rleEncoded,
                    .numRuns = runs,
                    .physicalLength = rlePhysicalLength};
        }
    }

    std::vector<std::uint8_t> deltaRleEncoded;
    std::uint32_t deltaRlePhysicalLength = 0;
    const bool shouldTryDeltaRle = option != IntegerEncodingOption::AUTO || deltaValues.size() / deltaRuns >= 2;
    if (shouldTryDeltaRle) {
        const auto deltaRle = util::encoding::rle::encodeIntRle<std::int64_t>(deltaValues);

        std::vector<std::int64_t> flattened;
        flattened.reserve(deltaRle.runs.size() + deltaRle.values.size());
        for (auto r : deltaRle.runs) {
            flattened.push_back(r);
        }
        for (auto v : deltaRle.values) {
            flattened.push_back(static_cast<std::int64_t>(util::encoding::encodeZigZag(v)));
        }
        deltaRleEncoded = encodeVarints64(flattened, /*zigZag=*/false);

        deltaRlePhysicalLength = static_cast<std::uint32_t>(deltaRle.runs.size() + deltaRle.values.size());
        if (deltaRleEncoded.size() < best.data->size()) {
            best = {.t1 = LogicalLevelTechnique::DELTA,
                    .t2 = LogicalLevelTechnique::RLE,
                    .data = &deltaRleEncoded,
                    .numRuns = deltaRuns,
                    .physicalLength = deltaRlePhysicalLength};
        }
    }

    switch (option) {
        case IntegerEncodingOption::AUTO:
            break;
        case IntegerEncodingOption::PLAIN:
            best = {.t1 = LogicalLevelTechnique::NONE,
                    .t2 = LogicalLevelTechnique::NONE,
                    .data = &plainEncoded,
                    .numRuns = 0,
                    .physicalLength = static_cast<std::uint32_t>(values.size())};
            break;
        case IntegerEncodingOption::DELTA:
            best = {.t1 = LogicalLevelTechnique::DELTA,
                    .t2 = LogicalLevelTechnique::NONE,
                    .data = &deltaEncoded,
                    .numRuns = 0,
                    .physicalLength = static_cast<std::uint32_t>(values.size())};
            break;
        case IntegerEncodingOption::RLE:
            best = {.t1 = LogicalLevelTechnique::RLE,
                    .t2 = LogicalLevelTechnique::NONE,
                    .data = &rleEncoded,
                    .numRuns = runs,
                    .physicalLength = rlePhysicalLength};
            break;
        case IntegerEncodingOption::DELTA_RLE:
            best = {.t1 = LogicalLevelTechnique::DELTA,
                    .t2 = LogicalLevelTechnique::RLE,
                    .data = &deltaRleEncoded,
                    .numRuns = deltaRuns,
                    .physicalLength = deltaRlePhysicalLength};
            break;
    }

    return {.logicalLevelTechnique1 = best.t1,
            .logicalLevelTechnique2 = best.t2,
            .encodedValues = std::move(*best.data),
            .numRuns = best.numRuns,
            .physicalLevelEncodedValuesLength = best.physicalLength};
}

std::vector<std::uint8_t> IntegerEncoder::buildStream(const IntegerEncodingResult& encoded,
                                                      std::uint32_t totalValues,
                                                      PhysicalLevelTechnique physicalTechnique,
                                                      PhysicalStreamType streamType,
                                                      std::optional<LogicalStreamType> logicalType) {
    const bool isRle = (encoded.logicalLevelTechnique1 == LogicalLevelTechnique::RLE ||
                        encoded.logicalLevelTechnique2 == LogicalLevelTechnique::RLE);
    std::vector<std::uint8_t> metadata;
    if (isRle) {
        metadata = RleEncodedStreamMetadata(streamType,
                                            std::move(logicalType),
                                            encoded.logicalLevelTechnique1,
                                            encoded.logicalLevelTechnique2,
                                            physicalTechnique,
                                            encoded.physicalLevelEncodedValuesLength,
                                            static_cast<std::uint32_t>(encoded.encodedValues.size()),
                                            encoded.numRuns,
                                            totalValues)
                       .encode();
    } else {
        metadata = StreamMetadata(streamType,
                                  std::move(logicalType),
                                  encoded.logicalLevelTechnique1,
                                  encoded.logicalLevelTechnique2,
                                  physicalTechnique,
                                  encoded.physicalLevelEncodedValuesLength,
                                  static_cast<std::uint32_t>(encoded.encodedValues.size()))
                       .encode();
    }

    std::vector<std::uint8_t> result;
    result.reserve(metadata.size() + encoded.encodedValues.size());
    result.insert(result.end(), metadata.begin(), metadata.end());
    result.insert(result.end(), encoded.encodedValues.begin(), encoded.encodedValues.end());
    return result;
}

std::vector<std::uint8_t> IntegerEncoder::encodeIntStream(std::span<const std::int32_t> values,
                                                          PhysicalLevelTechnique physicalTechnique,
                                                          bool isSigned,
                                                          PhysicalStreamType streamType,
                                                          std::optional<LogicalStreamType> logicalType) {
    const auto encoded = encodeInt(values, physicalTechnique, isSigned, impl->encodingOption);
    return buildStream(
        encoded, static_cast<std::uint32_t>(values.size()), physicalTechnique, streamType, std::move(logicalType));
}

std::vector<std::uint8_t> IntegerEncoder::encodeIntStream(std::span<const std::int32_t> values,
                                                          PhysicalLevelTechnique physicalTechnique,
                                                          bool isSigned,
                                                          PhysicalStreamType streamType,
                                                          std::optional<LogicalStreamType> logicalType,
                                                          IntegerEncodingOption option) {
    const auto encoded = encodeInt(values, physicalTechnique, isSigned, option);
    return buildStream(
        encoded, static_cast<std::uint32_t>(values.size()), physicalTechnique, streamType, std::move(logicalType));
}

std::vector<std::uint8_t> IntegerEncoder::encodeLongStream(std::span<const std::int64_t> values,
                                                           bool isSigned,
                                                           PhysicalStreamType streamType,
                                                           std::optional<LogicalStreamType> logicalType) {
    const auto encoded = encodeLong(values, isSigned, impl->encodingOption);
    return buildStream(encoded,
                       static_cast<std::uint32_t>(values.size()),
                       PhysicalLevelTechnique::VARINT,
                       streamType,
                       std::move(logicalType));
}

std::vector<std::uint8_t> IntegerEncoder::encodeLongStream(std::span<const std::int64_t> values,
                                                           bool isSigned,
                                                           PhysicalStreamType streamType,
                                                           std::optional<LogicalStreamType> logicalType,
                                                           IntegerEncodingOption option) {
    const auto encoded = encodeLong(values, isSigned, option);
    return buildStream(encoded,
                       static_cast<std::uint32_t>(values.size()),
                       PhysicalLevelTechnique::VARINT,
                       streamType,
                       std::move(logicalType));
}

} // namespace mlt::encoder
