#pragma once

#include <cstdint>
#include <metadata/stream.hpp>
#include <type_traits>
#include <util/buffer_stream.hpp>
#include <util/rle.hpp>
#include <util/vectorized.hpp>
#include <util/zigzag.hpp>

// from fastpfor
#include <compositecodec.h>
#include <fastpfor.h>
#include <variablebyte.h>

#include <cassert>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace mlt::decoder {

class IntegerDecoder {
public:
    using StreamMetadata = metadata::stream::StreamMetadata;
    using MortonEncodedStreamMetadata = metadata::stream::MortonEncodedStreamMetadata;

    IntegerDecoder() noexcept(false) = default;
    IntegerDecoder(const IntegerDecoder&) = delete;
    IntegerDecoder(IntegerDecoder&&) noexcept(false) = default;
    ~IntegerDecoder() = default;

    IntegerDecoder& operator=(const IntegerDecoder&) = delete;
    IntegerDecoder& operator=(IntegerDecoder&&) = delete;

    /// Decode a buffer of integers into another, according to the encoding scheme specified by the metadata
    /// @param values input values
    /// @param out output values, automatically resized as necessary
    /// @param metadata stream metadata specifying the encoding details
    template <typename T, typename TTarget = T, bool isSigned = std::is_signed_v<T>>
        requires((std::is_integral_v<T> || std::is_enum_v<T>) &&
                 (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>) && sizeof(T) <= sizeof(TTarget))
    void decodeIntArray(const std::vector<T>& values,
                        std::vector<TTarget>& out,
                        const StreamMetadata& streamMetadata) noexcept(false) {
        using namespace metadata::stream;
        using namespace util::decoding;
        switch (streamMetadata.getLogicalLevelTechnique1()) {
            case LogicalLevelTechnique::NONE: {
                out.resize(values.size());
                const auto f = isSigned ? [](T x){ return static_cast<TTarget>(decodeZigZag(x)); } : [](T x){ return static_cast<TTarget>(x); };
                std::ranges::transform(values, out.begin(), f);
                return;
            }
            case LogicalLevelTechnique::DELTA:
                if (streamMetadata.getLogicalLevelTechnique2() == LogicalLevelTechnique::RLE) {
                    //     var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
                    //     values =
                    //         DecodingUtils.decodeUnsignedRLE(
                    //             values, rleMetadata.runs(), rleMetadata.numRleValues());
                    //     return decodeZigZagDelta(values);
                    throw std::runtime_error("Logical level technique RLE-DELTA not implemented: ");
                }
                decodeZigZagDelta(values, out);
                break;
            case LogicalLevelTechnique::COMPONENTWISE_DELTA:
                if constexpr (std::is_same_v<TTarget, std::int32_t> || std::is_same_v<TTarget, std::uint32_t>) {
                    out.resize(values.size());
                    std::ranges::transform(values, out.begin(), [](auto x) { return static_cast<TTarget>(x); });
                    vectorized::decodeComponentwiseDeltaVec2(out);
                    break;
                }
                throw std::runtime_error(
                    "Logical level technique COMPONENTWISE_DELTA not implemented for 64-bit values");
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
                throw std::runtime_error(
                    "The specified logical level technique is not supported for integers: " +
                    std::to_string(std::to_underlying(streamMetadata.getLogicalLevelTechnique1())));
                break;
        }
    }

    /// Decode an integer stream into the target buffer
    /// @param tileData source data
    /// @param out output data, automatically resized
    /// @param metadata stream metadata specifying the encoding details
    /// @details Uses an internal buffer for intermediate values
    template <typename TDecode,
              typename TInt = TDecode,
              typename TTarget = TDecode,
              bool isSigned = std::is_signed_v<TDecode>>
    void decodeIntStream(BufferStream& tileData,
                         std::vector<TTarget>& out,
                         const StreamMetadata& metadata) noexcept(false) {
        decodeIntStream<TDecode, TInt, TTarget, isSigned>(tileData, getTempBuffer<TInt>(), out, metadata);
    }

    /// Decode an integer stream into the target buffer
    /// @param tileData source data
    /// @param buffer storage for intermediate values, automatically resized
    /// @param out output data, automatically resized
    /// @param metadata stream metadata specifying the encoding details
    template <typename TDecode,
              typename TInt = TDecode,
              typename TTarget = TInt,
              bool isSigned = std::is_signed_v<TDecode>>
    void decodeIntStream(BufferStream& tileData,
                         std::vector<TInt>& buffer,
                         std::vector<TTarget>& out,
                         const StreamMetadata& metadata) noexcept(false) {
        decodeStream<TDecode, TInt, isSigned>(tileData, buffer, metadata);
        decodeIntArray<TInt, TTarget, isSigned>(buffer, out, metadata);
        buffer.clear();
    }

    template <typename TDecode, typename TInt = TDecode, typename TTarget = TInt, bool Delta = true>
    void decodeMortonStream(BufferStream& tileData,
                            std::vector<TTarget>& out,
                            const MortonEncodedStreamMetadata& metadata) noexcept(false) {
        decodeMortonStream<TDecode, TInt, TTarget, Delta>(tileData, getTempBuffer<TInt>(), out, metadata);
    }

    template <typename TDecode, typename TInt = TDecode, typename TTarget = TInt, bool Delta = true>
    void decodeMortonStream(BufferStream& tileData,
                            std::vector<TInt>& buffer,
                            std::vector<TTarget>& out,
                            const MortonEncodedStreamMetadata& metadata) noexcept(false) {
        decodeStream<TDecode, TInt>(tileData, buffer, metadata);
        out.resize(2 * buffer.size());
        decodeMortonCodes<TInt, TTarget, Delta>(buffer, out, metadata.getNumBits(), metadata.getCoordinateShift());
        buffer.clear();
    }

private:
    FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte> codec;
    // Reused storage for intermediate values
    std::vector<std::uint32_t> buffer4;
    std::vector<std::uint64_t> buffer8;

    template <typename T>
    std::vector<T>& getTempBuffer() noexcept {
        // Can we make these covariant and eliminate the casts?
        if constexpr (std::is_trivially_assignable_v<decltype(buffer4)::reference, T>) {
            return reinterpret_cast<std::vector<T>&>(buffer4);
        } else if constexpr (std::is_trivially_assignable_v<decltype(buffer8)::reference, T>) {
            return static_cast<std::vector<T>&>(buffer8);
        }
        if constexpr (std::is_enum_v<T>) {
            if constexpr (std::is_trivially_assignable_v<decltype(buffer4)::reference, std::underlying_type_t<T>>) {
                return reinterpret_cast<std::vector<T>&>(buffer4);
            }
        }
        // Ideally this would be a `static_assert`, but that fails even if never reached by template expansion.
        assert(false);
        auto* fail = static_cast<std::vector<T>*>(nullptr);
        return *fail;
    }

    template <typename TDecode, typename TTarget = TDecode, bool isSigned = std::is_signed_v<TDecode>>
        requires(std::is_integral_v<TDecode> && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>))
    void decodeStream(BufferStream& tileData,
                      std::vector<TTarget>& out,
                      const StreamMetadata& metadata) noexcept(false) {
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
    static void decodeRLE(const std::vector<T>& values, std::vector<T>& out, const count_t numRuns) noexcept(false) {
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
    static void decodeDeltaRLE(const std::vector<T>& values,
                               std::vector<T>& out,
                               const count_t numRuns) noexcept(false) {
        count_t outPos = 0;
        T previousValue = 0;
        for (std::uint32_t i = 0; i < numRuns; ++i) {
            const auto run = values[i];
            if (outPos + run > out.size()) {
                throw std::runtime_error("RLE run exceeds output buffer size");
            }

            // TODO: check signs
            const auto value = static_cast<std::make_signed_t<T>>(values[i + numRuns]);
            const auto delta = util::decoding::decodeZigZag(value);
            for (std::size_t j = 0; j < run; ++j) {
                out[outPos++] = static_cast<T>(previousValue += delta);
            }
        }
    }

    template <typename T, typename TTarget>
        requires(std::is_integral_v<T> && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>) &&
                 sizeof(T) <= sizeof(TTarget))
    static void decodeZigZagDelta(const std::vector<T>& values, std::vector<TTarget>& out) noexcept(false) {
        out.resize(values.size());
        if (out.size() < values.size()) {
            throw std::runtime_error("insufficient output buffer");
        }
        count_t pos = 0;
        T previousValue = 0;
        for (const auto zigZagDelta : values) {
            // TODO: check signs
            const auto delta = util::decoding::decodeZigZag(zigZagDelta);
            out[pos++] = static_cast<TTarget>(previousValue += delta);
        }
    }

    /// Decode standard or delta Morton codes
    /// @param data input data
    /// @param out output data, must be 2x the input size
    template <typename TDecode, typename TTarget, bool delta>
        requires(std::is_integral_v<TDecode> && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>))
    static void decodeMortonCodes(const std::vector<TDecode>& data,
                                  std::vector<TTarget>& out,
                                  int numBits,
                                  int coordinateShift) noexcept {
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

    //   static std::pair<std::uint32_t, std::uint32_t> decodeMortonCode(std::uint32_t mortonCode, count_t numBits,
    //   count_t coordinateShift) {
    //     return std::make_pair(
    //     decodeMorton(mortonCode, numBits) - coordinateShift,
    //     decodeMorton(mortonCode >> 1, numBits) - coordinateShift);
    //   }

    static std::uint32_t decodeMorton(std::uint32_t code, int numBits) noexcept {
        std::uint32_t coordinate = 0;
        for (int i = 0; i < numBits; ++i) {
            coordinate |= (code & (1L << (2 * i))) >> i;
        }
        return coordinate;
    }

    std::uint32_t decodeFastPfor(BufferStream& buffer,
                                 std::uint32_t* const result,
                                 const std::size_t numValues,
                                 const std::size_t byteLength) noexcept(false) {
        const auto* inputValues = reinterpret_cast<const std::uint32_t*>(buffer.getReadPosition());
        auto resultCount = numValues;
        codec.decodeArray(inputValues, byteLength / sizeof(std::uint32_t), result, resultCount);
        return resultCount;

#if 0
            var encodedValuesSlice = Arrays.copyOfRange(encodedValues, pos.get(), pos.get() + byteLength);
            // TODO: get rid of that conversion
            IntBuffer intBuf =
                ByteBuffer.wrap(encodedValuesSlice)
                    // TODO: change to little endian
                    .order(ByteOrder.BIG_ENDIAN)
                    .asIntBuffer();
            int[] intValues = new int[(int) Math.ceil(byteLength / 4)];
            for (var i = 0; i < intValues.length; i++) {
            intValues[i] = intBuf.get(i);
            }

            int[] decodedValues = new int[numValues];
            var inputOffset = new IntWrapper(0);
            var outputOffset = new IntWrapper(0);
            IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
            ic.uncompress(intValues, inputOffset, intValues.length, decodedValues, outputOffset);

            pos.add(byteLength);
            return decodedValues;
#endif
    }
};

#if 0

  public static int[] decodeFastPforDeltaCoordinates(
      byte[] encodedValues, int numValues, int byteLength, IntWrapper pos) {
    var encodedValuesSlice = Arrays.copyOfRange(encodedValues, pos.get(), pos.get() + byteLength);
    // TODO: get rid of that conversion
    IntBuffer intBuf =
        ByteBuffer.wrap(encodedValuesSlice)
            // TODO: change to little endian
            .order(ByteOrder.BIG_ENDIAN)
            .asIntBuffer();
    int[] intValues = new int[(int) Math.ceil(byteLength / 4)];
    for (var i = 0; i < intValues.length; i++) {
      intValues[i] = intBuf.get(i);
    }

    int[] decompressedValues = new int[numValues];
    var inputOffset = new IntWrapper(0);
    var outputOffset = new IntWrapper(0);
    IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
    ic.uncompress(intValues, inputOffset, intValues.length, decompressedValues, outputOffset);

    var decodedValues = new int[numValues];
    for (var i = 0; i < numValues; i++) {
      var zigZagValue = decompressedValues[i];
      decodedValues[i] = (zigZagValue >>> 1) ^ (-(zigZagValue & 1));
    }

    pos.set(pos.get() + byteLength);

    var values = new int[numValues];
    var previousValueX = 0;
    var previousValueY = 0;
    for (var i = 0; i < numValues; i += 2) {
      var deltaX = decodedValues[i];
      var deltaY = decodedValues[i + 1];
      var x = previousValueX + deltaX;
      var y = previousValueY + deltaY;
      values[i] = x;
      values[i + 1] = y;

      previousValueX = x;
      previousValueY = y;
    }

    return values;
  }

  public static void decodeFastPforOptimized(
      byte[] buffer, IntWrapper offset, int byteLength, int[] decodedValues) {
    // TODO: get rid of that conversion
    IntBuffer intBuf =
        ByteBuffer.wrap(buffer, offset.get(), byteLength)
            // TODO: change to little endian
            .order(ByteOrder.BIG_ENDIAN)
            .asIntBuffer();
    int[] intValues = new int[(int) Math.ceil(byteLength / 4)];
    for (var i = 0; i < intValues.length; i++) {
      intValues[i] = intBuf.get(i);
    }

    IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
    ic.uncompress(intValues, new IntWrapper(0), intValues.length, decodedValues, new IntWrapper(0));

    offset.add(byteLength);
  }
#endif

} // namespace mlt::decoder
