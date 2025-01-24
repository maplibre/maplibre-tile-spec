#pragma once

#include <cstdint>
#include <metadata/stream.hpp>
#include <type_traits>
#include <util/buffer_stream.hpp>
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

    IntegerDecoder() noexcept(false) = default;
    IntegerDecoder(const IntegerDecoder&) = delete;
    IntegerDecoder(IntegerDecoder&&) noexcept(false) = default;
    ~IntegerDecoder() = default;

    IntegerDecoder& operator=(const IntegerDecoder&) = delete;
    IntegerDecoder& operator=(IntegerDecoder&&) = delete;

    template <typename T, typename TTarget = T>
        requires(std::is_integral_v<T> && (std::is_integral_v<TTarget> || std::is_enum_v<TTarget>) &&
                 sizeof(T) <= sizeof(TTarget))
    void decodeIntArray(const std::vector<T>& values,
                        std::vector<TTarget>& out,
                        const StreamMetadata& streamMetadata,
                        bool isSigned) noexcept(false) {
        using namespace metadata::stream;
        using namespace util::decoding;
        switch (streamMetadata.getLogicalLevelTechnique1()) {
            case LogicalLevelTechnique::NONE: {
                assert(out.size() == values.size());
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
            case LogicalLevelTechnique::COMPONENTWISE_DELTA:
                if constexpr (std::is_same_v<TTarget, std::uint32_t>) {
                    out = values;
                    vectorized::decodeComponentwiseDeltaVec2(out);
                    break;
                }
                throw std::runtime_error(
                    "Logical level technique COMPONENTWISE_DELTA not implemented for 64-bit values");
            case LogicalLevelTechnique::RLE: {
                // auto rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
                // auto decodedValues = decodeRLE(values, rleMetadata.runs(), rleMetadata.numRleValues());
                // return isSigned
                //     ? decodeZigZag(decodedValues.stream().mapToInt(i -> i).toArray())
                //     : decodedValues;
                throw std::runtime_error("Logical level technique RLE not implemented");
            }
            case LogicalLevelTechnique::MORTON:
                // TODO: zig-zag decode when morton second logical level technique
                // return decodeMortonCodes(
                //    values,
                //    ((MortonEncodedStreamMetadata) streamMetadata).numBits(),
                //    ((MortonEncodedStreamMetadata) streamMetadata).coordinateShift());
                throw std::runtime_error("Logical level technique MORTON not implemented");
            case LogicalLevelTechnique::PSEUDODECIMAL:
            default:
                throw std::runtime_error(
                    "The specified logical level technique is not supported for integers: " +
                    std::to_string(std::to_underlying(streamMetadata.getLogicalLevelTechnique1())));
                break;
        }
    }

    template <typename TDecode, typename TTarget = TDecode>
    void decodeIntStream(BufferStream& buffer,
                         std::vector<TTarget>& out,
                         const StreamMetadata& metadata,
                         bool isSigned) noexcept(false) {
        using namespace util::decoding;
        using namespace metadata::stream;

        std::vector<TDecode> values(metadata.getNumValues());
        switch (metadata.getPhysicalLevelTechnique()) {
            case PhysicalLevelTechnique::FAST_PFOR:
                decodeFastPfor(buffer, values.data(), metadata.getNumValues(), metadata.getByteLength());
                break;
            case PhysicalLevelTechnique::VARINT:
                decodeVarints<TDecode>(buffer, values);
                break;
            default:
                throw std::runtime_error("Specified physical level technique not yet supported " +
                                         std::to_string(std::to_underlying(metadata.getPhysicalLevelTechnique())));
        }
        out.resize(metadata.getNumValues());
        decodeIntArray<TDecode, TTarget>(values, out, metadata, isSigned);
    }

private:
    FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte> codec;

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
        assert(values.size() == out.size());
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

    template <typename T>
    void decodeFastPfor(BufferStream& buffer,
                        T* const result,
                        const std::size_t numValues,
                        const std::size_t byteLength) noexcept(false) {
        const auto* inputValues = reinterpret_cast<const std::uint32_t*>(buffer.getReadPosition());
        auto resultCount = numValues;
        codec.decodeArray(inputValues, byteLength / sizeof(T), result, resultCount);

        /*
            using X = T;
            std::vector<X> data(256);
            std::iota(data.begin(), data.end(), 0);
            std::vector<std::uint32_t> encoded(data.size() * 2, ~0);
            auto codec = new FastPForLib::FastPFor<sizeof(X)>();
            size_t encodeSize = data.size();
            codec->encodeArray(reinterpret_cast<const X*>(data.data()),
                                data.size(),
                                encoded.data(),
                                encodeSize);

            std::vector<X> decoded(data.size() * 2, ~0);
            auto decodeSize = decoded.size();
            codec->decodeArray(encoded.data(), encodeSize, decoded.data(), decodeSize);
            assert(!memcmp(decoded.data(), data.data(), data.size() * sizeof(T)));
            */
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
