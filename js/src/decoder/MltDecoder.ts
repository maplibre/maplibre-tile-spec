import { Feature } from '../data/Feature';
import { Layer } from '../data/Layer';
import { MapLibreTile } from '../data/MapLibreTile';
import { PhysicalLevelTechnique } from '../metadata/stream/PhysicalLevelTechnique';
import { StreamMetadataDecoder } from '../metadata/stream/StreamMetadataDecoder';
import { IntWrapper } from './IntWrapper';
import { DecodingUtils } from './DecodingUtils';
import { IntegerDecoder } from './IntegerDecoder';
import { GeometryDecoder } from './GeometryDecoder';
import { PropertyDecoder } from './PropertyDecoder';

class MltDecoder {
    private static ID_COLUMN_NAME = "id";
    private static GEOMETRY_COLUMN_NAME = "geometry";

    public static generateFeatureTables(tilesetMetadata: any): any {
        const featureTables = [];
        for (let i=0; i<tilesetMetadata.featureTables.length; i++) {
          const featureTable = tilesetMetadata.featureTables[i];
          const table = {"name": featureTable.name};
          const columns = [];
          featureTable.columns.forEach((column) => {
            // https://github.com/bufbuild/protobuf-es/blob/main/docs/runtime_api.md#accessing-oneof-groups
            if (column.name === "geometry" || column.name === "id") {
                columns.push({
                  "name": column.name
                });
              } else {
                columns.push({
                  "name": column.name,
                  "type": column.type.value.type.value
                });
            }
            table['columns'] = columns;
            featureTables[i] = table;
          });
        }
        return featureTables;
    }

    public static decodeMlTile(tile: Uint8Array, featureTables: any): MapLibreTile {
        const offset = new IntWrapper(0);
        const mltLayers: Layer[] = [];
        while (offset.get() < tile.length) {
            let ids = [];
            let geometryTypes = [];
            let geometryColumn = null;
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
            // Optimize metadata usage
            const featureTableMeta = featureTables[featureTableId];
            if (!featureTableMeta) {
                console.log(`could not find metadata for feature table id: ${featureTableId}`);
                return;
            }
            for (const columnMetadata of featureTableMeta.columns) {
                const columnName = columnMetadata.name;
                const numStreams = DecodingUtils.decodeVarint(tile, offset, 1)[0];
                if (columnName === "id") {
                    if (numStreams === 2) {
                        const presentStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                        const presentStream = DecodingUtils.decodeBooleanRle(tile, presentStreamMetadata.numValues(), presentStreamMetadata.byteLength(), offset);
                    }
                    // TODO: handle switching on physicalType
                    // const physicalType = columnMetadata.type;

                    const idDataStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                    ids = idDataStreamMetadata.physicalLevelTechnique() === PhysicalLevelTechnique.FAST_PFOR
                        ? IntegerDecoder.decodeIntStream(tile, offset, idDataStreamMetadata, false).map(i => i as number)
                        : IntegerDecoder.decodeLongStream(tile, offset, idDataStreamMetadata, false);

                } else if (columnName === "geometry") {
                    const geometryTypeMetadata = StreamMetadataDecoder.decode(tile, offset);
                    geometryTypes = IntegerDecoder.decodeIntStream(tile, offset, geometryTypeMetadata, false);
                    geometryColumn = GeometryDecoder.decodeGeometryColumn(tile, numStreams, offset);
                } else {
                    const propertyColumn = PropertyDecoder.decodePropertyColumn(tile, offset, columnMetadata.type, numStreams);
                    if (propertyColumn instanceof Map) {
                        for (const [key, value] of propertyColumn.entries()) {
                            properties[key] = value;
                        }
                    } else {
                        properties[columnName] = propertyColumn;
                    }
                }
            }

            const layer = MltDecoder.convertToLayer(ids, geometryTypes, geometryColumn, properties, featureTableMeta.name, numFeatures);
            mltLayers.push(layer);
        }

        return new MapLibreTile(mltLayers);
    }

    private static convertToLayer(ids: number[], geometryTypes, geometryColumn, properties, name: string, numFeatures: number): Layer {
        // if (numFeatures != geometries.length || numFeatures != ids.length) {
        //     console.log(
        //         "Warning, in convertToLayer the size of ids("
        //             + ids.length
        //             + "), geometries("
        //             + geometries.length
        //             + "), and features("
        //             + numFeatures
        //             + ") are not equal for layer: "
        //             + name);
        // }
        const features: Feature[] = new Array(numFeatures);
        const vals = Object.entries(properties);
        const geometries = GeometryDecoder.decodeGeometries(geometryTypes, geometryColumn);
        for (let j = 0; j < numFeatures; j++) {
            /* eslint-disable @typescript-eslint/no-explicit-any */
            const p: { [key: string]: any } = {};
            for (const [key, value] of vals) {
                p[key] = value ? value[j] : null;
            }
            const feature = new Feature(ids[j], geometries[j], p);
            features[j] = feature;
        }

        return new Layer(name, features);
    }
}

export { MltDecoder };
