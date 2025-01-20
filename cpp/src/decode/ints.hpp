#pragma once

#include <metadata/stream.hpp>
#include <util/buffer_stream.hpp>

#include /* fastpfor/ */ < fastpfor.h>
#include /* fastpfor/ */ < compositecodec.h>

#include <stdexcept>
#include <string>
#include <utility>
#include <vector>
#include "variablebyte.h"

namespace mlt::decoder {

class IntegerDecoder {
public:
    using StreamMetadata = metadata::stream::StreamMetadata;

    IntegerDecoder() = default;
    IntegerDecoder(const IntegerDecoder&) = delete;
    IntegerDecoder(IntegerDecoder&&) = default;
    ~IntegerDecoder() = default;

    template <typename T, typename TTarget = T>
    void decodeIntArray(const std::vector<T>& values,
                        std::vector<TTarget>& out,
                        const StreamMetadata& streamMetadata,
                        bool isSigned) {
        switch (streamMetadata.getLogicalLevelTechnique1()) {
            case metadata::stream::LogicalLevelTechnique::NONE:
                // if (isSigned) {
                //     return decodeZigZag(values);
                // }
                // return Arrays.stream(values).boxed().collect(Collectors.toList());
                throw std::runtime_error("Logical level technique NONE not implemented");
            case metadata::stream::LogicalLevelTechnique::DELTA:
                // if (streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.RLE)) {
                //     var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
                //     values =
                //         DecodingUtils.decodeUnsignedRLE(
                //             values, rleMetadata.runs(), rleMetadata.numRleValues());
                //     return decodeZigZagDelta(values);
                // }
                // return decodeZigZagDelta(values);
                throw std::runtime_error("Logical level technique DELTA not implemented");
            case metadata::stream::LogicalLevelTechnique::COMPONENTWISE_DELTA:
                // VectorizedDecodingUtils.decodeComponentwiseDeltaVec2(values);
                // return Arrays.stream(values).boxed().collect(Collectors.toList());
                throw std::runtime_error("Logical level technique COMPONENTWISE_DELTA not implemented");
            case metadata::stream::LogicalLevelTechnique::RLE: {
                // auto rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
                // auto decodedValues = decodeRLE(values, rleMetadata.runs(), rleMetadata.numRleValues());
                // return isSigned
                //     ? decodeZigZag(decodedValues.stream().mapToInt(i -> i).toArray())
                //     : decodedValues;
                throw std::runtime_error("Logical level technique RLE not implemented");
            }
            case metadata::stream::LogicalLevelTechnique::MORTON:
                // TODO: zig-zag decode when morton second logical level technique
                // return decodeMortonCodes(
                //    values,
                //    ((MortonEncodedStreamMetadata) streamMetadata).numBits(),
                //    ((MortonEncodedStreamMetadata) streamMetadata).coordinateShift());
                throw std::runtime_error("Logical level technique MORTON not implemented");
            case metadata::stream::LogicalLevelTechnique::PSEUDODECIMAL:
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
                         bool isSigned) {
        using namespace util::decoding;
        using namespace metadata::stream;

        std::vector<TDecode> values(metadata.getNumValues());
        switch (metadata.getPhysicalLevelTechnique()) {
            case PhysicalLevelTechnique::FAST_PFOR:
                decodeFastPfor(buffer, values.data(), metadata.getNumValues(), metadata.getByteLength());
                break;
            case PhysicalLevelTechnique::VARINT:
                decodeVarints<TDecode, TTarget>(buffer, out);
                break;
            default:
                throw std::runtime_error("Specified physical level technique not yet supported " +
                                         std::to_string(std::to_underlying(metadata.getPhysicalLevelTechnique())));
        }
        decodeIntArray<TDecode, TTarget>(values, out, metadata, isSigned);
    }

private:
    FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte> codec;

    template <typename T>
    void decodeFastPfor(BufferStream& buffer,
                        T* const result,
                        const std::size_t numValues,
                        const std::size_t byteLength) {
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
    }
};

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
  }

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
