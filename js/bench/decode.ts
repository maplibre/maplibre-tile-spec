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
const willRunMLTJSON = args.includes('--mltjson');
const willRunMVTJSON = args.includes('--mvtjson');
if (!willRunMLT && !willRunMVT && !willRunMLTJSON && !willRunMVTJSON) {
  console.error("Please provide at least one of --mlt, --mltjson, --mvt, or --mvtjson flags to run benchmarks.");
  process.exit(1);
}

let maxTime = 10;
if (process.env.GITHUB_RUN_ID) {
  maxTime = 2;
  console.log(`Running in CI, using smaller maxTime: ${maxTime} seconds`);
}

const decode = (decoded, collector) => {
  let count = 0;
  const layerNames = Object.keys(decoded.layers).sort();
  for (const layerName of layerNames) {
    const layer = decoded.layers[layerName];
    for (let i = 0; i < layer.length; i++) {
      const feature = layer.feature(i);
      const result = feature.loadGeometry();
      if (collector) {
        collector.push(result);
      }
      count++;
    }
  }
  return count;
}

const decodeJSON = (input, decoded, collector) => {
  let count = 0;
  const layerNames = Object.keys(decoded.layers).sort();
  for (const layerName of layerNames) {
    const layer = decoded.layers[layerName];
    for (let i = 0; i < layer.length; i++) {
      const feature = layer.feature(i);
      const result = feature.toGeoJSON(input.x, input.y, input.z);
      if (collector) {
        collector.push(result);
      }
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
    const features = [];
    const mvtFeatures = [];
    const decoded = MltDecoder.decodeMlTile(input.mltTile, input.tilesetMetadata);
    const mltCount = decodeJSON(input, decoded, features);
    const mvtDecoded = new VectorTile(new Protobuf(input.mvtTile));
    const mvtCount = decodeJSON(input, mvtDecoded, mvtFeatures);
    assert(mltCount === mvtCount, `Feature count mismatch for ${input.tile}`);
    assert(features.length === mvtFeatures.length, `Feature array count mismatch for ${input.tile}`)
    assert(features.length > 0, `No features found for ${input.tile}`)
    assert(mvtFeatures.length > 0, `No features found for ${input.tile}`)
    let count = 0;
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
        console.error(`Validation failed for ${input.tile} ${i}`);
        console.log(featureKeysStr);
        console.log('  vs')
        console.log(mvtFeatureKeysStr);
        process.exit(1);
      }
      assert(feature.geometry.type === mvtFeature.geometry.type, `Geometry type mismatch for ${input.tile} ${i}`);
      assert(feature.geometry.coordinates.length === mvtFeature.geometry.coordinates.length, `Geometry coordinates length mismatch for ${input.tile} ${i}`);
      for (let j = 0; j < feature.geometry.coordinates.length; j++) {
        const coord = feature.geometry.coordinates[j];
        const mvtCoord = mvtFeature.geometry.coordinates[j];
        assert(coord.length === mvtCoord.length, `Geometry coordinate length mismatch for ${input.tile} ${i}`);
        // TODO: cannot validate equal coordinates yet due to https://github.com/maplibre/maplibre-tile-spec/issues/184
      }
      count++;
    }
    assert(count === mltCount, `Validation count mismatch for ${input.tile}`)
    resolve(null);
  })
}

const runSuite = async (input) => {
  return new Promise((resolve) => {
      const tile = input.tile;
      const suite = new benchmark.Suite;
      suite.on('cycle', function(event: Event) {
        console.log(String(event.target));
      })
      if (willRunMVT) {
        let opts = null;
        suite
          .add(`MVT -> loadGeo ${tile}`, {
            defer: true,
            maxTime: maxTime,
            fn: (deferred: benchmark.Deferred) => {
                const decoded = new VectorTile(new Protobuf(input.mvtTile));
                const count = decode(decoded, null);
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
            console.log(`  Total MVT features(loadGeometry) processed: ${opts}`);
            resolve(null);
          });
      }
      if (willRunMLT) {
        let opts = null;
        suite
          .add(`MLT -> loadGeo ${tile}`, {
            defer: true,
            maxTime: maxTime,
            fn: (deferred: benchmark.Deferred) => {
                const decoded = MltDecoder.decodeMlTile(input.mltTile, input.tilesetMetadata);
                const count = decode(decoded, null);
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
            console.log(`  Total MLT features(loadGeometry) processed: ${opts}`);
            resolve(null);
          });
      }
      if (willRunMVTJSON) {
        let opts = null;
        suite
          .add(`MVT -> GeoJSON ${tile}`, {
            defer: true,
            maxTime: maxTime,
            fn: (deferred: benchmark.Deferred) => {
              const decoded = new VectorTile(new Protobuf(input.mvtTile));
                const count = decodeJSON(input, decoded, null);
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
            console.log(`  Total MVT features(json) processed: ${opts}`);
            resolve(null);
          });
      }
      if (willRunMLTJSON) {
        let opts = null;
        suite
          .add(`MLT -> GeoJSON ${tile}`, {
              defer: true,
              maxTime: maxTime,
              fn: (deferred: benchmark.Deferred) => {
                const decoded = MltDecoder.decodeMlTile(input.mltTile, input.tilesetMetadata);
                const count = decodeJSON(input, decoded, null);
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
            console.log(`  Total MLT features(json) processed: ${opts}`);
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
