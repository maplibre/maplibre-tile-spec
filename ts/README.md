# maplibre-tile-spec

This project contains a TypeScript decoder for the MapLibre Tile (MLT) vector tile format.

## Preparing the Environment 

Install a current version (>= 22) of Node.js.

Install node module dependencies
```bash
npm install
```

Build the project
```bash
npm run build
```

## Running the benchmarks

Go to the [benchmark directory ](./benchmark)to get more information about the data and benchmarks.

Running the decoding (transcoding) benchmarks for different basemaps datasets
```bash
npm run benchmark:decoding
```

Running the filtering benchmarks 
```bash
npm run benchmark:filering
```

The benchmarking results are stored in separate files in the dist folder.

