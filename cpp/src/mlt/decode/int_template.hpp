#pragma once

#include <mlt/common.hpp>
#include <mlt/decode/int.hpp>
#include <mlt/util/morton_curve.hpp>

#include <cassert>
#include <span>
#include <stdexcept>
#include <string>
#include <utility>

namespace mlt::decoder {

#pragma region Public

inline std::size_t IntegerDecoder::getIntArrayBufferSize(const std::size_t count,
                                                         const StreamMetadata& streamMetadata) {
    using namespace metadata::stream;
    switch (streamMetadata.getLogicalLevelTechnique1()) {
        case LogicalLevelTechnique::DELTA:
            if (streamMetadata.getLogicalLevelTechnique2() == LogicalLevelTechnique::RLE &&
                streamMetadata.getMetadataType() == LogicalLevelTechnique::RLE) {
                const auto& rleMetadata = static_cast<const RleEncodedStreamMetadata&>(streamMetadata);
                return rleMetadata.getNumRleValues();
            }
            return streamMetadata.getNumValues();
        case LogicalLevelTechnique::NONE:
        case LogicalLevelTechnique::COMPONENTWISE_DELTA:
            return count;
        case LogicalLevelTechnique::RLE: {
            if (streamMetadata.getMetadataType() != LogicalLevelTechnique::RLE) {
                return 0;
            }
            const auto& rleMetadata = static_cast<const RleEncodedStreamMetadata&>(streamMetadata);
            return rleMetadata.getNumRleValues();
        }
        case LogicalLevelTechnique::MORTON: {
            return 2 * count;
        }
        default:
            return 0;
    }
}

inline void IntegerDecoder::checkStream(const StreamMetadata& metadata, const BufferStream& tileData) {
    using namespace metadata::stream;
    if (metadata.getByteLength() > tileData.getRemaining()) {
        throw std::runtime_error("stream exceeds the available data");
    }

    // The least data that can describe a value, so that the count can be limited before allocating for it
    const auto numValues = static_cast<std::uint64_t>(metadata.getNumValues());
    switch (metadata.getPhysicalLevelTechnique()) {
        case PhysicalLevelTechnique::FAST_PFOR:
            // Each block of 256 values takes at least two bytes
            if (numValues > 128 * static_cast<std::uint64_t>(metadata.getByteLength())) {
                throw std::runtime_error("stream has more values than its data can hold");
            }
            break;
        case PhysicalLevelTechnique::VARINT:
            if (numValues > tileData.getRemaining()) {
                throw std::runtime_error("stream has more values than its data can hold");
            }
            break;
        default:
            break;
    }
}

template <typename T, typename TTarget>
    requires((std::is_integral_v<T> || std::is_enum_v<T>) && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>) &&
             sizeof(T) <= sizeof(TTarget))
void IntegerDecoder::decodeIntArray(const T* values,
                                    const std::size_t count,
                                    TTarget* out,
                                    const std::size_t outCount,
                                    const StreamMetadata& streamMetadata,
                                    const bool isSigned) {
    using namespace metadata::stream;
    using namespace util::decoding;
    switch (streamMetadata.getLogicalLevelTechnique1()) {
        case LogicalLevelTechnique::NONE: {
            if (count > outCount) {
                throw std::runtime_error("output buffer too small");
            }
            const auto f = isSigned ? [](T x) noexcept { return static_cast<TTarget>(decodeZigZag(x)); } : [](T x) noexcept { return static_cast<TTarget>(x); };
            std::transform(values, values + count, out, f);
            return;
        }
        case LogicalLevelTechnique::DELTA:
            if (streamMetadata.getLogicalLevelTechnique2() == LogicalLevelTechnique::RLE) {
                if (streamMetadata.getMetadataType() != LogicalLevelTechnique::RLE) {
                    throw std::runtime_error("invalid RLE metadata");
                }
                const auto& rleMetadata = static_cast<const RleEncodedStreamMetadata&>(streamMetadata);
                if constexpr (sizeof(T) == sizeof(TTarget)) {
                    rle::decodeInt<T, TTarget>(values, count, out, outCount, rleMetadata.getRuns());
                    decodeZigZagDelta(out, outCount, out, outCount);
                } else {
                    // Expand the runs at the column width, not the wider target width.
                    // The delta has to wrap at the column width before it is widened.
                    auto rleBuffer = getTempBuffer<T>(outCount);
                    rle::decodeInt<T, T>(values, count, rleBuffer.get(), outCount, rleMetadata.getRuns());
                    decodeZigZagDelta<T, TTarget>(rleBuffer.get(), outCount, out, outCount);
                }
            } else {
                if (count > outCount) {
                    throw std::runtime_error("output buffer too small");
                }
                decodeZigZagDelta(values, count, out, count);
            }
            break;
        case LogicalLevelTechnique::COMPONENTWISE_DELTA:
            if constexpr (std::is_same_v<TTarget, std::int32_t> || std::is_same_v<TTarget, std::uint32_t>) {
                if (count > outCount) {
                    throw std::runtime_error("output buffer too small");
                }
                if (count % 2 != 0) {
                    throw std::runtime_error("componentwise delta needs pairs of values");
                }
                std::transform(values, values + count, out, [](auto x) { return static_cast<TTarget>(x); });
                vectorized::decodeComponentwiseDeltaVec2(out, count);
                break;
            }
            throw std::runtime_error("Logical level technique COMPONENTWISE_DELTA not implemented for 64-bit values");
        case LogicalLevelTechnique::RLE: {
            if (streamMetadata.getMetadataType() != LogicalLevelTechnique::RLE) {
                throw std::runtime_error("invalid RLE metadata");
            }
            const auto& rleMetadata = static_cast<const RleEncodedStreamMetadata&>(streamMetadata);
            auto decoder = isSigned ? [](T x) {
                    return static_cast<TTarget>(decodeZigZag(x));
            } : [](T x) {
                    return static_cast<TTarget>(x);
            };
            rle::decodeInt<T, TTarget>(values, count, out, outCount, rleMetadata.getRuns(), std::move(decoder));
            break;
        }
        case LogicalLevelTechnique::MORTON: {
            // TODO: zig-zag decode when morton second logical level technique
            if (streamMetadata.getMetadataType() != LogicalLevelTechnique::MORTON) {
                throw std::runtime_error("invalid Morton metadata");
            }
            const auto& mortonMetadata = static_cast<const MortonEncodedStreamMetadata&>(streamMetadata);
            if (count > outCount / 2) {
                throw std::runtime_error("output buffer too small");
            }
            if constexpr (std::is_same_v<T, std::uint32_t> && std::is_same_v<TTarget, std::uint32_t>) {
                decodeMortonCodes<T, TTarget, true>(
                    values, count, out, outCount, mortonMetadata.getNumBits(), mortonMetadata.getCoordinateShift());
            } else {
                throw std::runtime_error("Logical level technique MORTON not implemented for 64-bit values");
            }
            break;
        }
        default:
            throw std::runtime_error("The specified logical level technique is not supported for integers: " +
                                     std::to_string(std::to_underlying(streamMetadata.getLogicalLevelTechnique1())));
            break;
    }
}

template <typename TDecode, typename TInt, typename TTarget>
void IntegerDecoder::decodeIntStream(BufferStream& tileData,
                                     std::vector<TTarget>& out,
                                     const StreamMetadata& metadata,
                                     const bool isSigned) {
    checkStream(metadata, tileData);
    auto tempBuffer = getTempBuffer<TInt>(metadata.getNumValues());
    decodeIntStream<TDecode, TInt, TTarget>(tileData, tempBuffer, metadata.getNumValues(), out, metadata, isSigned);
}

template <typename TDecode, typename TInt, typename TTarget>
void IntegerDecoder::decodeIntStream(BufferStream& tileData,
                                     TInt* buffer,
                                     std::size_t bufferSize,
                                     std::vector<TTarget>& out,
                                     const StreamMetadata& metadata,
                                     const bool isSigned) {
    decodeStream<TDecode, TInt>(tileData, buffer, bufferSize, metadata);
    const auto outSize = getIntArrayBufferSize(bufferSize, metadata);
    budget.consumeItems<TTarget>(outSize);
    out.resize(outSize);
    decodeIntArray<TInt, TTarget>(buffer, bufferSize, out.data(), out.size(), metadata, isSigned);
}

template <typename TDecode, typename TInt, typename TTarget>
TTarget IntegerDecoder::decodeConstIntStream(BufferStream& tileData,
                                             const StreamMetadata& metadata,
                                             const bool isSigned) {
    TInt buffer[2] = {0};
    decodeStream<TDecode, TInt>(tileData, &buffer[0], countof(buffer), metadata);

    if (metadata.getNumValues() < 1 || metadata.getNumValues() > 2) {
        throw std::runtime_error("unexpected number of values in constant stream");
    }

    if (metadata.getNumValues() == 1) {
        return isSigned ? decodeZigZagValue(buffer[0]) : buffer[0];
    }
    return isSigned ? decodeZigZagValue(buffer[1]) : buffer[1];
}

template <typename TDecode, typename TInt, typename TTarget, bool Delta>
void IntegerDecoder::decodeMortonStream(BufferStream& tileData,
                                        std::vector<TTarget>& out,
                                        const MortonEncodedStreamMetadata& metadata) {
    checkStream(metadata, tileData);
    auto tempBuffer = getTempBuffer<TInt>(metadata.getNumValues());
    budget.consumeItems<TTarget>(2 * static_cast<std::size_t>(metadata.getNumValues()));
    out.resize(2 * static_cast<std::size_t>(metadata.getNumValues()));
    decodeMortonStream<TDecode, TInt, TTarget, Delta>(
        tileData, tempBuffer, metadata.getNumValues(), out.data(), out.size(), metadata);
}

template <typename TDecode, typename TInt, typename TTarget, bool Delta>
void IntegerDecoder::decodeMortonStream(BufferStream& tileData,
                                        TInt* buffer,
                                        std::size_t bufferCount,
                                        TTarget* out,
                                        std::size_t outCount,
                                        const MortonEncodedStreamMetadata& metadata) {
    decodeStream<TDecode, TInt>(tileData, buffer, bufferCount, metadata);
    if (bufferCount > outCount / 2) {
        throw std::runtime_error("output buffer too small");
    }
    decodeMortonCodes<TInt, TTarget, Delta>(
        buffer, bufferCount, out, outCount, metadata.getNumBits(), metadata.getCoordinateShift());
}

#pragma endregion

#pragma region Private

template <typename TDecode, typename TTarget>
    requires(std::is_integral_v<TDecode> && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>))
void IntegerDecoder::decodeStream(BufferStream& tileData,
                                  TTarget* out,
                                  std::size_t outSize,
                                  const StreamMetadata& metadata) {
    using namespace metadata::stream;

    // The value count comes from the tile, so it's not enough to assert that the caller's buffer can hold it
    if (outSize < metadata.getNumValues()) {
        throw std::runtime_error("stream has more values than the output buffer can hold");
    }

    switch (metadata.getPhysicalLevelTechnique()) {
        case PhysicalLevelTechnique::FAST_PFOR: {
            std::uint32_t* outPtr = nullptr;
            std::optional<BufWrapper<std::uint32_t>> tempBuffer; // NOLINT(misc-const-correctness)
            if constexpr (sizeof(*out) == sizeof(std::uint32_t)) {
                // Decode directly into the output buffer
                outPtr = reinterpret_cast<std::uint32_t*>(out);
            } else {
                // Decode into a 32-bit temprary buffer ...
                tempBuffer.emplace(getTempBuffer<std::uint32_t>(metadata.getNumValues()));
                outPtr = tempBuffer->get();
            }

            decodeFastPfor(tileData, outPtr, metadata.getNumValues(), metadata.getByteLength());

            if constexpr (sizeof(*out) != sizeof(std::uint32_t)) {
                // ... then extend to 64-bit output (`.mapToLong(i -> i)` in Java)
                std::transform(
                    outPtr, outPtr + metadata.getNumValues(), out, [](auto x) { return static_cast<TTarget>(x); });
            }

            break;
        }
        case PhysicalLevelTechnique::NONE: {
            // Fixed-width little-endian values, no physical-level compression.
            const auto numValues = metadata.getNumValues();
            if (metadata.getByteLength() != numValues * sizeof(TDecode)) {
                throw std::runtime_error("unexpected byte length for unencoded integer stream");
            }
            // Check that the data is available before reading it
            const auto* const src = tileData.getReadPosition();
            tileData.consume(metadata.getByteLength());
            for (std::uint32_t i = 0; i < numValues; ++i) {
                TDecode value = 0;
                std::memcpy(&value, src + (i * sizeof(TDecode)), sizeof(TDecode));
                out[i] = static_cast<TTarget>(value);
            }
            break;
        }
        case PhysicalLevelTechnique::VARINT:
            util::decoding::decodeVarints<TDecode>(tileData, metadata.getNumValues(), out);
            break;
        default:
            throw std::runtime_error("Specified physical level technique not yet supported " +
                                     std::to_string(std::to_underlying(metadata.getPhysicalLevelTechnique())));
    }
}

template <typename T, typename TTarget>
    requires(std::is_integral_v<underlying_type_t<T>> && std::is_integral_v<underlying_type_t<TTarget>> &&
             sizeof(T) <= sizeof(TTarget))
void IntegerDecoder::decodeZigZagDelta(const T* values,
                                       const std::size_t count,
                                       TTarget* const out,
                                       [[maybe_unused]] const std::size_t outCount) noexcept {
    using namespace util::decoding;
    assert(count == outCount);

    // Sum at the column width so the total wraps like it did when encoded.
    // Unsigned makes the wrap well-defined.
    using Unsigned = std::make_unsigned_t<underlying_type_t<T>>;
    // Native carries the column's signedness back when widening to the target.
    // Unsigned ids zero-extend; signed values sign-extend.
    using Native = underlying_type_t<T>;

    std::uint32_t pos = 0;
    Unsigned runningTotal = 0;
    for (const auto zigZagDelta : std::span{values, count}) {
        runningTotal += static_cast<Unsigned>(decodeZigZag(zigZagDelta));
        const auto value = static_cast<Native>(runningTotal);
        out[pos++] = static_cast<TTarget>(value);
    }
}

template <typename TDecode, typename TTarget, bool delta>
    requires(std::is_integral_v<TDecode> && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>))
void IntegerDecoder::decodeMortonCodes(const TDecode* const data,
                                       const std::size_t count,
                                       TTarget* const out,
                                       [[maybe_unused]] std::size_t outCount,
                                       int numBits,
                                       int coordinateShift) noexcept {
    using namespace util::decoding;
    assert(2 * count <= outCount);
    std::uint32_t previousMortonCode = 0;
    for (std::size_t i = 0, j = 0; i < count; ++i) {
        auto mortonCode = static_cast<std::uint32_t>(data[i]);
        if constexpr (delta) {
            mortonCode += previousMortonCode;
        };
        out[j++] = static_cast<TTarget>(util::MortonCurve::decode(mortonCode, numBits) - coordinateShift);
        out[j++] = static_cast<TTarget>(util::MortonCurve::decode(mortonCode >> 1, numBits) - coordinateShift);
        if constexpr (delta) {
            previousMortonCode = mortonCode;
        }
    }
}

#pragma endregion Private

} // namespace mlt::decoder
