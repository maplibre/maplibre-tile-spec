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
import { columnValue, decodeTileColumns } from '@maplibre/mlt-wasm';

for (const layer of decodeTileColumns(data).layers) {
    const { dimension, vertices, zStep } = layer.geometry;
    // vertices: Int32Array of (x, y) pairs, or (x, y, z) triples when dimension is 3
    // geometryOffsets, partOffsets, ringOffsets: cumulative offsets, left out when not needed
    // triangleOffsets, indexBuffer: present on tessellated layers
    for (const column of layer.properties) {
        // column.values has one slot per feature; a slot without a value holds 0 or ""
        for (let f = 0; f < layer.featureCount; f++) {
            const value = columnValue(column, f); // undefined when feature f has no value
        }
    }
    // v2 vertex-scoped (m-value) and nested columns are not passed on
}
```

`decodeTileColumns(data, { layers: ['roads', 'water'] })` decodes only the layers with those names,
every one that has it, since layer names need not be unique. The other layers are skipped undecoded.

Each column is `{ name, type, values, present? }`. `present` is a bitmap with one bit per
feature, LSB-first, left out when every feature has a value. `bool` comes as `0`/`1`, and
64-bit integers, ids included, as `Float64Array`, so values above `Number.MAX_SAFE_INTEGER`
lose precision.

A 3D layer's `zStep` is the power of ten of its z grid's step in metres; `toElevation(z, zStep)`
converts one z to metres, exactly as `mlt-core` does.

### Features

`featureGeometry(layer, i)` reads feature `i` of a column layer without knowing the offset levels.
Its points, lines and polygon rings are views of the layer's vertices, still in tile coordinates,
and a polygon's rings come ready for earcut. A tessellated layer's triangles come per feature,
counted from the feature's own vertices.

```ts
import { featureGeometry } from '@maplibre/mlt-wasm';

const g = featureGeometry(layer, i);
if (g.kind === 'polygon') {
    for (const { vertices, holeIndices } of g.polygons) {
        const triangles = earcut(vertices, holeIndices, layer.geometry.dimension);
    }
}
```

#### Building GPU buffers

`layer.geometry.vertices` can be uploaded as the vertex buffer as it is: `dimension` integers per
vertex, read as `ivec2` or `ivec3` in the shader. What a renderer adds next to it depends on whether
its shapes need tessellation on the CPU.

Shapes drawn as they are, such as points and lines (thick lines can be extruded in the vertex
shader), need to know where each line starts, to break lines apart, and which feature it belongs to,
to style or pick it. `geometryStarts(layer.geometry)` gives both for the whole layer, from the tile's
own offset levels: where each feature's geometries start, where each geometry's vertices start and, in
a layer with polygons, where each polygon's rings and each ring's vertices start. The tile stores
these as counts per level, not per vertex, and the arrays are their running totals. Each has one
entry more than it has items, and stored offset columns come back as they are. They are resolved once
per layer and every call returns the same arrays, so don't modify them.

In a layer of one geometry kind they are what a renderer takes for a batch, such as deck.gl's binary
`PathLayer` for lines:

```ts
import { geometryStarts } from '@maplibre/mlt-wasm';

const { featureGeometries, geometryVertices, ringVertices } = geometryStarts(layer.geometry);
// Lines: line g is vertices geometryVertices[g] .. geometryVertices[g + 1], and feature f's lines
// are featureGeometries[f] .. featureGeometries[f + 1].
new PathLayer({
    data: {
        length: geometryVertices.length - 1,
        startIndices: geometryVertices,
        attributes: { getPath: { value: layer.geometry.vertices, size: layer.geometry.dimension } },
    },
    _pathType: 'open',
});
// Polygons: geometryVertices starts each polygon, ringVertices each ring (outlines without the
// closing vertex).
```

A shader that extrudes lines itself reads one vertex at a time, so it needs the line as a vertex
attribute instead. That is one fill per line from the same starts, done by the renderer for the id it
uploads:

```ts
// A segment is drawn only if both ends are the same line.
const lineOfVertex = new Uint32Array(geometryVertices[geometryVertices.length - 1]);
for (let g = 0; g + 1 < geometryVertices.length; g++) {
    lineOfVertex.fill(g, geometryVertices[g], geometryVertices[g + 1]);
}
```

A geometry is a point, a line or a polygon, and every kind shares one sequence in feature order, so in
a layer mixing kinds, `layer.geometry.types` tells which features are which. A `TessPolygons` layer has
no runs of vertices, only triangles.

On a 1.3 MB tile of 1,333 line features and 524,482 vertices, `geometryStarts` takes about 0.006 ms
and the fill above about 0.2 ms, against about 3.8 ms for `decodeTileColumns` (Node, one machine).

Shapes that need tessellation on the CPU work one polygon or line at a time:

- A tessellation that only adds triangles keeps the vertex buffer: earcut's indices are counted from a
  polygon's view in `featureGeometry`, which comes with its hole indices, so adding its `firstVertex`
  makes them an index buffer over `vertices`. A tessellated layer's triangles are that already.
- A tessellation that creates vertices, such as ribbons with joins built on the CPU or extruded walls,
  reads each line or ring between its starts, writes a buffer of its own, and tags its output with the
  line or feature it came from.

### 3D

`decodeTile3D` reads a tile whose every layer has z coordinates. Its features have the same
`loadGeometry()` and `loadPolygons()`, with every vertex an `[x, y, z]` array, so the rest of the
code can assume 3D. It throws when any layer has no z coordinates, for which `decodeTile` reads the
tile in 2D, and when any layer uses the `TessPolygons` or `TessPolygonsWithOutlines` geometry
layout. `decodeTileColumns` reads every layout, in 2D and 3D.

Like `x` and `y`, `z` is the stored integer: it lies on the layer's `zStep` grid, and
`toElevation(z, layer.zStep)` gives its elevation in metres.

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
