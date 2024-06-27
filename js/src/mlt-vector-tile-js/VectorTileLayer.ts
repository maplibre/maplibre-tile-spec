import { Layer } from '../data/Layer';
import { VectorTileFeature } from './VectorTileFeature';

class VectorTileLayer {
  version: number;
  name: string | null;
  extent: number;
  length: number = 0;

  _raw: any;
  _keys: string[];
  _values: any[];
  _features: VectorTileFeature[] = [];

  constructor(layer: Layer) {
    this.name = layer.name;
    this._features = layer.features.map((feature) => new VectorTileFeature(feature));
    this.length = this._features.length;
  }

  feature(i: number): VectorTileFeature {
    return this._features[i];
  }
}

export { VectorTileLayer };
