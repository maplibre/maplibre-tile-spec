import { DecodingUtils } from './DecodingUtils';
import { IntWrapper } from './IntWrapper';
import { StreamMetadata } from '../metadata/stream/StreamMetadata';
import { LogicalLevelTechnique } from '../metadata/stream/LogicalLevelTechnique';
import { PhysicalLevelTechnique } from '../metadata/stream/PhysicalLevelTechnique';
import { MortonEncodedStreamMetadata } from '../metadata/stream/MortonEncodedStreamMetadata';
import { RleEncodedStreamMetadata } from '../metadata/stream/RleEncodedStreamMetadata';

class IntegerDecoder {

    public static decodeMortonStream(data: Uint8Array, offset: IntWrapper, streamMetadata: MortonEncodedStreamMetadata): number[] {
        let values: number[];
        if (streamMetadata.physicalLevelTechnique() === PhysicalLevelTechnique.FAST_PFOR) {
            throw new Error("Specified physical level technique not yet supported: " + streamMetadata.physicalLevelTechnique());
            // TODO
            //values = DecodingUtils.decodeFastPfor128(data, streamMetadata.numValues(), streamMetadata.byteLength(), offset);
        } else if (streamMetadata.physicalLevelTechnique() === PhysicalLevelTechnique.VARINT) {
            values = DecodingUtils.decodeVarint(data, offset, streamMetadata.numValues());
        } else {
            throw new Error("Specified physical level technique not yet supported: " + streamMetadata.physicalLevelTechnique());
        }

        return this.decodeMortonDelta(values, streamMetadata.numBits(), streamMetadata.coordinateShift());
    }

    private static decodeMortonDelta(data: number[], numBits: number, coordinateShift: number): number[] {
        const vertices: number[] = [];
        let previousMortonCode = 0;
        for (const deltaCode of data) {
            const mortonCode = previousMortonCode + deltaCode;
            const vertex = this.decodeMortonCode(mortonCode, numBits, coordinateShift);
            vertices.push(vertex[0], vertex[1]);
            previousMortonCode = mortonCode;
        }
        return vertices;
    }

    private static decodeMortonCodes(data: number[], numBits: number, coordinateShift: number): number[] {
        const vertices: number[] = [];
        for (const mortonCode of data) {
            const vertex = this.decodeMortonCode(mortonCode, numBits, coordinateShift);
            vertices.push(vertex[0], vertex[1]);
        }
        return vertices;
    }

    private static decodeMortonCode(mortonCode: number, numBits: number, coordinateShift: number): number[] {
        const x = this.decodeMorton(mortonCode, numBits) - coordinateShift;
        const y = this.decodeMorton(mortonCode >> 1, numBits) - coordinateShift;
        return [x, y];
    }

    private static decodeMorton(code: number, numBits: number): number {
        let coordinate = 0;
        for (let i = 0; i < numBits; i++) {
            coordinate |= (code & (1 << (2 * i))) >> i;
        }
        return coordinate;
    }

    public static decodeIntStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata, isSigned: boolean): number[] {
        let values: number[];
        if (streamMetadata.physicalLevelTechnique() === PhysicalLevelTechnique.FAST_PFOR) {
            throw new Error("Specified physical level technique not yet supported: " + streamMetadata.physicalLevelTechnique());
            // TODO
            //values = DecodingUtils.decodeFastPfor128(data, streamMetadata.numValues(), streamMetadata.byteLength(), offset);
        } else if (streamMetadata.physicalLevelTechnique() === PhysicalLevelTechnique.VARINT) {
            values = DecodingUtils.decodeVarint(data, offset, streamMetadata.numValues());
        } else {
            throw new Error("Specified physical level technique not yet supported: " + streamMetadata.physicalLevelTechnique());
        }
        return this.decodeIntArray(values, streamMetadata, isSigned);
    }

    private static decodeIntArray(values: number[], streamMetadata: StreamMetadata, isSigned: boolean): number[] {
        switch (streamMetadata.logicalLevelTechnique1()) {
            case LogicalLevelTechnique.DELTA: {
                if (streamMetadata.logicalLevelTechnique2() === LogicalLevelTechnique.RLE) {
                    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                    values =
                        DecodingUtils.decodeUnsignedRLE(
                            values, rleMetadata.runs(), rleMetadata.numRleValues());
                    return this.decodeZigZagDelta(values);
                }
                return this.decodeZigZagDelta(values);
            }
            case LogicalLevelTechnique.RLE: {
                const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                const decodedValues = this.decodeRLE(values, rleMetadata.runs(), rleMetadata.numRleValues());
                return isSigned ? this.decodeZigZag(decodedValues) : decodedValues;
            }
            case LogicalLevelTechnique.NONE: {
                return isSigned ? this.decodeZigZag(values) : values;
            }
            case LogicalLevelTechnique.MORTON: {
                const mortonMetadata = streamMetadata as MortonEncodedStreamMetadata;
                return this.decodeMortonCodes(values, mortonMetadata.numBits(), mortonMetadata.coordinateShift());
            }
            case LogicalLevelTechnique.COMPONENTWISE_DELTA: {
                DecodingUtils.decodeComponentwiseDeltaVec2(values);
                return values;
            }
            default:
                throw new Error("The specified logical level technique is not supported for integers: " + streamMetadata.logicalLevelTechnique1());
        }
    }

    public static decodeLongStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata, isSigned: boolean): bigint[] {
        if (streamMetadata.physicalLevelTechnique() !== PhysicalLevelTechnique.VARINT) {
            throw new Error("Specified physical level technique not yet supported: " + streamMetadata.physicalLevelTechnique());
        }

        const values = DecodingUtils.decodeLongVarint(data, offset, streamMetadata.numValues());
        return this.decodeLongArray(values, streamMetadata, isSigned);
    }

    private static decodeLongArray(values: bigint[], streamMetadata: StreamMetadata, isSigned: boolean): bigint[] {
        switch (streamMetadata.logicalLevelTechnique1()) {
            case LogicalLevelTechnique.DELTA: {
                if (streamMetadata.logicalLevelTechnique2() === LogicalLevelTechnique.RLE) {
                    const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                    values =
                        DecodingUtils.decodeUnsignedRLELong(
                            values, rleMetadata.runs(), rleMetadata.numRleValues());
                    return this.decodeLongZigZagDelta(values);
                }
                return this.decodeLongZigZagDelta(values);
            }
            case LogicalLevelTechnique.RLE: {
                const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                const decodedValues = this.decodeLongRLE(values, rleMetadata.runs(), rleMetadata.numRleValues());
                return isSigned ? this.decodeZigZagLong(decodedValues) : decodedValues;
            }
            case LogicalLevelTechnique.NONE: {
                return isSigned ? this.decodeZigZagLong(values) : values;
            }
            default:
                throw new Error("The specified logical level technique is not supported for integers: " + streamMetadata.logicalLevelTechnique1());
        }
    }

    private static decodeRLE(data: number[], numRuns: number, numRleValues: number): number[] {
        const values = new Array<number>(numRleValues);
        for (let i = 0; i < numRuns; i++) {
            const run = data[i];
            const value = data[i + numRuns];
            for (let j = 0; j < run; j++) {
                values.push(value);
            }
        }
        return values;
    }

    private static decodeLongRLE(data: bigint[], numRuns: number, numRleValues: number): bigint[] {
        const values = new Array<bigint>(numRleValues);
        for (let i = 0; i < numRuns; i++) {
            const run = data[i];
            const value = data[i + numRuns];
            for (let j = 0; j < run; j++) {
                values.push(value);
            }
        }
        return values;
    }

    private static decodeZigZagDelta(data: number[]): number[] {
        const values: number[] = [];
        let previousValue = 0;
        for (const zigZagDelta of data) {
            const delta = DecodingUtils.decodeZigZag(zigZagDelta);
            const value = previousValue + delta;
            values.push(value);
            previousValue = value;
        }
        return values;
    }

    private static decodeDelta(data: number[]): number[] {
        const values: number[] = [];
        let previousValue = 0;
        for (const delta of data) {
            const value = previousValue + delta;
            values.push(value);
            previousValue = value;
        }
        return values;
    }

    private static decodeLongZigZagDelta(data: bigint[]): bigint[] {
        const values: bigint[] = [];
        let previousValue = BigInt(0);
        for (const zigZagDelta of data) {
            const delta = DecodingUtils.decodeZigZagLong(zigZagDelta);
            const value = previousValue + delta;
            values.push(value);
            previousValue = value;
        }
        return values;
    }

    private static decodeZigZag(data: number[]): number[] {
        return data.map(zigZagDelta => DecodingUtils.decodeZigZag(zigZagDelta));
    }

    private static decodeZigZagLong(data: bigint[]): bigint[] {
        return data.map(zigZagDelta => DecodingUtils.decodeZigZagLong(zigZagDelta));
    }
}

export { IntegerDecoder };
