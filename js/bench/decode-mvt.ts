#!/usr/bin/env node

import { VectorTile } from '@mapbox/vector-tile';
import Protobuf from 'pbf';
import { parseArgs, bench } from './utils';
import { readFileSync } from 'fs';


function main() {
  const { tilePath, iterations } = parseArgs(process.argv.slice(2));
  const mvtTile = readFileSync(tilePath);
  const decoder = () => {
    return new VectorTile(new Protobuf(mvtTile));
  }

  bench('mvt', decoder, iterations);
}

main();
