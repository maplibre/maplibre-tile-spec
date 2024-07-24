# maplibre-tile-spec

This package contains a JavaScript decoder for the experimental MapLibre Tile (MLT) vector tile format.

## Install

`npm install @maplibre/maplibre-tile-spec`

## Quick Start

To decode a tile, you will want to load `MltDecoder` and `TileSetMetadata`:

```js
import { MltDecoder, TileSetMetadata } from '@maplibre/maplibre-tile-spec';

const data = fs.readFileSync(tilePath);
const metadata = fs.readFileSync(metadataPath);
const tilesetMetadata = TileSetMetadata.fromBinary(metadata);
const tile = MltDecoder.decodeMlTile(data, tilesetMetadata);
```

## Contents

### Code

Code is in `src/`.

### Tests

Tests are in `test/unit/`. Run tests by running `npm run test`.

### Benchmarks

Benchmarks are in `bench/`. Run benchmarks by running `npm run bench`.
