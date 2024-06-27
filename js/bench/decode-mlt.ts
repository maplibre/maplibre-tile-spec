#!/usr/bin/env node

import { MltDecoder, TileSetMetadata } from '../src/index';
import { parseArgs, bench } from './utils';
import { readFileSync } from 'fs';


function main() {
  const { tilePath, iterations } = parseArgs(process.argv.slice(2));
  const mltMetadataPbf = readFileSync(tilePath.replace('.advanced','') + ".meta.pbf");
  const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
  const mltTile = readFileSync(tilePath);
  const decoder = () => {
    return MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
  }

  bench('mlt', decoder, iterations);
}

main();
