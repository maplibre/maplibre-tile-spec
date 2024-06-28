import { Layer } from '../data/Layer';
import { VectorTileFeature } from './VectorTileFeature';

class VectorTileLayer {
  version: number;
  name: string | null;
  extent: number;
  length: number = 0;

  _raw: Layer;

  constructor(layer: Layer) {
    this.name = layer.name;
    this._raw = layer;
    this.length = layer.features.length;
  }

  feature(i: number): VectorTileFeature {
    return new VectorTileFeature(this._raw.features[i]);
  }
}

export { VectorTileLayer };
