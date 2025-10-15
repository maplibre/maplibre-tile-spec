# maplibre-tile-spec

This package contains a JavaScript decoder for the experimental MapLibre Tile (MLT) vector tile format.

## Install

`npm install @maplibre/maplibre-tile-spec`

## Quickstart

To decode a tile, you will want to load `MltDecoder`:

```js
import { decodeTile } from '@maplibre/maplibre-tile-spec';

const data = fs.readFileSync(tilePath);
const tile = decodeTile(data);
```
## Contents

### Code

Code is in `src/`.

### Tests

Tests are in `test/unit/`. Run tests by running `npm run test`.
currently not integrated
