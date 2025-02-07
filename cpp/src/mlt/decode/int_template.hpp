#pragma once

#include <mlt/decode/int.hpp>

#include <cassert>
#include <stdexcept>
#include <string>
#include <utility>

namespace mlt::decoder {

#pragma region Public

template <typename T, typename TTarget, bool isSigned>
    requires((std::is_integral_v<T> || std::is_enum_v<T>) && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>) &&
             sizeof(T) <= sizeof(TTarget))
void IntegerDecoder::decodeIntArray(const std::vector<T>& values,
                                    std::vector<TTarget>& out,
                                    const StreamMetadata& streamMetadata) {
    using namespace metadata::stream;
    using namespace util::decoding;
    switch (streamMetadata.getLogicalLevelTechnique1()) {
        case LogicalLevelTechnique::NONE: {
            out.resize(values.size());
            const auto f = isSigned ? [](T x) noexcept { return static_cast<TTarget>(decodeZigZag(x)); } : [](T x) noexcept { return static_cast<TTarget>(x); };
            std::ranges::transform(values, out.begin(), f);
            return;
        }
        case LogicalLevelTechnique::DELTA:
            if (streamMetadata.getLogicalLevelTechnique2() == LogicalLevelTechnique::RLE) {
                if (streamMetadata.getMetadataType() != metadata::stream::LogicalLevelTechnique::RLE) {
                    throw std::runtime_error("invalid RLE metadata");
                }
                const auto& rleMetadata = static_cast<const RleEncodedStreamMetadata&>(streamMetadata);
                out.resize(rleMetadata.getNumRleValues());
                rle::decodeInt<T, TTarget>(values, out, rleMetadata.getRuns());
                decodeZigZagDelta(out, out);
            } else {
                out.resize(values.size());
                decodeZigZagDelta(values, out);
            }
            break;
        case LogicalLevelTechnique::COMPONENTWISE_DELTA:
            if constexpr (std::is_same_v<TTarget, std::int32_t> || std::is_same_v<TTarget, std::uint32_t>) {
                out.resize(values.size());
                std::ranges::transform(values, out.begin(), [](auto x) { return static_cast<TTarget>(x); });
                vectorized::decodeComponentwiseDeltaVec2(out);
                break;
            }
            throw std::runtime_error("Logical level technique COMPONENTWISE_DELTA not implemented for 64-bit values");
        case LogicalLevelTechnique::RLE: {
            if (streamMetadata.getMetadataType() != metadata::stream::LogicalLevelTechnique::RLE) {
                throw std::runtime_error("invalid RLE metadata");
            }
            const auto& rleMetadata = static_cast<const RleEncodedStreamMetadata&>(streamMetadata);
            out.resize(rleMetadata.getNumRleValues());
            rle::decodeInt<T, TTarget>(values, out, rleMetadata.getRuns(), [](T x) {
                if constexpr (isSigned)
                    return static_cast<TTarget>(decodeZigZag(x));
                else
                    return static_cast<TTarget>(x);
            });
            break;
        }
        case LogicalLevelTechnique::MORTON: {
            // TODO: zig-zag decode when morton second logical level technique
            if (streamMetadata.getMetadataType() != metadata::stream::LogicalLevelTechnique::MORTON) {
                throw std::runtime_error("invalid RLE metadata");
            }
            const auto& mortonMetadata = static_cast<const MortonEncodedStreamMetadata&>(streamMetadata);
            out.resize(2 * values.size());
            if constexpr (std::is_same_v<T, std::uint32_t> && std::is_same_v<TTarget, std::uint32_t>) {
                decodeMortonCodes<T, TTarget, true>(
                    values, out, mortonMetadata.getNumBits(), mortonMetadata.getCoordinateShift());
            } else {
                throw std::runtime_error("Logical level technique MORTON not implemented for 64-bit values");
            }
            break;
        }
        case LogicalLevelTechnique::PSEUDODECIMAL:
        default:
            throw std::runtime_error("The specified logical level technique is not supported for integers: " +
                                     std::to_string(std::to_underlying(streamMetadata.getLogicalLevelTechnique1())));
            break;
    }
}

template <typename TDecode, typename TInt, typename TTarget, bool isSigned>
void IntegerDecoder::decodeIntStream(BufferStream& tileData,
                                     std::vector<TTarget>& out,
                                     const StreamMetadata& metadata) {
    decodeIntStream<TDecode, TInt, TTarget, isSigned>(tileData, getTempBuffer<TInt>(), out, metadata);
}

template <typename TDecode, typename TInt, typename TTarget, bool isSigned>
void IntegerDecoder::decodeIntStream(BufferStream& tileData,
                                     std::vector<TInt>& buffer,
                                     std::vector<TTarget>& out,
                                     const StreamMetadata& metadata) {
    decodeStream<TDecode, TInt, isSigned>(tileData, buffer, metadata);
    decodeIntArray<TInt, TTarget, isSigned>(buffer, out, metadata);
    buffer.clear();
}

template <typename TDecode, typename TInt, typename TTarget, bool Delta>
void IntegerDecoder::decodeMortonStream(BufferStream& tileData,
                                        std::vector<TTarget>& out,
                                        const MortonEncodedStreamMetadata& metadata) {
    decodeMortonStream<TDecode, TInt, TTarget, Delta>(tileData, getTempBuffer<TInt>(), out, metadata);
}

template <typename TDecode, typename TInt, typename TTarget, bool Delta>
void IntegerDecoder::decodeMortonStream(BufferStream& tileData,
                                        std::vector<TInt>& buffer,
                                        std::vector<TTarget>& out,
                                        const MortonEncodedStreamMetadata& metadata) {
    decodeStream<TDecode, TInt>(tileData, buffer, metadata);
    out.resize(2 * buffer.size());
    decodeMortonCodes<TInt, TTarget, Delta>(buffer, out, metadata.getNumBits(), metadata.getCoordinateShift());
    buffer.clear();
}

#pragma endregion

#pragma region Private

template <typename TDecode, typename TTarget, bool isSigned>
    requires(std::is_integral_v<TDecode> && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>))
void IntegerDecoder::decodeStream(BufferStream& tileData, std::vector<TTarget>& out, const StreamMetadata& metadata) {
    using namespace metadata::stream;

    out.resize(metadata.getNumValues());
    switch (metadata.getPhysicalLevelTechnique()) {
        case PhysicalLevelTechnique::FAST_PFOR: {
            auto* outPtr = reinterpret_cast<std::uint32_t*>(out.data());
            const auto resultLength = decodeFastPfor(
                tileData, outPtr, metadata.getNumValues(), metadata.getByteLength());
            assert(resultLength == out.size());
            out.resize(resultLength);
        } break;
        case PhysicalLevelTechnique::VARINT:
            util::decoding::decodeVarints<TDecode>(tileData, out);
            break;
        default:
            throw std::runtime_error("Specified physical level technique not yet supported " +
                                     std::to_string(std::to_underlying(metadata.getPhysicalLevelTechnique())));
    }
}

template <typename T>
void IntegerDecoder::decodeRLE(const std::vector<T>& values, std::vector<T>& out, const count_t numRuns) {
    count_t outPos = 0;
    for (std::uint32_t i = 0; i < numRuns; ++i) {
        const auto run = values[i];
        const auto value = values[i + numRuns];
        if (outPos + run > out.size()) {
            throw std::runtime_error("RLE run exceeds output buffer size");
        }
        std::fill(std::next(out.begin(), outPos), std::next(out.begin(), outPos + run), value);
    }
}

template <typename T>
void IntegerDecoder::decodeDeltaRLE(const std::vector<T>& values, std::vector<T>& out, const count_t numRuns) {
    count_t outPos = 0;
    T previousValue = 0;
    for (std::uint32_t i = 0; i < numRuns; ++i) {
        const auto run = values[i];
        if (outPos + run > out.size()) {
            throw std::runtime_error("RLE run exceeds output buffer size");
        }

        const auto value = static_cast<std::make_signed_t<T>>(values[i + numRuns]);
        const auto delta = util::decoding::decodeZigZag(value);
        for (std::size_t j = 0; j < run; ++j) {
            out[outPos++] = static_cast<T>(previousValue += delta);
        }
    }
}

template <typename T, typename TTarget>
    requires(std::is_integral_v<underlying_type_t<T>> && std::is_integral_v<underlying_type_t<TTarget>> &&
             sizeof(T) <= sizeof(TTarget))
void IntegerDecoder::decodeZigZagDelta(const std::vector<T>& values, std::vector<TTarget>& out) noexcept {
    using namespace util::decoding;
    assert(out.size() == values.size());
    count_t pos = 0;
    using ST = std::make_signed_t<underlying_type_t<T>>;
    ST previousValue = 0;
    for (const auto zigZagDelta : values) {
        const auto delta = static_cast<ST>(decodeZigZag(zigZagDelta));
        out[pos++] = static_cast<TTarget>(previousValue += delta);
    }
}

template <typename TDecode, typename TTarget, bool delta>
    requires(std::is_integral_v<TDecode> && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>))
void IntegerDecoder::decodeMortonCodes(const std::vector<TDecode>& data,
                                       std::vector<TTarget>& out,
                                       int numBits,
                                       int coordinateShift) noexcept {
    using namespace util::decoding;
    assert(out.size() == 2 * data.size());
    std::uint32_t previousMortonCode = 0;
    for (std::size_t i = 0, j = 0; i < data.size(); ++i) {
        auto mortonCode = static_cast<std::uint32_t>(data[i]);
        if constexpr (delta) {
            mortonCode += previousMortonCode;
        }
        out[j++] = static_cast<TTarget>(decodeMorton(mortonCode, numBits) - coordinateShift);
        out[j++] = static_cast<TTarget>(decodeMorton(mortonCode >> 1, numBits) - coordinateShift);
        if constexpr (delta) {
            previousMortonCode = mortonCode;
        }
    }
}

#pragma endregion Private

} // namespace mlt::decoder
