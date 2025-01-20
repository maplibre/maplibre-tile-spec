#pragma once

#include <metadata/stream.hpp>
#include <util/buffer_stream.hpp>

#include <functional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace mlt::decoder {

class IntegerDecoder {
public:
    using StreamMetadata = metadata::stream::StreamMetadata;

    IntegerDecoder() = delete;
    IntegerDecoder(const IntegerDecoder&) = delete;
    IntegerDecoder(IntegerDecoder&&) = default;

    template <typename T, typename TTarget = T>
    static void decodeIntArray(const std::vector<T>& values, std::vector<TTarget>& out, const StreamMetadata& streamMetadata, bool isSigned) {
    switch (streamMetadata.getLogicalLevelTechnique1()) {
        case metadata::stream::LogicalLevelTechnique::NONE:
            //if (isSigned) {
            //    return decodeZigZag(values);
            //}
            //return Arrays.stream(values).boxed().collect(Collectors.toList());
            throw std::runtime_error("Logical level technique NONE not implemented");
        case metadata::stream::LogicalLevelTechnique::DELTA:
            //if (streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.RLE)) {
            //    var rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
            //    values =
            //        DecodingUtils.decodeUnsignedRLE(
            //            values, rleMetadata.runs(), rleMetadata.numRleValues());
            //    return decodeZigZagDelta(values);
            //}
            //return decodeZigZagDelta(values);
            throw std::runtime_error("Logical level technique DELTA not implemented");
        case metadata::stream::LogicalLevelTechnique::COMPONENTWISE_DELTA:
            //VectorizedDecodingUtils.decodeComponentwiseDeltaVec2(values);
            //return Arrays.stream(values).boxed().collect(Collectors.toList());
            throw std::runtime_error("Logical level technique COMPONENTWISE_DELTA not implemented");
        case metadata::stream::LogicalLevelTechnique::RLE: {
            //auto rleMetadata = (RleEncodedStreamMetadata) streamMetadata;
            //auto decodedValues = decodeRLE(values, rleMetadata.runs(), rleMetadata.numRleValues());
            //return isSigned
            //    ? decodeZigZag(decodedValues.stream().mapToInt(i -> i).toArray())
            //    : decodedValues;
            throw std::runtime_error("Logical level technique RLE not implemented");
        }
        case metadata::stream::LogicalLevelTechnique::MORTON:
            // TODO: zig-zag decode when morton second logical level technique
            //return decodeMortonCodes(
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
    static void decodeIntStream(BufferStream& buffer, std::vector<TTarget>& out, const StreamMetadata& metadata, bool isSigned) {
        using namespace util::decoding;
        using namespace metadata::stream;

        std::vector<TDecode> values(metadata.getNumValues());
        switch (metadata.getPhysicalLevelTechnique()) {
            case PhysicalLevelTechnique::FAST_PFOR:
                //decodeFastPfor(buffer, metadata.getNumValues(), metadata.getByteLength(), values);
                throw std::runtime_error("FastPFor not implemented");
                break;
            case PhysicalLevelTechnique::VARINT:
                decodeVarints<TDecode, TTarget>(buffer, out);
                break;
            default:
                throw std::runtime_error("Specified physical level technique not yet supported " + std::to_string(std::to_underlying(metadata.getPhysicalLevelTechnique())));
        }
        decodeIntArray<TDecode, TTarget>(values, out, metadata, isSigned);
    }
};

} // namespace mlt::decoder
