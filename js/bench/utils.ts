

import { performance } from "perf_hooks";
import { access } from 'node:fs/promises';
import earcut from 'earcut';
import { classifyRings } from '@maplibre/maplibre-gl-style-spec';
import { GeometryType } from "../src/data/GeometryType";

export const toKb = (bytes: number) => {
  return (bytes / 1024).toFixed(2);
}

export const parseArgs = async (args: string[]): Promise<{ tilePath: string; iterations: number; }> => {
    const tilePath = args[0];
    if (!tilePath || !((tilePath.endsWith(".mlt")) || (tilePath.endsWith(".mvt")))) {
      console.error("Please provide a path to an .mlt or .mvt file");
      process.exit(1);
    }
    await access(tilePath);
    const iterations = args[1] ? parseInt(args[1]) : 1000;
    if (Number.isNaN(iterations) || iterations < 1) {
      throw new Error("Please provide a valid number of iterations");
    }
    return { tilePath, iterations };
}

function round(value, precision) {
  const multiplier = Math.pow(10, precision || 0);
  return Math.round(value * multiplier) / multiplier;
}

function msToSec(ms) {
  return ms / 1000;
}

function logger(message: string, ops: number, elapsed: number) {
    const opsPerSec = Math.round(ops/msToSec(elapsed));
    const msPerOp = round(elapsed/ops,1);
    console.log('-', opsPerSec, 'ops/s |', msPerOp, 'ms/op', 'for', message, '(runs:', ops,')');
}

const countVerticies = async (impl) : Promise<number> => {
    const tile = await impl();
    const layerNames = Object.keys(tile.layers).sort((a, b) => a.localeCompare(b));
    let count = 0;
    for (const layerName of layerNames) {
      const layer = tile.layers[layerName];
      for (let i = 0; i < layer.length; i++) {
          const feature = layer.feature(i);
          const geometry = feature.loadGeometry();
          count += geometry.reduce((acc, g) => acc + g.length, 0);
      }
    }
    return count;
}

// Intended to match https://github.com/maplibre/maplibre-gl-js/blob/350064ecfe6c4bd074a19b5e7195cf010bede168/src/data/bucket/fill_bucket.ts#L172-L212
const tessellate = async (geometry) : Promise<number[]> => {
    const EARCUT_MAX_RINGS = 500;
    const triangles = [];
    for (const polygon of classifyRings(geometry, EARCUT_MAX_RINGS)) {
      const flattened = [];
      const holeIndices = [];
      for (const ring of polygon) {
          if (ring.length === 0) {
              continue;
          }
          if (ring !== polygon[0]) {
              holeIndices.push(flattened.length / 2);
          }
          flattened.push(ring[0].x);
          flattened.push(ring[0].y);
          for (let i = 1; i < ring.length; i++) {
              flattened.push(ring[i].x);
              flattened.push(ring[i].y);
          }
      }
      triangles.push(earcut(flattened, holeIndices));
  }
  return triangles;
}

const decode = async (impl, earcut: boolean) => {
    const tile = await impl();
    const layerNames = Object.keys(tile.layers).sort((a, b) => a.localeCompare(b));
    let featureCount = 0;
    let triangleCount = 0;
    for (const layerName of layerNames) {
      const layer = tile.layers[layerName];
      for (let i = 0; i < layer.length; i++) {
          const feature = layer.feature(i);
          const geometries = feature.loadGeometry();
          if (geometries.length === 0) {
            featureCount++;
          }
          if (earcut && feature.type === GeometryType.Polygon) {
            const triangles = await tessellate(geometries);
            triangleCount+=triangles.length;
          }
      }
    }
    return { featureCount, triangleCount };
}

const run = async (name: string, impl, earcut: boolean, expectedFeatures: number, expectedTriangles: number, iterations: number) : Promise<number> => {
    const start = performance.now();
    let ops = 0;
    for (let i=0; i<iterations; i++) {
      const { featureCount, triangleCount } = await decode(impl, earcut);
      if (featureCount !== expectedFeatures) {
        console.error("Error: unexpected count", featureCount, "expected", expectedFeatures);
        process.exit(1);
      }
      if (triangleCount !== expectedTriangles) {
        console.error("Error: unexpected count", triangleCount, "expected", expectedTriangles);
        process.exit(1);
      }
      ops++;
    }
    const elapsed = (performance.now() - start);
    logger(name, ops, elapsed);
    return;
  }


const stats = async (name: string, impl, earcut: boolean) => {
    const { featureCount, triangleCount } = await decode(impl, earcut);
    const verticies = await countVerticies(impl);
    let message = `${name} (${featureCount} features with ${verticies} vertices`;
    if (triangleCount) {
      message += ` and ${triangleCount} triangles)`;
    } else {
      message += ')';
    }
    console.log(message);
    return { featureCount, triangleCount };
}

export const bench = async (name: string, decoder: () => void, earcut: boolean, iterations: number) : Promise<void> => {
    const { featureCount, triangleCount } = await stats(name, decoder, earcut);
    // Only do a warmup if we're running more than 100 iterations for the main test
    if (iterations > 50) {
      await run('Warmup', decoder, earcut, featureCount, triangleCount, 50);
    }
    await run("Main  ", decoder, earcut, featureCount, triangleCount, iterations);
    return;
}


