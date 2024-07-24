import { MltDecoder } from '../../src/decoder/MltDecoder';
import { VectorTileLayer } from './VectorTileLayer';

class VectorTile {
  layers: { [_: string]: VectorTileLayer } = {};

  constructor(data: Uint8Array, featureTables: any) {
    const decoded = MltDecoder.decodeMlTile(data, featureTables);

    for (const layerName of Object.keys(decoded.layers)) {
      this.layers[layerName] = new VectorTileLayer(decoded.layers[layerName]);
    }
  }
}

export { VectorTile };
