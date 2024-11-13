#!/usr/bin/env node

import { VectorTile } from '@mapbox/vector-tile';
import Protobuf from 'pbf';
import { parseArgs, bench, stats, toKb } from './utils';
import zlib from 'zlib';
import { readFile } from 'node:fs/promises';
import { basename } from 'node:path';

const main = async (data: Buffer, decoder: () => Promise<VectorTile>, iterations: number, gzip: boolean) => {
  if (gzip) {
    const compressed = zlib.gzipSync(data);
    const gzipDecoder = async () => {
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
    await bench(`🍎 MVT+gzip (${size}kb)`, gzipDecoder, false, iterations);
    await bench(`🍏 MVT+gzip+earcut`, gzipDecoder, true, iterations);
  } else {
    const size = toKb(data.length);
    await bench(`🍐 MVT+raw (${size}kb)`, decoder, false, iterations);
  }
}

void async function go() {
  const { tilePath, iterations } = await parseArgs(process.argv.slice(2));
  const data = await readFile(tilePath);
  const decoder = async () => {
    return new Promise((resolve) => {
      return resolve(new VectorTile(new Protobuf(data)));
    });
  };
  await stats(basename(tilePath), decoder);
  await main(data, decoder, iterations, true);
  await main(data, decoder, iterations, false);
}()
