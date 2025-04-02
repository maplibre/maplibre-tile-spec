import FeatureTable from "./vector/featureTable";
import {Column, ScalarColumn, ScalarType, TileSetMetadata} from "./metadata/tileset/tilesetMetadata";
import IntWrapper from "./encodings/intWrapper";
import {StreamMetadataDecoder} from "./metadata/tile/streamMetadataDecoder";
import {RleEncodedStreamMetadata} from "./metadata/tile/rleEncodedStreamMetadata";
import {VectorType} from "./vector/vectorType";
import {IntFlatVector} from "./vector/flat/intFlatVector";
import BitVector from "./vector/flat/bitVector";
import IntegerStreamDecoder from "./encodings/integerStreamDecoder";
import {IntSequenceVector} from "./vector/sequence/intSequenceVector";
import {LongFlatVector} from "./vector/flat/longFlatVector";
import {LongSequenceVector} from "./vector/sequence/longSequenceVector";
import {IntVector} from "./vector/intVector";
import {decodeVarintInt32, decodeZigZagValue} from "./encodings/integerDecodingUtils";
import {decodeGeometryColumn} from "./encodings/geometryDecoder";
import {
    decodePropertyColumn
} from "./encodings/propertyDecoder";
import {IntConstVector} from "./vector/constant/intConstVector";
import {LongConstVector} from "./vector/constant/longConstVector";
import GeometryScaling from "./encodings/geometryScaling";
import {decodeBooleanRle} from "./encodings/decodingUtils";
import {DoubleFlatVector} from "./vector/flat/doubleFlatVector";


const ID_COLUMN_NAME = "id";
const GEOMETRY_COLUMN_NAME = "geometry";

export function decodeMetadata(buffer: Uint8Array): TileSetMetadata{
    return TileSetMetadata.fromBinary(buffer);
}

export function decodeTileAndMetadata(tile: Uint8Array, tilesetMetadata: Uint8Array): FeatureTable[] {
    const metadata = TileSetMetadata.fromBinary(tilesetMetadata);
    return decodeTile(tile, metadata);

}

/**
 *  Converts the specified tile from the MLT storage into the in-memory representation for efficient processing.
 *
 * @param [featureTableDecodingOptions] Can be used to partially decode a tile by specifying which FeatureTables
 *  and/or which property columns should be decoded. If not specified the full tile will be decoded.
 * @param [geometryScaling] Specifies how the vertices of the features should be scaled.
 * @param [idWithinMaxSafeInteger] Specifies if for performance reasons the id columns with Int64 data types are
 * limited to 53 bits so withi the range of the max safe integer in js.
 */
export default function decodeTile(tile: Uint8Array, tileMetadata: TileSetMetadata,
                                   featureTableDecodingOptions?: Map<string, Set<string>>,
                                   geometryScaling?: GeometryScaling,
                                   idWithinMaxSafeInteger = true): FeatureTable[] {
    const offset = new IntWrapper(0);
    const featureTables: FeatureTable[]  =  [];

    while (offset.get() < tile.length) {
        let idVector = null;
        let geometryVector = null;
        const propertyVectors = [];

        offset.increment();
        const infos = decodeVarintInt32(tile, offset, 5);
        const version = tile[offset.get()];
        const featureTableId = infos[0];
        const featureTableBodySize = infos[1];
        const featureTableMetadata = tileMetadata.featureTables[featureTableId];

        let propertyColumnNames;
        if(featureTableDecodingOptions){
            propertyColumnNames = featureTableDecodingOptions.get(featureTableMetadata.name);
            if(!propertyColumnNames){
                offset.add(featureTableBodySize);
                continue;
            }
        }

        const extent = infos[2];
        const maxTileExtent = decodeZigZagValue(infos[3]);
        const numFeatures = infos[4];

        if (!featureTableMetadata) {
            console.error(`Could not find metadata for feature table id: ${featureTableId}`);
            return;
        }

        for (const columnMetadata of featureTableMetadata.columns) {
            const columnName = columnMetadata.name;
            const numStreams = decodeVarintInt32(tile, offset, 1)[0];

            if (columnName === ID_COLUMN_NAME) {
                let nullabilityBuffer = null;
                if (numStreams === 2) {
                    const presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                    const values = decodeBooleanRle(tile, presentStreamMetadata.numValues, offset);
                    nullabilityBuffer = new BitVector(values, presentStreamMetadata.numValues);
                }

                idVector =  decodeIdColumn(tile, columnMetadata, offset, columnName, nullabilityBuffer,
                    idWithinMaxSafeInteger);
            } else if (columnName === GEOMETRY_COLUMN_NAME) {
                if(geometryScaling){
                    geometryScaling.scale = geometryScaling.extent / extent;
                }

                geometryVector = decodeGeometryColumn(tile, numStreams, offset, numFeatures, geometryScaling);
            }
            else{
                if(numStreams === 0 && columnMetadata.type.value instanceof ScalarColumn){
                    continue;
                }

                const propertyVector = decodePropertyColumn(tile, offset, columnMetadata,
                    numStreams, numFeatures, propertyColumnNames);
                if(propertyVector){
                    Array.isArray(propertyVector)?
                        propertyVectors.push(...propertyVector) :
                        propertyVectors.push(propertyVector);
                }
            }
        }

        const featureTable = new FeatureTable(featureTableMetadata.name, geometryVector, idVector,
            propertyVectors, extent);
        featureTables.push(featureTable);
    }

    return featureTables;
}

function decodeIdColumn(
    tile: Uint8Array,
    columnMetadata: Column,
    offset: IntWrapper,
    columnName: string,
    nullabilityBuffer: BitVector,
    idWithinMaxSafeInteger: boolean = false
): IntVector {
    const idDataStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
    const idDataType = (columnMetadata.type.value as ScalarColumn).type.value as ScalarType;
    const vectorType = IntegerStreamDecoder.getVectorTypeIntStream(idDataStreamMetadata);
    if (idDataType === ScalarType.UINT_32) {
        switch (vectorType) {
            case VectorType.FLAT:{
                const id = IntegerStreamDecoder.decodeIntStream(tile, offset, idDataStreamMetadata, false);
                return new IntFlatVector(columnName, id, nullabilityBuffer);
            }
            case VectorType.SEQUENCE:{
                const id = IntegerStreamDecoder.decodeSequenceIntStream(tile, offset, idDataStreamMetadata);
                return new IntSequenceVector(columnName, id[0], id[1], (idDataStreamMetadata as RleEncodedStreamMetadata).numRleValues);
            }
            case VectorType.CONST:{
                const id = IntegerStreamDecoder.decodeConstIntStream(tile, offset, idDataStreamMetadata, false);
                return new IntConstVector(columnName, id, nullabilityBuffer);
            }

        }
    } else {
        switch (vectorType) {
            case VectorType.FLAT:{
                if(idWithinMaxSafeInteger){
                    const id =  IntegerStreamDecoder.decodeLongFloat64Stream(tile, offset, idDataStreamMetadata, false);
                    return new DoubleFlatVector(columnName, id, nullabilityBuffer);
                }

                const id = IntegerStreamDecoder.decodeLongStream(tile, offset, idDataStreamMetadata, false);
                return new LongFlatVector(columnName, id, nullabilityBuffer);
            }
            case VectorType.SEQUENCE:{
                const id = IntegerStreamDecoder.decodeSequenceLongStream(tile, offset, idDataStreamMetadata);
                return new LongSequenceVector(columnName, id[0], id[1], (idDataStreamMetadata as RleEncodedStreamMetadata).numRleValues);
            }
            case VectorType.CONST:{
                const id = IntegerStreamDecoder.decodeConstLongStream(tile, offset, idDataStreamMetadata, false);
                return new LongConstVector(columnName, id, nullabilityBuffer);
            }
        }
    }

    throw new Error("Vector type not supported for id column.");
}

/*
export function decodeTileSequential(tile: Uint8Array, tileMetadata: TileSetMetadata): FeatureTable[] {
    const offset = new IntWrapper(0);
    const featureTables: FeatureTable[]  =  [];

    while (offset.get() < tile.length) {
        let idVector = null;
        let geometryVector = null;
        const propertyVectors = [];

        offset.increment();
        const infos = decodeVarint(tile, offset, 5);
        const version = tile[offset.get()];
        const featureTableId = infos[0];
        const featureTableBodySize = infos[1];
        const extent = infos[2];
        const maxTileExtent = DecodingUtils.decodeZigZag(infos[3]);
        const numFeatures = infos[4];
        const featureTableMetadata = tileMetadata.featureTables[featureTableId];

        if (!featureTableMetadata) {
            console.log(`could not find metadata for feature table id: ${featureTableId}`);
            return;
        }

        for (const columnMetadata of featureTableMetadata.columns) {
            const columnName = columnMetadata.name;
            const numStreams = decodeVarint(tile, offset, 1)[0];

            if (columnName === ID_COLUMN_NAME) {
                let nullabilityBuffer = null;
                if (numStreams == 2) {
                    const presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                    const values = DecodingUtils.decodeBooleanRle(tile, presentStreamMetadata.numValues, offset);
                    nullabilityBuffer = new BitVector(values, presentStreamMetadata.numValues);
                }

                idVector = decodeIdColumn(tile, columnMetadata, offset, columnName, nullabilityBuffer);
            } else if (columnName === GEOMETRY_COLUMN_NAME) {
                geometryVector = decodeGeometryColumnSequential(tile, numStreams, offset, numFeatures);
            }
            else{
                const propertyVector = decodePropertyColumnSequential(tile, offset, columnMetadata, numStreams,numFeatures);
                Array.isArray(propertyVector)? propertyVectors.push(...propertyVector) : propertyVectors.push(propertyVector);
            }
        }

        const featureTable = new FeatureTable(featureTableMetadata.name, geometryVector, idVector, propertyVectors, extent);
        featureTables.push(featureTable);
    }

    return featureTables;
}

export function decodeMlTileGeometrySequential(tile: Uint8Array, tileMetadata: TileSetMetadata): FeatureTable[] {
    const offset = new IntWrapper(0);
    const featureTables: FeatureTable[]  =  [];

    while (offset.get() < tile.length) {
        let idVector = null;
        let geometryVector = null;
        const propertyVectors = [];

        offset.increment();
        const infos = decodeVarint(tile, offset, 5);
        const version = tile[offset.get()];
        const featureTableId = infos[0];
        const featureTableBodySize = infos[1];
        const extent = infos[2];
        const maxTileExtent = DecodingUtils.decodeZigZag(infos[3]);
        const numFeatures = infos[4];

        const featureTableMetadata = tileMetadata.featureTables[featureTableId];

        if (!featureTableMetadata) {
            console.log(`could not find metadata for feature table id: ${featureTableId}`);
            return;
        }

        for (const columnMetadata of featureTableMetadata.columns) {
            const columnName = columnMetadata.name;
            const numStreams = decodeVarint(tile, offset, 1)[0];

            if (columnName === ID_COLUMN_NAME) {
                let nullabilityBuffer = null;
                if (numStreams == 2) {
                    const presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                    const values = DecodingUtils.decodeBooleanRle(tile, presentStreamMetadata.numValues, offset);
                    nullabilityBuffer = new BitVector(values, presentStreamMetadata.numValues);
                }

                idVector = decodeIdColumn(tile, columnMetadata, offset, columnName, nullabilityBuffer);
            } else if (columnName === GEOMETRY_COLUMN_NAME) {
                geometryVector = decodeGeometryColumnSequential(tile, numStreams, offset, numFeatures);
            }
            else{
                const propertyVector = decodePropertyColumn(tile, offset, columnMetadata, numStreams,numFeatures);
                Array.isArray(propertyVector)? propertyVectors.push(...propertyVector) : propertyVectors.push(propertyVector);
            }
        }

        const featureTable = new FeatureTable(featureTableMetadata.name, geometryVector, idVector, propertyVectors, extent);
        featureTables.push(featureTable);
    }

    return featureTables;
}
*/
