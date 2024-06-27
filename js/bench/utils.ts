

import { performance } from "perf_hooks";
import { existsSync } from "fs";

export function parseArgs(args: string[]): { tilePath: string, iterations: number } {
    const tilePath = args[0];
    if (!tilePath || !((tilePath.endsWith(".mlt")) || (tilePath.endsWith(".mvt")))) {
      console.error("Please provide a path to an .mlt or .mvt file");
      process.exit(1);
    }

    if (!existsSync(tilePath)) {
      console.error("File not found:", tilePath);
      process.exit(1);
    }

    const iterations = args[1] ? parseInt(args[1]) : 1000;
    if (Number.isNaN(iterations) || iterations < 1) {
      console.error("Please provide a valid number of iterations");
      process.exit(1);
    }
    return { tilePath, iterations };
}

function logger(message: string, ops: number, elapsed: number) {
    console.log('-', Math.round(ops / (elapsed / 1000)), 'ops/s for', message, '|', Math.round(elapsed/ops), ' ms/op (', ops, ' runs sampled)');
}

function countVerticies(impl: () => any) : number {
    const tile = impl();
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

function decode(impl: () => any) : number {
    const tile = impl();
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

function run(name: string, impl: () => any, expectedCount: number, iterations: number) {
    const start = performance.now();
    let ops = 0;
    for (let i=0; i<iterations; i++) {
      const count = decode(impl);
      if (count !== expectedCount) {
        console.error("Error: unexpected count", count, "expected", expectedCount);
        process.exit(1);
      }
      ops++;
    }
    const elapsed = (performance.now() - start);
    logger(name, ops, elapsed);
  }


function stats(name: string, impl: () => any) : number {
    const count = decode(impl);
    const verticies = countVerticies(impl);
    console.log(`Decoding ${name} with ${count} features and ${verticies} vertices`);
    return count;
}

export function bench(name: string, decoder: () => void, iterations: number) {
    const expectedCount = stats(name, decoder);
    // Only do a warmup if we're running more than 100 iterations for the main test
    if (iterations > 50) {
        run('Warmup', decoder, expectedCount, 50);
    }
    run("Main", decoder, expectedCount, iterations);
}

