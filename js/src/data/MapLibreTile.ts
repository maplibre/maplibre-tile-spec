import { Layer } from './Layer';

function printValue(key, value) {
    return typeof value === 'bigint'
    ? value.toString()
    : value;
}

export class MapLibreTile {
    layers: Layer[];

    constructor(layers: Layer[]) {
        this.layers = layers;
    }

    public toGeoJSON = () : string => {
        let fc = { "type": "FeatureCollection", "features": []};
        for (const layer of this.layers) {
            for (const feature of layer.features) {
                fc.features.push(feature.toGeoJSON());
            }
        }
        return JSON.stringify(fc, printValue, 1);
    }
}
