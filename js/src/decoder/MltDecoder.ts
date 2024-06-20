import { Feature } from '../data/Feature';
import { Layer } from '../data/Layer';
import { MapLibreTile } from '../data/MapLibreTile';
import { PhysicalLevelTechnique } from '../metadata/stream/PhysicalLevelTechnique';
import { StreamMetadataDecoder } from '../metadata/stream/StreamMetadataDecoder';
import { FeatureTableSchema, TileSetMetadata } from "../metadata/mlt_tileset_metadata_pb";
import { IntWrapper } from './IntWrapper';
import { DecodingUtils } from './DecodingUtils';
import { IntegerDecoder } from './IntegerDecoder';
import { GeometryDecoder } from './GeometryDecoder';
import { PropertyDecoder } from './PropertyDecoder';
import { ScalarType } from "../metadata/mlt_tileset_metadata_pb";

class MltDecoder {
    private static ID_COLUMN_NAME = "id";
    private static GEOMETRY_COLUMN_NAME = "geometry";

    public static decodeMlTile(tile: Uint8Array, tileMetadata: TileSetMetadata): MapLibreTile {
        const offset = new IntWrapper(0);
        const mltLayers: Layer[] = [];
        while (offset.get() < tile.length) {
            let ids = [];
            let geometries = [];
            const properties = {};

            offset.increment();
            const infos = DecodingUtils.decodeVarint(tile, offset, 4);
            // TODO: keep these unused variables for now to match Java code
            /* eslint-disable @typescript-eslint/no-unused-vars */
            const version = tile[offset.get()];
            const tileExtent = infos[1];
            const maxTileExtent = infos[2];
            const featureTableId = infos[0];
            const numFeatures = infos[3];
            const metadata = tileMetadata.featureTables[featureTableId];
            if (!metadata) {
                console.log(`could not find metadata for feature table id: ${featureTableId}`);
                return;
            }
            for (const columnMetadata of metadata.columns) {
                const columnName = columnMetadata.name;
                const numStreams = DecodingUtils.decodeVarint(tile, offset, 1)[0];
                if (columnName === "id") {
                    if (numStreams === 2) {
                        const presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                        const presentStream = DecodingUtils.decodeBooleanRle(tile, presentStreamMetadata.numValues(), offset);
                    } else {
                        throw new Error("Unsupported number of streams for ID column: " + numStreams);
                    }
                    const idDataStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                    const physicalType = columnMetadata.type.value.type.value;
                    if (physicalType === ScalarType.UINT_32) {
                        ids = IntegerDecoder.decodeIntStream(tile, offset, idDataStreamMetadata, false);
                    } else if (physicalType === ScalarType.UINT_64){
                        ids = IntegerDecoder.decodeLongStream(tile, offset, idDataStreamMetadata, false);
                    } else {
                        throw new Error("Unsupported ID column type: " + physicalType);
                    }
                } else if (columnName === "geometry") {
                    const geometryColumn = GeometryDecoder.decodeGeometryColumn(tile, numStreams, offset);
                    geometries = GeometryDecoder.decodeGeometry(geometryColumn);
                } else {
                    const propertyColumn = PropertyDecoder.decodePropertyColumn(tile, offset, columnMetadata, numStreams);
                    if (propertyColumn instanceof Map) {
                        throw new Error("Nested properties are not implemented yet");
                        // for (const [key, value] of propertyColumn.entries()) {
                        //     properties[key] = value;
                        // }
                    } else {
                        properties[columnName] = propertyColumn;
                    }
                }
            }

            const layer = MltDecoder.convertToLayer(ids, geometries, properties, metadata, numFeatures);
            mltLayers.push(layer);
        }

        return new MapLibreTile(mltLayers);
    }

    private static convertToLayer(ids: number[], geometries, properties, metadata: FeatureTableSchema, numFeatures: number): Layer {
        // if (numFeatures != geometries.length || numFeatures != ids.length) {
        //     console.log(
        //         "Warning, in convertToLayer the size of ids("
        //             + ids.length
        //             + "), geometries("
        //             + geometries.length
        //             + "), and features("
        //             + numFeatures
        //             + ") are not equal for layer: "
        //             + metadata.name);
        // }
        const features: Feature[] = new Array(numFeatures);
        const vals = Object.entries(properties);
        for (let j = 0; j < numFeatures; j++) {
            /* eslint-disable @typescript-eslint/no-explicit-any */
            const p: { [key: string]: any } = {};
            for (const [key, value] of vals) {
                p[key] = value ? value[j] : null;
            }
            const feature = new Feature(ids[j], geometries[j], p);
            features[j] = feature;
        }

        return new Layer(metadata.name, features);
    }
}

export { MltDecoder };
