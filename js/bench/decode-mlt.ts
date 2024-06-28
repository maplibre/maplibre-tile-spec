#!/usr/bin/env node

import { MltDecoder, TileSetMetadata } from '../src/index';
import { parseArgs, bench, stats, toKb } from './utils';
import zlib from 'zlib';
import { readFile } from 'node:fs/promises';
import { basename } from 'node:path';

const main = async (data: Buffer, decoder, tilesetMetadata: TileSetMetadata,  iterations: number, gzip: boolean) => {
  if (gzip) {
    const compressed = zlib.gzipSync(data);
    const gzipDecoder = async () => {
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
    await bench(`🍊 MLT+gzip (${size}kb)`, gzipDecoder, false, iterations);
  } else {
    const size = toKb(data.length);
    await bench(`🍎 MLT+raw (${size}kb)`, decoder, false, iterations);
  }

}


void async function go() {
  const { tilePath, iterations } = await parseArgs(process.argv.slice(2));
  const data = await readFile(tilePath);
  const mltMetadataPbf = await readFile(tilePath.replace('.advanced','') + ".meta.pbf");
  const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
  const decoder = async () => {
    return new Promise((resolve) => {
      return resolve(MltDecoder.decodeMlTile(data, tilesetMetadata));
    });
  };
  await stats(basename(tilePath), decoder);
  await main(data, decoder, tilesetMetadata, iterations, true);
  await main(data, decoder, tilesetMetadata, iterations, false);
}()

