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
│   ├── columns.ts     # typed-array layers, as decoded
│   └── vectorTile.ts  # VectorTileLike wrapper over columns.ts
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

The `unstable-v2` feature is on by default, so v2 layers decode. Building with
`--no-default-features` skips them.

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
        console.log(feature.properties);     // read from the decoded columns
        console.log(feature.loadGeometry()); // Point[][]
    }
}
```

### Columns

`decodeTileColumns` hands each layer over as the typed arrays it decodes to, for consumers that
build their own buffers, such as a renderer. Nothing per feature crosses the WASM boundary.

```ts
import { decodeTileColumns, isPresent } from '@maplibre/mlt-wasm';

for (const layer of decodeTileColumns(data).layers) {
    const { dimension, vertices, zStep } = layer.geometry;
    // vertices: Int32Array of (x, y) pairs, or (x, y, z) triples when dimension is 3
    // geometryOffsets, partOffsets, ringOffsets: cumulative offsets, left out when not needed
    // triangleOffsets, indexBuffer: present on tessellated layers
    for (const column of layer.properties) {
        // column.values has one slot per feature; a slot without a value holds 0 or ""
        const has0 = isPresent(column, 0);
    }
    // layer.mValues: v2 vertex-scoped columns, one value per vertex
}
```

Each column is `{ name, type, values, present? }`. `present` is a bitmap with one bit per
feature, LSB-first, left out when every feature has a value. `bool` comes as `0`/`1`, and
64-bit integers, ids included, as `Float64Array`, so values above `Number.MAX_SAFE_INTEGER`
lose precision.

### 3D

`decodeTile3D` reads a tile whose every layer has z coordinates. Its features have the same
`loadGeometry()` and `loadPolygons()`, with every vertex an `[x, y, z]` array, so the rest of the
code can assume 3D. It throws when any layer has no z coordinates, for which `decodeTile` reads the
tile in 2D, and when any layer uses the `TessPolygons` or `TessPolygonsWithOutlines` geometry
layout. `decodeTileColumns` reads every layout, in 2D and 3D.

Like `x` and `y`, `z` is the stored integer: it lies on the layer's `zStep` grid, and its
elevation is `-10000 + z * 10 ** zStep` metres.

```ts
import { decodeTile3D } from '@maplibre/mlt-wasm';

const tile = decodeTile3D(data);
for (const [name, layer] of Object.entries(tile.layers)) {
    for (let i = 0; i < layer.length; i++) {
        const feature = layer.feature(i);
        // exterior ring, then holes; each ring is closed (its first vertex is repeated at the end)
        const polygons = feature.loadPolygons(); // [x, y, z][][][]
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
const { bufLen, regions } = tile.tree();        // the whole tile

for (const [i, region] of regions.entries()) {
    console.log(region.offset, region.len, region.label, region.value);
    if (region.blob) {
        console.log(tile.decodeBlob(i, 64));    // decoded values, capped
    }
}

console.log(tile.error);                        // null, or what stopped the walk
tile.free();
```

`tree(layerIndex)` narrows to one top-level layer, but `decodeBlob` keeps counting the regions of
the whole tile.

A handle, not a document: the tile bytes stay on the WASM side, `tree()` crosses once per layer
and is memoized, and payload values are decoded one blob at a time with a cap the caller picks.

A malformed tile never throws. The walk hands back the regions it reached, `error` carries the
failure, and a final `<unannotated>` leaf covers the bytes it never got to, so the leaves still
partition the buffer.
