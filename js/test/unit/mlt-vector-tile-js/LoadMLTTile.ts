import * as fs from 'fs';

import { MltDecoder, TileSetMetadata } from '../../../src/index';
import * as vt from '../../../src/mlt-vector-tile-js/index';

const loadTile = (tilePath, metadataPath) : vt.VectorTile => {
  const data : Buffer = fs.readFileSync(tilePath);
  const metadata : Buffer = fs.readFileSync(metadataPath);
  const tilesetMetadata = TileSetMetadata.fromBinary(metadata);
  const tile : vt.VectorTile = new vt.VectorTile(data, tilesetMetadata);
  return tile;
};

export default loadTile;
