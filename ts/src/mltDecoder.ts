import FeatureTable from "./vector/featureTable";
import { type Column, ScalarType } from "./metadata/tileset/tilesetMetadata";
import IntWrapper from "./encodings/intWrapper";
import { StreamMetadataDecoder } from "./metadata/tile/streamMetadataDecoder";
import { type RleEncodedStreamMetadata } from "./metadata/tile/rleEncodedStreamMetadata";
import { VectorType } from "./vector/vectorType";
import { IntFlatVector } from "./vector/flat/intFlatVector";
import BitVector from "./vector/flat/bitVector";
import IntegerStreamDecoder from "./encodings/integerStreamDecoder";
import { IntSequenceVector } from "./vector/sequence/intSequenceVector";
import { LongFlatVector } from "./vector/flat/longFlatVector";
import { LongSequenceVector } from "./vector/sequence/longSequenceVector";
import { type IntVector } from "./vector/intVector";
import { decodeVarintInt32 } from "./encodings/integerDecodingUtils";
import { decodeGeometryColumn } from "./encodings/geometryDecoder";
import { decodePropertyColumn } from "./encodings/propertyDecoder";
import { IntConstVector } from "./vector/constant/intConstVector";
import { LongConstVector } from "./vector/constant/longConstVector";
import type GeometryScaling from "./encodings/geometryScaling";
import { decodeBooleanRle } from "./encodings/decodingUtils";
import { DoubleFlatVector } from "./vector/flat/doubleFlatVector";
import { decodeEmbeddedTileSetMetadata } from "./metadata/tileset/embeddedTilesetMetadataDecoder";
import { TypeMap } from "./metadata/tileset/typeMap";
import { type StreamMetadata } from "./metadata/tile/streamMetadata";
import { type GeometryVector } from "./vector/geometry/geometryVector";
import type Vector from "./vector/vector";
import { type GpuVector } from "./vector/geometry/gpuVector";

const ID_COLUMN_NAME = "id";
const GEOMETRY_COLUMN_NAME = "geometry";

/**
 * Decodes a tile with embedded metadata (Tag 0x01 format).
 * This is the primary decoder function for MLT tiles.
 *
 * @param tile The tile data to decode
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
        const blockLength = decodeVarintInt32(tile, offset, 1)[0] >>> 0;
        const blockStart = offset.get();
        const blockEnd = blockStart + blockLength;
        if (blockEnd > tile.length) {
            throw new Error(`Block overruns tile: ${blockEnd} > ${tile.length}`);
        }

        const tag = decodeVarintInt32(tile, offset, 1)[0] >>> 0;
        if (tag !== 1) {
            // Skip unknown block types
            offset.set(blockEnd);
            continue;
        }

        // Decode embedded metadata and extent (one of each per block)
        const decode = decodeEmbeddedTileSetMetadata(tile, offset);
        const metadata = decode[0];
        const extent = decode[1];
        const featureTableMetadata = metadata.featureTables[0];

        // Decode columns from streams
        let idVector: IntVector | null = null;
        let geometryVector: GeometryVector | GpuVector | null = null;
        const propertyVectors: Vector[] = [];
        let numFeatures = 0;

        for (const columnMetadata of featureTableMetadata.columns) {
            const columnName = columnMetadata.name;
            const numStreams = TypeMap.hasStreamCount(columnMetadata) ? decodeVarintInt32(tile, offset, 1)[0] : 1;

            if (columnName === ID_COLUMN_NAME) {
                let nullabilityBuffer = null;
                // Check column metadata nullable flag, not numStreams (ID columns don't have stream count)
                if (columnMetadata.nullable) {
                    const presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                    const streamDataStart = offset.get();
                    const values = decodeBooleanRle(tile, presentStreamMetadata.numValues, offset);
                    // Fix offset: decodeBooleanRle doesn't consume all compressed bytes
                    offset.set(streamDataStart + presentStreamMetadata.byteLength);
                    nullabilityBuffer = new BitVector(values, presentStreamMetadata.numValues);
                }

                const idDataStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                numFeatures = idDataStreamMetadata.getDecompressedCount();

                idVector = decodeIdColumn(
                    tile,
                    columnMetadata,
                    offset,
                    columnName,
                    idDataStreamMetadata,
                    nullabilityBuffer ?? numFeatures,
                    idWithinMaxSafeInteger,
                );
            } else if (columnName === GEOMETRY_COLUMN_NAME) {
                if (geometryScaling) {
                    geometryScaling.scale = geometryScaling.extent / extent;
                }

                geometryVector = decodeGeometryColumn(tile, numStreams, offset, numFeatures, geometryScaling);
            } else {
                if (numStreams === 0 && columnMetadata.type === "scalarType") {
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
                        propertyVectors.push(...propertyVector);
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
    idWithinMaxSafeInteger: boolean = false,
): IntVector {
    const idDataType = columnMetadata.scalarType.physicalType;
    const vectorType = IntegerStreamDecoder.getVectorType(idDataStreamMetadata, sizeOrNullabilityBuffer, tile, offset);
    if (idDataType === ScalarType.UINT_32) {
        switch (vectorType) {
            case VectorType.FLAT: {
                const id = IntegerStreamDecoder.decodeIntStream(tile, offset, idDataStreamMetadata, false);
                return new IntFlatVector(columnName, id, sizeOrNullabilityBuffer);
            }
            case VectorType.SEQUENCE: {
                const id = IntegerStreamDecoder.decodeSequenceIntStream(tile, offset, idDataStreamMetadata);
                return new IntSequenceVector(
                    columnName,
                    id[0],
                    id[1],
                    (idDataStreamMetadata as RleEncodedStreamMetadata).numRleValues,
                );
            }
            case VectorType.CONST: {
                const id = IntegerStreamDecoder.decodeConstIntStream(tile, offset, idDataStreamMetadata, false);
                return new IntConstVector(columnName, id, sizeOrNullabilityBuffer);
            }
        }
    } else {
        switch (vectorType) {
            case VectorType.FLAT: {
                if (idWithinMaxSafeInteger) {
                    const id = IntegerStreamDecoder.decodeLongFloat64Stream(tile, offset, idDataStreamMetadata, false);
                    return new DoubleFlatVector(columnName, id, sizeOrNullabilityBuffer);
                }

                const id = IntegerStreamDecoder.decodeLongStream(tile, offset, idDataStreamMetadata, false);
                return new LongFlatVector(columnName, id, sizeOrNullabilityBuffer);
            }
            case VectorType.SEQUENCE: {
                const id = IntegerStreamDecoder.decodeSequenceLongStream(tile, offset, idDataStreamMetadata);
                return new LongSequenceVector(
                    columnName,
                    id[0],
                    id[1],
                    (idDataStreamMetadata as RleEncodedStreamMetadata).numRleValues,
                );
            }
            case VectorType.CONST: {
                const id = IntegerStreamDecoder.decodeConstLongStream(tile, offset, idDataStreamMetadata, false);
                return new LongConstVector(columnName, id, sizeOrNullabilityBuffer);
            }
        }
    }

    throw new Error("Vector type not supported for id column.");
}
