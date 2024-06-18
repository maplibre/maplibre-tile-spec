#!/usr/bin/env node

import { MltDecoder, TileSetMetadata } from '../src/index';
import { VectorTile } from '@mapbox/vector-tile';
import Protobuf from 'pbf';
import { readFileSync, existsSync } from "fs";
import * as benchmark from 'benchmark';
import { execSync } from "child_process";

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

console.log('Regenerating MLT files');
tiles.forEach(tile => {
  const cmd = `java -jar ../java/build/libs/encode.jar -mvt ../test/fixtures/${tile}.mvt -metadata -decode -mlt ../test/expected/${tile}.mlt`;
  console.log(cmd)
  execSync(cmd);
});

const runSuite = async (tile) => {
  console.log(`Running benchmarks for ${tile}`);
  const metadata: Buffer = readFileSync(`../test/expected/${tile}.mlt.meta.pbf`);
  const mvtTile: Buffer = readFileSync(`../test/fixtures/${tile}.mvt`);
  const mltTile: Buffer = readFileSync(`../test/expected/${tile}.mlt`);
  const uri = tile.split('/')[1].split('-').map(Number);
  const { z, x, y } = { z: uri[0], x: uri[1], y: uri[2] };
  const tilesetMetadata = TileSetMetadata.fromBinary(metadata);
  const featureTables = MltDecoder.generateFeatureTables(tilesetMetadata);

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
                const decoded = MltDecoder.decodeMlTile(mltTile, featureTables);
                const features = [];
                decoded.layers.forEach(layer => {
                  for (let i = 0; i < layer.features.length; i++) {
                    const feature = layer.features[i];
                    features.push(feature.toGeoJSON(x, y, z));
                  }
                })
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
