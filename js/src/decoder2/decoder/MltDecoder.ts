import { Feature } from '../data/Feature';
import { Layer } from '../data/Layer';
import { MapLibreTile } from '../data/MapLibreTile';
import { PhysicalLevelTechnique } from '../metadata/stream/PhysicalLevelTechnique';
import { StreamMetadataDecoder } from '../metadata/stream/StreamMetadataDecoder';
import { FeatureTableSchema, TileSetMetadata } from "../../../src/decoder/mlt_tileset_metadata_pb";
import { IntWrapper } from './IntWrapper';
import { DecodingUtils } from './DecodingUtils';
import { IntegerDecoder } from './IntegerDecoder';
import { GeometryDecoder } from './GeometryDecoder';
import { PropertyDecoder } from './PropertyDecoder';

class MltDecoder {
    private static ID_COLUMN_NAME = "id";
    private static GEOMETRY_COLUMN_NAME = "geometry";

    public static decodeMlTile(tile: Uint8Array, tileMetadata: TileSetMetadata): MapLibreTile {
        const offset = new IntWrapper(0);
        const mltLayers: Layer[] = [];
        while (offset.get() < tile.length) {
            let ids = [];
            const geometries = [];
            const properties = {};

            const version = tile[offset.get()];
            offset.increment();
            const infos = DecodingUtils.decodeVarint(tile, offset, 4);
            const featureTableId = infos[0];
            const tileExtent = infos[1];
            const maxTileExtent = infos[2];
            const numFeatures = infos[3];

            const metadata = tileMetadata.featureTables[featureTableId];
            for (const columnMetadata of metadata.columns) {
                const columnName = columnMetadata.name;
                const numStreams = DecodingUtils.decodeVarint(tile, offset, 1)[0];
                if (columnName === "id") {
                    if (numStreams === 2) {
                        const presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                        const presentStream = DecodingUtils.decodeBooleanRle(tile, presentStreamMetadata.numValues(), presentStreamMetadata.byteLength(), offset);
                    }

                    const idDataStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                    ids = idDataStreamMetadata.physicalLevelTechnique() === PhysicalLevelTechnique.FAST_PFOR
                        ? IntegerDecoder.decodeIntStream(tile, offset, idDataStreamMetadata, false).map(i => i as number)
                        : IntegerDecoder.decodeLongStream(tile, offset, idDataStreamMetadata, false);

                } else if (columnName === "geometry") {
                    const geometryColumn = GeometryDecoder.decodeGeometryColumn(tile, numStreams, offset);
                    // TODO
                    // geometries = GeometryDecoder.decodeGeometry(geometryColumn);
                } else {
                    const propertyColumn = PropertyDecoder.decodePropertyColumn(tile, offset, columnMetadata, numStreams);
                    if (propertyColumn instanceof Map) {
                        const p = propertyColumn as Map<string, any>;
                        for (const [key, value] of p.entries()) {
                            properties[key] = value;
                        }
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
        const features: Feature[] = new Array(numFeatures);
        for (let j = 0; j < numFeatures; j++) {
            const p: { [key: string]: any } = {};
            for (const [key, value] of Object.entries(properties)) {
                p[key] = value ? value[j] : null;
            }
            const feature = new Feature(ids[j], geometries[j], p);
            features[j] = feature;
        }

        return new Layer(metadata.name, features);
    }
}

export { MltDecoder };
