import FeatureTable from "./vector/featureTable";
import { type Column, LogicalScalarType, ScalarType } from "./metadata/tileset/tilesetMetadata";
import IntWrapper from "./decoding/intWrapper";
import { decodeStreamMetadata, type RleEncodedStreamMetadata } from "./metadata/tile/streamMetadataDecoder";
import { VectorType } from "./vector/vectorType";
import { Int32FlatVector } from "./vector/flat/int32FlatVector";
import BitVector from "./vector/flat/bitVector";
import {
    decodeUnsignedConstInt32Stream,
    decodeUnsignedConstInt64Stream,
    decodeUnsignedInt64AsFloat64Stream,
    decodeUnsignedInt32Stream,
    decodeUnsignedInt64Stream,
    decodeSequenceInt32Stream,
    decodeSequenceInt64Stream,
    getVectorType,
} from "./decoding/integerStreamDecoder";
import { Int32SequenceVector } from "./vector/sequence/int32SequenceVector";
import { Int64FlatVector } from "./vector/flat/int64FlatVector";
import { Int64SequenceVector } from "./vector/sequence/int64SequenceVector";
import type { IdVector } from "./vector/idVector";
import { decodeVarintInt32Value } from "./decoding/integerDecodingUtils";
import { decodeGeometryColumn } from "./decoding/geometryDecoder";
import { decodePropertyColumn } from "./decoding/propertyDecoder";
import { Int32ConstVector } from "./vector/constant/int32ConstVector";
import { Int64ConstVector } from "./vector/constant/int64ConstVector";
import type GeometryScaling from "./decoding/geometryScaling";
import { decodeBooleanRle } from "./decoding/decodingUtils";
import { DoubleFlatVector } from "./vector/flat/doubleFlatVector";
import { decodeEmbeddedTileSetMetadata } from "./metadata/tileset/embeddedTilesetMetadataDecoder";
import { hasStreamCount, isGeometryColumn, isLogicalIdColumn } from "./metadata/tileset/typeMap";
import type { StreamMetadata } from "./metadata/tile/streamMetadataDecoder";
import type { GeometryVector } from "./vector/geometry/geometryVector";
import type Vector from "./vector/vector";
import type { GpuVector } from "./vector/geometry/gpuVector";

/**
 * Decodes a tile with embedded metadata (Tag 0x01 format).
 * This is the primary decoder function for MLT tiles.
 *
 * @param tile The tile data to decode (will be decompressed if gzip-compressed)
 * @param geometryScaling Optional geometry scaling parameters
 * @param idWithinMaxSafeInteger If true, limits ID values to JavaScript safe integer range (53 bits)
 */
export default function decodeTile(
    tile: Uint8Array,
    geometryScaling?: GeometryScaling,
    idWithinMaxSafeInteger = true,
): FeatureTable[] {
    const offset = new IntWrapper(0);
    const featureTables: FeatureTable[] = [];

    while (offset.get() < tile.length) {
        const blockLength = decodeVarintInt32Value(tile, offset) >>> 0;
        const blockStart = offset.get();
        const blockEnd = blockStart + blockLength;
        if (blockEnd > tile.length) {
            throw new Error(`Block overruns tile: ${blockEnd} > ${tile.length}`);
        }

        const tag = decodeVarintInt32Value(tile, offset) >>> 0;
        if (tag !== 1) {
            // Skip unknown block types
            offset.set(blockEnd);
            continue;
        }

        const [metadata, extent] = decodeEmbeddedTileSetMetadata(tile, offset);
        const featureTableMetadata = metadata.featureTables[0];

        let idVector: IdVector | null = null;
        let geometryVector: GeometryVector | GpuVector | null = null;
        const propertyVectors: Vector[] = [];
        let numFeatures = 0;

        for (const columnMetadata of featureTableMetadata.columns) {
            const columnName = columnMetadata.name;

            if (isLogicalIdColumn(columnMetadata)) {
                let nullabilityBuffer = null;
                // Check column metadata nullable flag, not numStreams (ID columns don't have stream count)
                if (columnMetadata.nullable) {
                    const presentStreamMetadata = decodeStreamMetadata(tile, offset);
                    const streamDataStart = offset.get();
                    const values = decodeBooleanRle(
                        tile,
                        presentStreamMetadata.numValues,
                        presentStreamMetadata.byteLength,
                        offset,
                    );
                    offset.set(streamDataStart + presentStreamMetadata.byteLength);
                    nullabilityBuffer = new BitVector(values, presentStreamMetadata.numValues);
                }

                const idDataStreamMetadata = decodeStreamMetadata(tile, offset);
                // decompressedCount is the count WITHOUT nulls, but we may have nulls
                numFeatures = nullabilityBuffer ? nullabilityBuffer.size() : idDataStreamMetadata.decompressedCount;

                idVector = decodeIdColumn(
                    tile,
                    columnMetadata,
                    offset,
                    columnName,
                    idDataStreamMetadata,
                    nullabilityBuffer ?? numFeatures,
                    idWithinMaxSafeInteger,
                );
            } else if (isGeometryColumn(columnMetadata)) {
                const numStreams = decodeVarintInt32Value(tile, offset);

                // If no ID column, get numFeatures from geometry type stream metadata
                if (numFeatures === 0) {
                    const savedOffset = offset.get();
                    const geometryTypeMetadata = decodeStreamMetadata(tile, offset);
                    numFeatures = geometryTypeMetadata.decompressedCount;
                    offset.set(savedOffset); // Reset to re-read in decodeGeometryColumn
                }

                if (geometryScaling) {
                    geometryScaling.scale = geometryScaling.extent / extent;
                }

                geometryVector = decodeGeometryColumn(tile, numStreams, offset, numFeatures, geometryScaling);
            } else {
                const columnHasStreamCount = hasStreamCount(columnMetadata);
                const numStreams = columnHasStreamCount ? decodeVarintInt32Value(tile, offset) : 1;

                if (numStreams === 0) {
                    continue;
                }

                const propertyVector = decodePropertyColumn(
                    tile,
                    offset,
                    columnMetadata,
                    numStreams,
                    numFeatures,
                    undefined,
                );
                if (propertyVector) {
                    if (Array.isArray(propertyVector)) {
                        for (const property of propertyVector) {
                            propertyVectors.push(property);
                        }
                    } else {
                        propertyVectors.push(propertyVector);
                    }
                }
            }
        }

        const featureTable = new FeatureTable(
            featureTableMetadata.name,
            geometryVector,
            idVector,
            propertyVectors,
            extent,
        );
        featureTables.push(featureTable);
        offset.set(blockEnd);
    }

    return featureTables;
}

function decodeIdColumn(
    tile: Uint8Array,
    columnMetadata: Column,
    offset: IntWrapper,
    columnName: string,
    idDataStreamMetadata: StreamMetadata,
    sizeOrNullabilityBuffer: number | BitVector,
    idWithinMaxSafeInteger = false,
): IdVector {
    const scalarTypeMetadata = columnMetadata.scalarType;
    if (
        !scalarTypeMetadata ||
        scalarTypeMetadata.type !== "logicalType" ||
        scalarTypeMetadata.logicalType !== LogicalScalarType.ID
    ) {
        throw new Error(`ID column must be a logical ID scalar type: ${columnName}`);
    }

    const idDataType = scalarTypeMetadata.longID ? ScalarType.UINT_64 : ScalarType.UINT_32;
    const nullabilityBuffer = typeof sizeOrNullabilityBuffer === "number" ? undefined : sizeOrNullabilityBuffer;

    const vectorType = getVectorType(idDataStreamMetadata, sizeOrNullabilityBuffer, tile, offset);
    if (idDataType === ScalarType.UINT_32) {
        switch (vectorType) {
            case VectorType.FLAT: {
                const id = decodeUnsignedInt32Stream(tile, offset, idDataStreamMetadata, undefined, nullabilityBuffer);
                return new Int32FlatVector(columnName, id, sizeOrNullabilityBuffer);
            }
            case VectorType.SEQUENCE: {
                const id = decodeSequenceInt32Stream(tile, offset, idDataStreamMetadata);
                return new Int32SequenceVector(
                    columnName,
                    id[0],
                    id[1],
                    (idDataStreamMetadata as RleEncodedStreamMetadata).numRleValues,
                );
            }
            case VectorType.CONST: {
                const id = decodeUnsignedConstInt32Stream(tile, offset, idDataStreamMetadata);
                return new Int32ConstVector(columnName, id, sizeOrNullabilityBuffer, false);
            }
        }
    }
    switch (vectorType) {
        case VectorType.FLAT: {
            if (idWithinMaxSafeInteger) {
                const id = decodeUnsignedInt64AsFloat64Stream(tile, offset, idDataStreamMetadata);
                return new DoubleFlatVector(columnName, id, sizeOrNullabilityBuffer);
            }
            const id = decodeUnsignedInt64Stream(tile, offset, idDataStreamMetadata, nullabilityBuffer);
            return new Int64FlatVector(columnName, id, sizeOrNullabilityBuffer);
        }
        case VectorType.SEQUENCE: {
            const id = decodeSequenceInt64Stream(tile, offset, idDataStreamMetadata);
            return new Int64SequenceVector(
                columnName,
                id[0],
                id[1],
                (idDataStreamMetadata as RleEncodedStreamMetadata).numRleValues,
            );
        }
        case VectorType.CONST: {
            const id = decodeUnsignedConstInt64Stream(tile, offset, idDataStreamMetadata);
            return new Int64ConstVector(columnName, id, sizeOrNullabilityBuffer, false);
        }
    }

    throw new Error("Vector type not supported for id column.");
}
