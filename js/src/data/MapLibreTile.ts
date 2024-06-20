import { Layer } from './Layer';

export class MapLibreTile {
    layers: Layer[];

    constructor(layers: Layer[]) {
        this.layers = layers;
    }
}
