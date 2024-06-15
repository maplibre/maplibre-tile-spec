import { Layer } from './Layer';

export class MapLibreTile {
    layers: Layer[];

    constructor(layers: Layer[]) {
        this.layers = layers;
    }

    public toGeoJSON = () : object => {
        let fc = { "type": "FeatureCollection", "features": []};
        for (const layer of this.layers) {
            for (const feature of layer.features) {
                fc.features.push(feature.toGeoJSON());
            }
        }
        return fc;
    }
}
