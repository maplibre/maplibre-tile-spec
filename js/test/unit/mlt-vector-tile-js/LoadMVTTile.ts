import * as fs from 'fs';

import { VectorTile } from '@mapbox/vector-tile';
import Protobuf from 'pbf';

const loadTile = (tilePath) => {
  const data : Buffer = fs.readFileSync(tilePath);
  return new VectorTile(new Protobuf(data));
};

export default loadTile;
