import { encodeVarintInt32Array, encodeVarintInt64Array, encodeZigZag64 } from "./encodingUtils";
import { type RleEncodedStreamMetadata, type StreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";
import {
    encodeDeltaRleInt32,
    encodeZigZagInt32,
    encodeZigZagRleInt32,
    encodeUnsignedRleInt32,
    encodeDeltaInt32,
    encodeUnsignedRleFloat64,
    encodeZigZagDeltaFloat64,
    encodeZigZagFloat64,
    encodeZigZagRleFloat64,
} from "./integerEncodingUtils";

export function encodeIntStream(values: Int32Array, metadata: StreamMetadata, isSigned: boolean): Uint8Array {
    const { data } = encodeIntBuffer(values, metadata, isSigned);
    return encodeVarintInt32Array(data);
}

function encodeIntBuffer(
    values: Int32Array,
    streamMetadata: StreamMetadata,
    isSigned: boolean,
): { data: Int32Array; runs?: number } {
    const data = new Int32Array(values);
    switch (streamMetadata.logicalLevelTechnique1) {
        case LogicalLevelTechnique.DELTA:
            if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                const encoded = encodeDeltaRleInt32(data);
                return { data: encoded.data, runs: encoded.runs };
            }
            encodeDeltaInt32(data);
            encodeZigZagInt32(data);
            return { data };
        case LogicalLevelTechnique.RLE: {
            if (isSigned) {
                const encoded = encodeZigZagRleInt32(data);
                return { data: encoded.data, runs: encoded.runs };
            }
            const encoded = encodeUnsignedRleInt32(data);
            return { data: encoded.data, runs: encoded.runs };
        }
        case LogicalLevelTechnique.MORTON:
            encodeDeltaInt32(data);
            return { data };
        case LogicalLevelTechnique.COMPONENTWISE_DELTA:
            throw new Error("COMPONENTWISE_DELTA encoding not implemented yet");
        case LogicalLevelTechnique.NONE:
            if (isSigned) {
                encodeZigZagInt32(data);
            }
            return { data };
        default:
            throw new Error(
                `The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`,
            );
    }
}

export function encodeFloat64(values: Float64Array, streamMetadata: StreamMetadata, isSigned: boolean): Float64Array {
    switch (streamMetadata.logicalLevelTechnique1) {
        case LogicalLevelTechnique.DELTA:
            encodeZigZagDeltaFloat64(values);
            if (streamMetadata.logicalLevelTechnique2 === LogicalLevelTechnique.RLE) {
                //const rleMetadata = streamMetadata as RleEncodedStreamMetadata;
                values = encodeUnsignedRleFloat64(values).data; //, rleMetadata.runs, rleMetadata.numRleValues);
            }
            return values;
        case LogicalLevelTechnique.RLE:
            return encodeRleFloat64(values, streamMetadata as RleEncodedStreamMetadata, isSigned);
        case LogicalLevelTechnique.NONE:
            if (isSigned) {
                encodeZigZagFloat64(values);
            }
            return values;
        default:
            throw new Error(
                `The specified Logical level technique is not supported: ${streamMetadata.logicalLevelTechnique1}`,
            );
    }
}

function encodeRleFloat64(
    data: Float64Array,
    streamMetadata: RleEncodedStreamMetadata,
    isSigned: boolean,
): Float64Array {
    return isSigned ? encodeZigZagRleFloat64(data).data : encodeUnsignedRleFloat64(data).data;
}

/**
 * Encodes Int32 values with MORTON encoding (delta without zigzag)
 */
export function encodeInt32Morton(values: Int32Array): Uint8Array {
    const deltaEncoded = new Int32Array(values.length);
    deltaEncoded[0] = values[0];
    for (let i = 1; i < values.length; i++) {
        deltaEncoded[i] = values[i] - values[i - 1];
    }
    return encodeVarintInt32Array(deltaEncoded);
}

/**
 * Encodes BigInt64 values with zigzag encoding and varint compression
 */
export function encodeInt64SignedNone(values: BigInt64Array): Uint8Array {
    const zigzagEncoded = new BigInt64Array(Array.from(values, (val) => encodeZigZag64(val)));
    return encodeVarintInt64Array(zigzagEncoded);
}

/**
 * Encodes BigInt64 values with delta encoding, zigzag, and varint
 */
export function encodeInt64SignedDelta(values: BigInt64Array): Uint8Array {
    const deltaEncoded = new BigInt64Array(values.length);
    deltaEncoded[0] = values[0];
    for (let i = 1; i < values.length; i++) {
        deltaEncoded[i] = values[i] - values[i - 1];
    }
    const zigzagEncoded = new BigInt64Array(deltaEncoded.length);
    for (let i = 0; i < deltaEncoded.length; i++) {
        zigzagEncoded[i] = encodeZigZag64(deltaEncoded[i]);
    }
    return encodeVarintInt64Array(zigzagEncoded);
}

/**
 * Encodes BigInt64 values with RLE, zigzag, and varint
 * @param runs - Array of [runLength, value] pairs
 */
export function encodeInt64SignedRle(runs: Array<[number, bigint]>): Uint8Array {
    const runLengths: bigint[] = [];
    const values: bigint[] = [];

    for (const [runLength, value] of runs) {
        runLengths.push(BigInt(runLength));
        values.push(encodeZigZag64(value));
    }

    const rleValues = [...runLengths, ...values];
    return encodeVarintInt64Array(new BigInt64Array(rleValues));
}

/**
 * Encodes BigInt64 values with delta+RLE, zigzag, and varint
 * @param runs - Array of [runLength, deltaValue] pairs representing RLE-encoded delta values
 */
export function encodeInt64SignedDeltaRle(runs: Array<[number, bigint]>): Uint8Array {
    const runLengths: bigint[] = [];
    const values: bigint[] = [];

    for (const [runLength, value] of runs) {
        runLengths.push(BigInt(runLength));
        values.push(encodeZigZag64(value));
    }

    const rleValues = [...runLengths, ...values];
    return encodeVarintInt64Array(new BigInt64Array(rleValues));
}

/**
 * Encodes unsigned BigInt64 values with varint compression (no zigzag)
 */
export function encodeInt64UnsignedNone(values: BigInt64Array): Uint8Array {
    return encodeVarintInt64Array(values);
}
