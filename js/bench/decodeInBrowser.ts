/*
 * js/bench/decodeInBrowser.ts
 *
 * Run benchmarks in the browser. To build:
 *
 *  npm run build-bench
 *
 * Then run a simple server from the root of the repo and navigate to
 *
 *  localhost:[your_port]/js/bench.html
 */
import { MltDecoder, TileSetMetadata } from '../src/index';
import { VectorTile } from '@mapbox/vector-tile';
import Protobuf from 'pbf';

declare var Benchmark: any;

const tiles = [
  // 'bing/4-8-5',
  // 'bing/4-12-6',
  'bing/4-13-6',
  // 'bing/5-15-10',
  // 'bing/5-16-11',
  // 'bing/5-16-9',
  // 'bing/5-17-10',
  // 'bing/5-17-11',
];

let maxTime = 10;

const runSuite = async (tile) => {
  console.log(`Running benchmarks for ${tile}`);
  const metadataResponse = await fetch(`../test/expected/${tile}.mlt.meta.pbf`);
  const metadata: ArrayBuffer = await metadataResponse.arrayBuffer();

  const mvtResponse = await fetch(`../test/fixtures/${tile}.mvt`);
  const mvtTile: ArrayBuffer = await mvtResponse.arrayBuffer();

  const mltResponse = await fetch(`../test/expected/${tile}.mlt`);
  const mltTile: Uint8Array = new Uint8Array(await mltResponse.arrayBuffer());

  const uri = tile.split('/')[1].split('-').map(Number);
  const { z, x, y } = { z: uri[0], x: uri[1], y: uri[2] };
  const tilesetMetadata = TileSetMetadata.fromBinary(new Uint8Array(metadata));

  return new Promise((resolve) => {
      const suite = new Benchmark.Suite;
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
              maxTime: maxTime,
              fn: (deferred) => {
                const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
                const features = [];
                const layers = Object.keys(decoded.layers);
                for (const layerName of layers) {
                  const layer = decoded.layers[layerName];
                  for (const feature of layer.features) {
                    features.push(feature.toGeoJSON(z, y, z));
                  }
                }
                deferred.resolve();
              }
          })
          .add(`MVT ${tile}`, {
            defer: true,
            maxTime: maxTime,
            fn: (deferred) => {
                const vectorTile = new VectorTile(new Protobuf(mvtTile));
                const features = [];
                const layers = Object.keys(vectorTile.layers);
                for (const layerName of layers) {
                  const layer = vectorTile.layers[layerName];
                  for (let i = 0; i < layer.length; i++) {
                    const feature = layer.feature(i);
                    features.push(feature.toGeoJSON(x, y, z));
                  }
                }
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
