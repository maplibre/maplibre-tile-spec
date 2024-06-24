#!/usr/bin/env node

import { MltDecoder, TileSetMetadata } from '../src/index';
import { VectorTile } from '@mapbox/vector-tile';
import Protobuf from 'pbf';
import { readFileSync, existsSync } from "fs";
import * as benchmark from 'benchmark';
import { execSync } from "child_process";
import { assert } from 'console';


const args = process.argv.slice(2);

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

if (args.includes('--one')) {
  tiles.length = 1;
}

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

const willValidate = args.includes('--validate');
const willRunMLT = args.includes('--mlt');
const willRunMVT = args.includes('--mvt');
if (!willRunMLT && !willRunMVT) {
  console.error("Please provide at least one of --mlt or --mvt");
  process.exit(1);
}

let maxTime = 2;
if (process.env.GITHUB_RUN_ID) {
  maxTime = 2;
  console.log(`Running in CI, using smaller maxTime: ${maxTime} seconds`);
}

const mltDecode = (input) => {
  const decoded = MltDecoder.decodeMlTile(input.mltTile, input.tilesetMetadata);
  let count = 0;
  for (const layer of decoded.layers) {
    for (const feature of layer.features) {
      feature.toGeoJSON(input.x, input.y, input.z);
      count++;
    }
  }
  return count;
}

const mvtDecode = (input) => {
  const vectorTile = new VectorTile(new Protobuf(input.mvtTile));
  const mvtLayers = Object.keys(vectorTile.layers);
  let count = 0;
  for (const layerName of mvtLayers) {
    const layer = vectorTile.layers[layerName];
    for (let i = 0; i < layer.length; i++) {
      const feature = layer.feature(i);
      feature.toGeoJSON(input.x, input.y, input.z);
      count++;
    }
  }
  return count;
}

const load = (tile) => {
  const metadata: Buffer = readFileSync(`../test/expected/${tile}.mlt.meta.pbf`);
  const mvtTile: Buffer = readFileSync(`../test/fixtures/${tile}.mvt`);
  const mltTile: Buffer = readFileSync(`../test/expected/${tile}.mlt`);
  const uri = tile.split('/')[1].split('-').map(Number);
  const { z, x, y } = { z: uri[0], x: uri[1], y: uri[2] };
  const tilesetMetadata = TileSetMetadata.fromBinary(metadata);
  return { tile, x, y, z, mltTile, mvtTile, tilesetMetadata };
}

const validate = async (input) => {
  return new Promise((resolve) => {
    const mltResult = mltDecode(input);
    const mvtResult = mvtDecode(input);
    for (const layerName of Object.keys(mltResult)) {
      const features = mltResult[layerName];
      const mvtFeatures = mvtResult[layerName];
      assert(features.length === mvtFeatures.length, `Feature count mismatch for ${input.tile}`);
      for (let i = 0; i < features.length; i++) {
        const feature = features[i];
        const mvtFeature = mvtFeatures[i];
        const featureKeys = Object.keys(feature.properties).sort();
        const mvtFeatureKeys = Object.keys(mvtFeature.properties).sort();
        // workaround https://github.com/maplibre/maplibre-tile-spec/issues/181
        if (mvtFeatureKeys.indexOf('id') !== -1) {
          mvtFeatureKeys.splice(mvtFeatureKeys.indexOf('id'), 1);
        }
        assert(featureKeys.length === mvtFeatureKeys.length, `Feature keys mismatch for ${input.tile}`);
        const featureKeysStr = JSON.stringify(featureKeys);
        const mvtFeatureKeysStr = JSON.stringify(mvtFeatureKeys);
        if (featureKeysStr !== mvtFeatureKeysStr) {
          console.error(`Validation failed for ${input.tile} ${layerName} ${i}`);
          console.log(featureKeysStr);
          console.log('  vs')
          console.log(mvtFeatureKeysStr);
          process.exit(1);
        }
        assert(feature.geometry.type === mvtFeature.geometry.type, `Geometry type mismatch for ${input.tile} ${layerName} ${i}`);
        assert(feature.geometry.coordinates.length === mvtFeature.geometry.coordinates.length, `Geometry coordinates length mismatch for ${input.tile} ${layerName} ${i}`);
        for (let j = 0; j < feature.geometry.coordinates.length; j++) {
          const coord = feature.geometry.coordinates[j];
          const mvtCoord = mvtFeature.geometry.coordinates[j];
          assert(coord.length === mvtCoord.length, `Geometry coordinate length mismatch for ${input.tile} ${layerName} ${i}`);
          // TODO: cannot validate equal coordinates yet due to https://github.com/maplibre/maplibre-tile-spec/issues/184
        }
      }
    }
    resolve(null);
  })
}

const runSuite = async (inputs) => {
  return new Promise((resolve) => {
      const tile = inputs.tile;
      const suite = new benchmark.Suite;
      suite.on('cycle', function(event: Event) {
        console.log(String(event.target));
      })
      if (willRunMVT) {
        let opts = null;
        suite
          .add(`MVT ${tile}`, {
            defer: true,
            maxTime: maxTime,
            fn: (deferred: benchmark.Deferred) => {
                const count = mvtDecode(inputs);
                if (opts === null) {
                  opts = count;
                } else {
                  if (count !== opts || count < 1) {
                    console.error(`Feature count mismatch for ${tile}`);
                    process.exit(1);
                  }
                }
                deferred.resolve();
            }
          }).
          on('complete', () => {
            console.log(`  Total MVT features processed: ${opts}`);
            resolve(null);
          });
      }
      if (willRunMLT) {
        let opts = null;
        suite
          .add(`MLT ${tile}`, {
              defer: true,
              maxTime: maxTime,
              fn: (deferred: benchmark.Deferred) => {
                const count = mltDecode(inputs);
                if (opts === null) {
                  opts = count;
                } else {
                  if (count !== opts || count < 1) {
                    console.error(`Feature count mismatch for ${tile}`);
                    process.exit(1);
                  }
                }
                deferred.resolve();
              }
          }).
          on('complete', () => {
            console.log(`  Total MLT features processed: ${opts}`);
            resolve(null);
          });
      }
      suite.run({ async: true });
  })
}

const runSuites = async (tiles) => {
  const inputs = [];
  for (const tile of tiles) {
      inputs.push(load(tile));
  }
  if (willValidate) {
    for (const input of inputs) {
      console.log(`Validating result for ${input.tile}`);
      await validate(input);
      console.log(` ✔ passed for ${input.tile}`);
    }
  }
  for (const input of inputs) {
    console.log(`Running benchmarks for ${input.tile}`);
    await runSuite(input);
  }
}

runSuites(tiles);
