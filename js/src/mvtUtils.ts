import { VectorTile } from '@mapbox/vector-tile';
import Protobuf from 'pbf';

export interface MVTFeature {
    id: number;
    geometry: { x: number; y: number }[][];
    properties: Map<string, unknown>;
}

export interface MVTLayer {
    name: string;
    features: MVTFeature[];
}

export function parseMvtTile(mvtTile: Buffer): MVTLayer[] {
    const vectorTile = new VectorTile(new Protobuf(mvtTile));

    const layers = [];
    for (const layerName of Object.keys(vectorTile.layers)) {
        const layer = vectorTile.layers[layerName];
        const features = [];
        layers.push({ name: layerName, features });
        for (let i = 0; i < layer.length; i++) {
            const feature = layer.feature(i);
            const id = feature.id;
            const geometry = feature.loadGeometry();

            const properties = new Map<string, unknown>();
            for (const [name, value] of Object.entries(feature.properties)) {
                properties.set(name, value);
            }

            features.push({ id, geometry, properties });
        }
    }

    return layers;
}

export function parseMvtTileFast(mvtTile: Buffer): MVTLayer[] {
    const vectorTile = new VectorTile(new Protobuf(mvtTile));

    for (const layerName of Object.keys(vectorTile.layers)) {
        const layer = vectorTile.layers[layerName];
        for (let i = 0; i < layer.length; i++) {
            layer.feature(i).loadGeometry();
        }
    }

    return [];
}
