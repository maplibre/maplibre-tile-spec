

import { performance } from "perf_hooks";
import { access } from 'node:fs/promises';

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

function logger(message: string, ops: number, elapsed: number) {
    console.log('-', Math.round(ops / (elapsed / 1000)), 'ops/s |', Math.round(elapsed/ops), 'ms/op', 'for', message, '(runs:', ops, ')');
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

const decode = async (impl) : Promise<number> => {
    const tile = await impl();
    const layerNames = Object.keys(tile.layers).sort((a, b) => a.localeCompare(b));
    let count = 0;
    for (const layerName of layerNames) {
      const layer = tile.layers[layerName];
      for (let i = 0; i < layer.length; i++) {
          const feature = layer.feature(i);
          if (feature.loadGeometry().length > 0) {
            count++;
          }
      }
    }
    return count;
}

const run = async (name: string, impl, expectedCount: number, iterations: number) : Promise<number> => {
    const start = performance.now();
    let ops = 0;
    for (let i=0; i<iterations; i++) {
      const count = await decode(impl);
      if (count !== expectedCount) {
        console.error("Error: unexpected count", count, "expected", expectedCount);
        process.exit(1);
      }
      ops++;
    }
    const elapsed = (performance.now() - start);
    logger(name, ops, elapsed);
    return;
  }


const stats = async (name: string, impl): Promise<number> => {
    const count = await decode(impl);
    const verticies = await countVerticies(impl);
    console.log(`Decoding ${name} with ${count} features and ${verticies} vertices`);
    return count;
}

export const bench = async (name: string, decoder: () => void, iterations: number) : Promise<void> => {
    const expectedCount = await stats(name, decoder);
    // Only do a warmup if we're running more than 100 iterations for the main test
    if (iterations > 50) {
      await run('Warmup', decoder, expectedCount, 50);
    }
    await run("Main  ", decoder, expectedCount, iterations);
    return;
}


