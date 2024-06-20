#!/usr/bin/env node

import { MltDecoder, TileSetMetadata } from '../src/index';
import { VectorTile } from '@mapbox/vector-tile';
import Protobuf from 'pbf';
import { readFileSync, existsSync } from "fs";
import * as benchmark from 'benchmark';
import { execSync } from "child_process";
import { cwd } from 'process';

const tiles = [
  'bing/4-8-5',
  'bing/4-12-6',
  'bing/4-13-6',
  'bing/5-15-10',
  'bing/5-16-11',
  'bing/5-16-9',
  'bing/5-17-10',
  'bing/5-17-11',
];

if (!existsSync('../java/build/libs/encode.jar')) {
  console.log('encode.jar does not exist, building java project...');
  execSync('./gradlew cli', { cwd: '../java' });
}
tiles.forEach(tile => {
  if (!existsSync(`../test/expected/${tile}.mlt.meta.pbf`)) {
    console.log('Generating MLT tiles & metadata');
    const cmd = `java -jar ../java/build/libs/encode.jar -mvt ../test/fixtures/${tile}.mvt -metadata -decode -mlt ../test/expected/${tile}.mlt`;
    console.log(cmd)
    execSync(cmd);
  }
});

const runSuite = async (tile) => {
  console.log(`Running benchmarks for ${tile}`);
  const metadata: Buffer = readFileSync(`../test/expected/${tile}.mlt.meta.pbf`);
  const mvtTile: Buffer = readFileSync(`../test/fixtures/${tile}.mvt`);
  const mltTile: Buffer = readFileSync(`../test/expected/${tile}.mlt`);
  const uri = tile.split('/')[1].split('-').map(Number);
  const { z, x, y } = { z: uri[0], x: uri[1], y: uri[2] };
  const tilesetMetadata = TileSetMetadata.fromBinary(metadata);

  return new Promise((resolve, reject) => {
      const suite = new benchmark.Suite;
      suite
          .on('cycle', function(event: Event) {
              console.log(String(event.target));
          })
          .on('complete', () => {
              console.log('Fastest is ' + suite.filter('fastest').map('name'));
              resolve(null);
          })
          .add(`MLT ${tile}`, {
              defer: true,
              fn: (deferred: benchmark.Deferred) => {
                const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
                const features = [];
                for (const layer of decoded.layers) {
                  for (const feature of layer.features) {
                    features.push(feature.toGeoJSON());
                  }
                }
                deferred.resolve();
              }
          })
          .add(`MVT ${tile}`, {
            defer: true,
            fn: (deferred: benchmark.Deferred) => {
                const vectorTile = new VectorTile(new Protobuf(mvtTile));
                const features = [];
                const layers = Object.keys(vectorTile.layers);
                for (const layerName of layers) {
                  const layer = vectorTile.layers[layerName];
                  for (let i = 0; i < layer.length; i++) {
                    const feature = layer.feature(i);
                    features.push(feature.toGeoJSON(x, y, z));
                  }
                };
                deferred.resolve();
            }
        })
        .run({ 'async': true });
  })
}

const runSuites = async (tiles) => {
  for (const tile of tiles) {
      await runSuite(tile);
  }
}

runSuites(tiles);
