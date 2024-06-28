# Benchmarks

This directory contains benchmarks comparing MVT decoding to MLT decoding.

It assumes the reader will first try things out, and then will be interested to learn more. Therefore this document is structured with the details for running the benchmarks first, followed by context about them.

## Running

First make sure you've installed the dependencies of the project:

```bash
npm ci
```

Then to run the entire suite of benchmarks that decodes dozens of Bing tiles both as MVT and MLT you can do:

```bash
npm run bench
```

Developers seeking to optimize the code may wish to run single tile benchmark before and after a change. An example of that is:

```bash
node dist/bench/decode-mlt.js ../test/expected/bing/5-16-11.mlt
```

To compare the decoding speed to MVT do:

```bash
node dist/bench/decode-mvt.js ../test/fixtures/bing/5-16-11.mvt
```

If you want to run the benchmark for more time pass an additional argument with how many iterations to execute:

```bash
node dist/bench/decode-mlt.js ../test/expected/bing/5-16-11.mlt 5000
```

## Developing

If you would like to see if an optimization improves decoding performance make changes to the code in `./src` and then rebuild:

```bash
npm run build
```

Then the benchmarks can be updated and re-run. Alternatively you can avoid the `npm run build` step with this one-liner:

```bash
npx ts-node bench/decode-mlt.ts ../test/expected/bing/5-16-11.mlt
```

## Benchmark structure

The key files are:

- `bench/decode-mlt.ts`: Typescript benchmark of MLT decoding performance against any MLT passed in as a command line argument. Gets compiled to `./dist/bench/decode-mlt.js`

- `bench/decode-mvt.ts`: Typescript benchmark of MVT decoding performance against any MVT passed in as a command line argument. Gets compiled to `./dist/bench/decode-mvt.js`

- `bench/decode-bing.ts`: Typescript benchmark of all Bing MVT sample tiles in the ../test directory (MVT are in `../test/fixtures` and MLT and the MLT metadata are in `../test/expected`)

- `bench/utils.ts` Typescript core logic used across the benchmark scripts that contains various shared code to execute both the MLT and MVT benchmarks.

## Benchmark design

- Each benchmark runs for a default of `50` iterations to "Warmup" the code and then another `500` iterations for the main "Bench" run.
  - A developer should primarily care about the "Bench" run, but if the "Warmup" run is drastically different that also may be important to pay attention to.
- Separate scripts are used for benchmarking MVT and MLT in independent node processes in order to avoid unexpected interactions between the two decoding libraries. None are expected, but this rules out the possibility.
- Only node.js is supported currently, but we plan to port the benchmarks to the browser in the future.
- A very simple calculation is made of average decoding time over a number of iterations. This is then displayed in the console in two formats: operations per second and milliseconds per operation.
  - More sophisticated methods can be used to understand performance and variability between runs (notably p90 and p99 timing), but this was determined to be an adequate baseline starting place.
- The benchmarks tooling is asynchronous but the actual decoding code is largely synchronous. This is by design in order to match how MapLibre works. In MapLibre tile decoding is currently parallelized in a thread/web worker, but run synchonously within that web worker to avoid unbounded resource usage.

## Benchmark goals

There are multiple goals of these benchmarks:

1. Provide a way to test performance optimizations to the MLT decoder. The MLT decoder has not yet been optimized and major opportunities exist, see [future.md](../future.md) for details.

1. Explore how expensive MVT decoding is with comparisons including raw decoding vs decoding of gzip'd MVT vs decoding of gzip'd MVT that are tesselated with [earcut](https://github.com/mapbox/earcut).

1. Explore tile sizes of the wire format of each scenario: gzipped vs raw for both MVT and MLT

## Discussion & Intepreting results

Currently the most apples to apples comparison between MVT decoding and MLT decoding is to compare the performance of `MVT decoding of gzip'd data` to `MLT decoding of raw` data that has not been gzip'd. This is because the MLT spec is designed to avoid the need for a "heavyweight" encoding to run over the entire tile and rather uses smart internal encodings to keep sizes down without the need for a "heavyweight" encoding. The advantage of this is that tiles can be worked with without being uncompressed. But MVT heavily depends on gzip compression and while it is not in the spec it was essentially the standard. So it is rare for any MVT implementation to not gzip compress (or use some similiar "heavyweight" encoding) to keep sizes down.

It is for this reason that the 🍎 is used in both the console output for the MVT and MLT benchmarks. This is to help the developers eye quickly find the tests that are most comparable.

With discussion of 🍎 out of the way, let's also discuss the other fruit:

 - 🍏 shows the MVT performance with earcut tesselation added in. This is relevant because MapLibre needs to invoke earcut for polygon rendering and it is known to be very expensive (multiples more expensive than tile decoding). So, this benchmark indicates how much more expensive this is on top of just MVT decoding for a given input tile, which will vary by the complexity of the polygon features in the tile. Notably the MLT spec includes support for pre-tesselation such that this expense can be avoided in the future. Note: no 🍏 exists yet for MLT decoding because pre-tesselation support, while supported in the spec, is not yet supported in the JS decoder: https://github.com/maplibre/maplibre-tile-spec/issues/223
 - 🍊 shows the performance of heavyweight gzip compression over MLT. Again, as mentioend above, this should be unneeded, but it is added as a reference. Currently the JS decoder does not support the "Advanced" encodings that really make the biggest difference in size: https://github.com/maplibre/maplibre-tile-spec/issues/222. Therefore gzip compressing does help reduce sizes. To the extent that the JS decoder gains the "advanced" encoding support the size benefits of gzip will be reduced.
 - 🍐 is an oddball and not very representative, but added for reference. As discussed above MVT are exclusively compressed with gzip, but this shows the decoding performance of raw tiles without the overhead of gzip decompression.
