# maplibre-tile-spec

This package contains a JavaScript encoder and decoder for the experimental MapLibre Tile (MLT) vector tile format.

## Install

`npm install @maplibre/maplibre-tile-spec`

## Quickstart

### Decode a tile

```js
import { decodeTile } from '@maplibre/maplibre-tile-spec';

const data = fs.readFileSync(tilePath);
const tile = decodeTile(data);
```

### Encode a tile

```ts
import { encodeTile, type Layer } from '@maplibre/maplibre-tile-spec';

const layers: Layer[] = [
    {
        name: 'places',
        features: [
            {
                id: 1,
                geometry: { type: 'Point', coordinates: [10, 20] },
                properties: { name: 'Example' },
            },
        ],
    },
];

const data = encodeTile(layers);
```

## Contents

### Code

Code is in `src/`.

### Tests

Tests are in `test/unit/`. Run tests by running `npm run test`.
currently not integrated
