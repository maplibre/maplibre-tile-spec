#!/usr/bin/env node

import { MltDecoder, TileSetMetadata } from '../src/index';
import { readFileSync, existsSync } from "fs";
import { performance } from "perf_hooks";

const args = process.argv.slice(2);
const mltFilePath = args[0];
if (!mltFilePath || !(mltFilePath.endsWith(".mlt"))) {
  console.error("Please provide a path to an .mlt file");
  process.exit(1);
}

if (!existsSync(mltFilePath)) {
  console.error("File not found:", mltFilePath);
  process.exit(1);
}

const iterations = args[1] ? parseInt(args[1]) : 1000;
if (Number.isNaN(iterations) || iterations < 1) {
  console.error("Please provide a valid number of iterations");
  process.exit(1);
}

const mltMetadataPbf = readFileSync(mltFilePath.replace('.advanced','') + ".meta.pbf");
const tilesetMetadata = TileSetMetadata.fromBinary(mltMetadataPbf);
const mltTile = readFileSync(mltFilePath);

function decode() {
  const decoded = MltDecoder.decodeMlTile(mltTile, tilesetMetadata);
  const layerNames = Object.keys(decoded.layers).sort();
  for (const layerName of layerNames) {
    const layer = decoded.layers[layerName];
    for (let i = 0; i < layer.length; i++) {
        const feature = layer.feature(i);
        feature.loadGeometry();
    }
  }
}

let start = performance.now();
let ops = 0;

for (let i=0; i<100; i++) {
  decode();
  ops++;
}

let elapsed = (performance.now() - start);

console.log('Warmup: ' + Math.round(ops / (elapsed / 1000)) + ' ops/s | ' + Math.round(elapsed/ops) + ' ms/op (' + ops + ' runs sampled)');

start = performance.now();
ops = 0;

for (let i=0; i<iterations; i++) {
  decode();
  ops++;
}

elapsed = (performance.now() - start);

console.log('Run: ' + Math.round(ops / (elapsed / 1000)) + ' ops/s | ' + Math.round(elapsed/ops) + ' ms/op (' + ops + ' runs sampled)');
