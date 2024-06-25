import {StreamMetadata} from "../metadata/tile/streamMetadata";
import {PhysicalLevelTechnique} from "../metadata/tile/physicalLevelTechnique";
import IntWrapper from "./intWrapper";
import {
    decodeComponentwiseDeltaVec2,
    decodeComponentwiseDeltaVec2Scaled,
    decodeFastPfor,
    decodeNullableRle,
    decodeNullableRleInt64,
    decodeNullableZigZagDelta,
    decodeNullableZigZagDeltaInt64,
    decodeRle, decodeRleFloat64,
    decodeRleInt64,
    decodeUnsignedConstRle,
    decodeUnsignedConstRleInt64,
    decodeUnsignedRle, decodeUnsignedRleFloat64,
    decodeUnsignedRleInt64,
    decodeVarintInt32,
    decodeVarintInt64, decodeVarintFloat64,
    decodeZigZag,
    decodeZigZagConstRle,
    decodeZigZagConstRleInt64,
    decodeZigZagDelta, decodeZigZagDeltaFloat64,
    decodeZigZagDeltaInt64, decodeZigZagFloat64,
    decodeZigZagInt64,
    decodeZigZagSequenceRle,
    decodeZigZagSequenceRleInt64,
    decodeZigZagValue,
    decodeZigZagValueInt64,
    fastInverseDelta, inverseDelta,
    padWithZeros,
    padWithZerosInt64,
    padZigZagWithZeros,
    padZigZagWithZerosInt64,
    rleDeltaDecoding,
    zigZagDeltaOfDeltaDecoding,
    zigZagRleDeltaDecoding
} from "./integerDecodingUtils";
import {LogicalLevelTechnique} from "../metadata/tile/logicalLevelTechnique";
import {RleEncodedStreamMetadata} from "../metadata/tile/rleEncodedStreamMetadata";
import BitVector from "../vector/flat/bitVector";
import {VectorType} from "../vector/vectorType";
import GeometryScaling from "./geometryScaling";


export default class IntegerStreamDecoder {
    private constructor() {}

    static decodeIntStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata, isSigned: boolean,
                           scalingData?: GeometryScaling): Int32Array {
        const values = IntegerStreamDecoder.decodePhysicalLevelTechnique(data, offset, streamMetadata);
        return this.decodeIntBuffer(values, streamMetadata, isSigned, scalingData);
    }

    static decodeLengthStreamToOffsetBuffer(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata): Int32Array {
        const values = IntegerStreamDecoder.decodePhysicalLevelTechnique(data, offset, streamMetadata);
        return this.decodeLengthToOffsetBuffer(values, streamMetadata);
    }

    private static decodePhysicalLevelTechnique(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata){
        const physicalLevelTechnique = streamMetadata.physicalLevelTechnique;
        if(physicalLevelTechnique === PhysicalLevelTechnique.VARINT){
            return decodeVarintInt32(data, offset, streamMetadata.numValues);
        }

        if(physicalLevelTechnique === PhysicalLevelTechnique.NONE){
            const dataOffset = offset.get();
            const byteLength = streamMetadata.byteLength;
            offset.add(byteLength);
            //TODO: use Byte Rle for geometry type encoding
            const slice = data.subarray(dataOffset, offset.get());
            return new Int32Array(slice);
        }

        throw new Error("Specified physicalLevelTechnique is not supported (yet).");
    }

    static decodeConstIntStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata, isSigned: boolean): number {
        const values = IntegerStreamDecoder.decodePhysicalLevelTechnique(data, offset, streamMetadata);

        if (values.length === 1) {
            const value = values[0];
            return isSigned ? decodeZigZagValue(value) : value;
        }

        return isSigned
            ? decodeZigZagConstRle(values)
            : decodeUnsignedConstRle(values);
    }

    static decodeSequenceIntStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata): [baseValue: number, delta: number] {
        const values = IntegerStreamDecoder.decodePhysicalLevelTechnique(data, offset, streamMetadata);

        return decodeZigZagSequenceRle(values);
    }

    static decodeSequenceLongStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata): [baseValue: bigint, delta: bigint]  {
        const values = decodeVarintInt64(data, offset, streamMetadata.numValues);
        return decodeZigZagSequenceRleInt64(values);
    }

    static decodeLongStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata, isSigned: boolean): BigInt64Array {
        const values = decodeVarintInt64(data, offset, streamMetadata.numValues);
        return this.decodeLongBuffer(values, streamMetadata, isSigned);
    }

    static decodeLongFloat64Stream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata, isSigned: boolean): Float64Array {
        const values = decodeVarintFloat64(data, streamMetadata.numValues, offset);
        return this.decodeFloat64Buffer(values, streamMetadata, isSigned);
    }

    static decodeConstLongStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata, isSigned: boolean): bigint {
        const values = decodeVarintInt64(data, offset, streamMetadata.numValues);

        if (values.length === 1) {
            const value = values[0];
            return isSigned ? decodeZigZagValueInt64(value) : value;
        }

        return isSigned ? decodeZigZagConstRleInt64(values) : decodeUnsignedConstRleInt64(values);
    }

    private static decodeIntBuffer(values: Int32Array, streamMetadata: StreamMetadata, isSigned: boolean,
                                   scalingData?: GeometryScaling): Int32Array {
        /*
         * Currently the encoder uses only fixed combinations of encodings.
         * For performance reasons it is also used a fixed combination of the encodings on the decoding side.
         * The following encodings and combinations are used:
         *   - Morton Delta -> always sorted so not ZigZag encoding needed
         *   - Delta -> currently always in combination with ZigZag encoding
         *   - Rle -> in combination with ZigZag encoding if data type is signed
         *   - Delta Rle
         *   - Componentwise Delta -> always ZigZag encoding is used
        * */
        switch (streamMetadata.logicalLevelTechnique1) {
            case LogicalLevelTechnique.DELTA:
                if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                    values = decodeUnsignedRle(values, rleMetadata.runs, rleMetadata.numRleValues);
                }
                decodeZigZagDelta(values);
                return values;
            case LogicalLevelTechnique.RLE:
                return decodeRle(values, streamMetadata as RleEncodedStreamMetadata, isSigned);
            case LogicalLevelTechnique.MORTON:
                fastInverseDelta(values);
                return values;
            case LogicalLevelTechnique.COMPONENTWISE_DELTA:
                if(scalingData){
                    decodeComponentwiseDeltaVec2Scaled(values, scalingData.scale, scalingData.min, scalingData.max);
                    return values;
                }

                decodeComponentwiseDeltaVec2(values);
                return values;
            case LogicalLevelTechnique.NONE:
                if (isSigned) {
                    decodeZigZag(values);
                }
                return values;
            default:
                throw new Error(`The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`);
        }
    }

    private static decodeLongBuffer(values: BigInt64Array, streamMetadata: StreamMetadata, isSigned: boolean): BigInt64Array {
        switch (streamMetadata.logicalLevelTechnique1) {
            case LogicalLevelTechnique.DELTA:
                if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                    values = decodeUnsignedRleInt64(values, rleMetadata.runs, rleMetadata.numRleValues);
                }
                decodeZigZagDeltaInt64(values);
                return values;
            case LogicalLevelTechnique.RLE:
                return decodeRleInt64(values, streamMetadata as RleEncodedStreamMetadata, isSigned);
            case LogicalLevelTechnique.NONE:
                if (isSigned) {
                    decodeZigZagInt64(values);
                }
                return values;
            default:
                throw new Error(`The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`);
        }
    }

    private static decodeFloat64Buffer(values: Float64Array, streamMetadata: StreamMetadata, isSigned: boolean): Float64Array {
        switch (streamMetadata.logicalLevelTechnique1) {
            case LogicalLevelTechnique.DELTA:
                if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                    values = decodeUnsignedRleFloat64(values, rleMetadata.runs, rleMetadata.numRleValues);
                }
                decodeZigZagDeltaFloat64(values);
                return values;
            case LogicalLevelTechnique.RLE:
                return decodeRleFloat64(values, streamMetadata as RleEncodedStreamMetadata, isSigned);
            case LogicalLevelTechnique.NONE:
                if (isSigned) {
                    decodeZigZagFloat64(values);
                }
                return values;
            default:
                throw new Error(`The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`);
        }
    }

    private static decodeLengthToOffsetBuffer(values: Int32Array, streamMetadata: StreamMetadata): Int32Array {
        if (streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.DELTA && streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.NONE) {
            const decodedValues = zigZagDeltaOfDeltaDecoding(values);
            return decodedValues;
        }

        if (streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.RLE && streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.NONE) {
            const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
            const decodedValues = rleDeltaDecoding(values, rleMetadata.runs, rleMetadata.numRleValues);
            return decodedValues;
        }

        if (streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.NONE && streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.NONE) {
            //TODO: use fastInverseDelta again and check what are the performance problems in zoom 14
            //fastInverseDelta(values);
            inverseDelta(values);
            const offsets = new Int32Array(streamMetadata.numValues + 1);
            offsets[0] = 0;
            offsets.set(values, 1);
            return offsets;
        }

        if (streamMetadata.logicalLevelTechnique1 === LogicalLevelTechnique.DELTA && streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
            const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
            const decodedValues = zigZagRleDeltaDecoding(values, rleMetadata.runs, rleMetadata.numRleValues);
            fastInverseDelta(decodedValues);
            return decodedValues;
        }

        throw new Error("Only delta encoding is supported for transforming length to offset streams yet.");
    }

    public static decodeNullableIntStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata, isSigned: boolean, bitVector: BitVector): Int32Array {
        const values = streamMetadata.physicalLevelTechnique === PhysicalLevelTechnique.FAST_PFOR
            ? decodeFastPfor(data, streamMetadata.numValues, streamMetadata.byteLength, offset)
            : decodeVarintInt32(data, offset, streamMetadata.numValues);

        return this.decodeNullableIntBuffer(values, streamMetadata, isSigned, bitVector);
    }

    public static decodeNullableLongStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata, isSigned: boolean, bitVector: BitVector): BigInt64Array {
        const values = decodeVarintInt64(data, offset, streamMetadata.numValues);
        return this.decodeNullableLongBuffer(values, streamMetadata, isSigned, bitVector);
    }

    private static decodeNullableIntBuffer(values: Int32Array, streamMetadata: StreamMetadata, isSigned: boolean, bitVector: BitVector): Int32Array {
        switch (streamMetadata.logicalLevelTechnique1) {
            case LogicalLevelTechnique.DELTA:
                if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                    values = decodeUnsignedRle(values, rleMetadata.runs, rleMetadata.numRleValues);
                }
                return decodeNullableZigZagDelta(bitVector, values);
            case LogicalLevelTechnique.RLE:
                return decodeNullableRle(values, streamMetadata, isSigned, bitVector);
            case LogicalLevelTechnique.MORTON:
                fastInverseDelta(values);
                return values;
            case LogicalLevelTechnique.COMPONENTWISE_DELTA:
                decodeComponentwiseDeltaVec2(values);
                return values;
            case LogicalLevelTechnique.NONE:
                values = isSigned
                    ? padZigZagWithZeros(bitVector, values)
                    : padWithZeros(bitVector, values);
                return values;
            default:
                throw new Error("The specified Logical level technique is not supported");
        }
    }

    private static decodeNullableLongBuffer(values: BigInt64Array, streamMetadata: StreamMetadata, isSigned: boolean, bitVector: BitVector): BigInt64Array {
        switch (streamMetadata.logicalLevelTechnique1) {
            case LogicalLevelTechnique.DELTA:
                if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                    values = decodeUnsignedRleInt64(values, rleMetadata.runs, rleMetadata.numRleValues);
                }
                return  decodeNullableZigZagDeltaInt64(bitVector, values);
            case LogicalLevelTechnique.RLE:
                return decodeNullableRleInt64(values, streamMetadata, isSigned, bitVector);
            case LogicalLevelTechnique.NONE:
                values = isSigned
                    ? padZigZagWithZerosInt64(bitVector, values)
                    : padWithZerosInt64(bitVector, values);
                return values;
            default:
                throw new Error("The specified Logical level technique is not supported");
        }
    }

    static getVectorTypeIntStream(streamMetadata: StreamMetadata): VectorType {
        const  logicalLevelTechnique1 = streamMetadata.logicalLevelTechnique1;
        if (logicalLevelTechnique1 === LogicalLevelTechnique.RLE) {
            return (streamMetadata as RleEncodedStreamMetadata).runs == 1
                ? VectorType.CONST
                : VectorType.FLAT;
        }

        if (logicalLevelTechnique1 === LogicalLevelTechnique.DELTA
            && streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE
            /* If base value equals delta value then one run else two runs */
            && ((streamMetadata as RleEncodedStreamMetadata).runs === 1
            || (streamMetadata as RleEncodedStreamMetadata).runs === 2)) {
            return VectorType.SEQUENCE;
        }

        return streamMetadata.numValues === 1 ? VectorType.CONST : VectorType.FLAT;
    }
    
}
