#!/usr/bin/env node

import { VectorTile } from '@mapbox/vector-tile';
import Protobuf from 'pbf';
import { parseArgs, bench, toKb } from './utils';
import zlib from 'zlib';
import { readFile } from 'node:fs/promises';

const main = async (gzip: boolean) => {
  const { tilePath, iterations } = await parseArgs(process.argv.slice(2));
  const data = await readFile(tilePath)
  if (gzip) {
    const compressed = zlib.gzipSync(data);
    const decoder = async () => {
      return new Promise((resolve, reject) => {
        zlib.gunzip(compressed, (err, data) => {
          if (err) {
            return reject(err);
          }
          return resolve(new VectorTile(new Protobuf(data)));
        });
      });
    };
    const size = toKb(compressed.length);
    await bench(`MVT+gzip (${size}kb) 🍎`, decoder, iterations);
  } else {
    const decoder = async () => {
      return new Promise((resolve) => {
          return resolve(new VectorTile(new Protobuf(data)));
      });
    };
    const size = toKb(data.length);
    await bench(`MVT+raw (${size}kb) 🍐`, decoder, iterations);
  }
}

void async function go() {
  await main(true);
  await main(false);
}()
