#!/usr/bin/env node

import { MltDecoder, TileSetMetadata } from '../src/index';
import { parseArgs, bench, toKb } from './utils';
import zlib from 'zlib';
import { readFile } from 'node:fs/promises';

const main = async (gzip: boolean) => {
  const { tilePath, iterations } = await parseArgs(process.argv.slice(2));
  const data = await readFile(tilePath)
  const mltMetadataPbf = await readFile(tilePath.replace('.advanced','') + ".meta.pbf");
  const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
  if (gzip) {
    const compressed = zlib.gzipSync(data);
    const decoder = async () => {
      return new Promise((resolve, reject) => {
        zlib.gunzip(compressed, (err, data) => {
          if (err) {
            return reject(err);
          }
          return resolve(MltDecoder.decodeMlTile(data, tilesetMetadata));
        });
      });
    };
    const size = toKb(compressed.length);
    await bench(`MLT+gzip (${size}kb) 🍊`, decoder, iterations);
  } else {
    const decoder = async () => {
      return new Promise((resolve) => {
          return resolve(MltDecoder.decodeMlTile(data, tilesetMetadata));
      });
    };
    const size = toKb(data.length);
    await bench(`MLT+raw (${size}kb) 🍎`, decoder, iterations);
  }

}


void async function go() {
  await main(true);
  await main(false);
}()

