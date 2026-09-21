# mlt-wasm

WebAssembly bindings for the [MapLibre Tile (MLT)](https://github.com/maplibre/maplibre-tile-spec) decoder.

Compiles `mlt-core` to WASM via [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) and ships a TypeScript wrapper exposing a `VectorTileLike` API.

## Layout

```
mlt-wasm/
├── src/lib.rs         # wasm-bindgen bindings
├── js/
│   ├── index.ts       # package entry point
│   ├── wasm.ts        # the one module that imports pkg/
│   ├── annotate.ts    # annotated-dump wire contract
│   └── vectorTile.ts  # VectorTileLike wrapper
├── pkg/               # wasm-pack output (gitignored)
├── dist/              # tsc output (gitignored)
├── Cargo.toml
├── package.json
└── tsconfig.json
```

## Build

Requires [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) and Node.js.

```sh
npm run build
```

This runs `wasm-pack build --target bundler --out-dir pkg` followed by `tsc`. Both steps are also available individually:

```sh
npm run build:wasm
npm run build:ts
```

## Usage

```ts
import { decodeTile } from '@maplibre/mlt-wasm';

const data = new Uint8Array(await fetch(tileUrl).then(r => r.arrayBuffer()));
const tile = decodeTile(data);

for (const [name, layer] of Object.entries(tile.layers)) {
    for (let i = 0; i < layer.length; i++) {
        const feature = layer.feature(i);
        console.log(feature.type);           // 1 | 2 | 3
        console.log(feature.id);             // number | undefined
        console.log(feature.properties);     // fetched lazily from WASM
        console.log(feature.loadGeometry()); // Point[][]
    }
}
```

## Annotate

`annotateTile` walks a tile into the regions an annotated hexdump is made of: every byte beside
the meaning of the field that owns it. The crate is built with `unstable-v2`, so v1 and v2 tiles
both annotate.

```ts
import { annotateTile } from '@maplibre/mlt-wasm';

const tile = annotateTile(data);
const { bufLen, regions } = tile.tree();        // or tile.tree(layerIndex)

for (const [i, region] of regions.entries()) {
    console.log(region.offset, region.len, region.label, region.value);
    if (region.blob) {
        console.log(tile.decodeBlob(i, 64));    // decoded values, capped
    }
}

console.log(tile.error);                        // null, or what stopped the walk
tile.free();
```

A handle, not a document: the tile bytes stay on the WASM side, `tree()` crosses once per layer
and is memoized, and payload values are decoded one blob at a time with a cap the caller picks.

A malformed tile never throws. The walk hands back the regions it reached, `error` carries the
failure, and a final `<unannotated>` leaf covers the bytes it never got to, so the leaves still
partition the buffer.
