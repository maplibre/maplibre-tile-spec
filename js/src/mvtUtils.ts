import { VectorTile } from '@mapbox/vector-tile';
import Protobuf from 'pbf';

export function parseMvtTile(mvtTile: Buffer): any {
    const vectorTile = new VectorTile(new Protobuf(mvtTile));
    const layers = [];
    for (const layerName of Object.keys(vectorTile.layers)) {
        const layer = vectorTile.layers[layerName];
        const features = [];
        layers.push({ name: layerName, features });
        for (let i = 0; i < layer.length; i++) {
            features.push(layer.feature(i));
        }
    }
    return {
        layers: layers
    }
}
