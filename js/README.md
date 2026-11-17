# mlt-js

A pure JavaScript decoder for [MapLibre Tile (MLT)](https://github.com/maplibre/maplibre-tile-spec)
v1 and v2 tiles, `Meshes` layers included.

It has no runtime dependencies, no WASM and no build step for the user. Its API is the same as
[`mlt-wasm`](../rust/mlt-wasm/README.md)'s, so code can move from one to the other by changing the import.

## Layout

```
mlt-js/
├── dist/
│   ├── index.js       # full API, minified self-contained ES module
│   ├── index.d.ts     # TypeScript declarations for index.js
│   ├── columns.js     # columnar API only, minified self-contained ES module
│   └── columns.d.ts   # TypeScript declarations for columns.js
├── src/
│   ├── index.js       # package entry point of the sources
│   ├── index.d.ts     # TypeScript declarations for index.js
│   ├── columns.js     # typed-array layers, as decoded
│   ├── columns.d.ts   # TypeScript declarations for columns.js
│   ├── vectorTile.js  # VectorTileLike wrapper over columns.js
│   ├── featureGeometry.js
│   ├── geojson.js
│   ├── decoder/       # stream, geometry, string, nested and layer decoders
│   └── codecs/        # FastPFOR, rANS, FSST, ALP, RLE, Morton, triangle trees, ...
├── viewer/
│   ├── meshes.html    # three.js viewer of v2 Meshes tiles
│   └── serve.js       # static server for the viewer
├── test/              # node --test suites and TypeScript declaration checks
├── bench/             # decode speed, heap use, and a comparison with mlt-wasm
├── scripts/           # build and typecheck scripts
├── LICENSE-APACHE
├── LICENSE-MIT
└── package.json
```

## Bundles

Each `.js` file in `dist/` is a self-contained, minified ES module without dependencies, paired
with its `.d.ts` TypeScript declaration file.

| Entry point                | Bundle            | Types              | Exports                                                                                                                                                   | Size (brotli) |
| -------------------------- | ----------------- | ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------- |
| `@maplibre/mlt-js/columns` | `dist/columns.js` | `dist/columns.d.ts` | `decodeTileColumns`, `columnValue`, `mValueSlice`, `toElevation`, `MltGeometryType`                                                                       | 16 KB         |
| `@maplibre/mlt-js`         | `dist/index.js`   | `dist/index.d.ts`   | the above, and `decodeTile`, `decodeTile3D`, `tileGeoJson`, `featureGeometry`, `geometryStarts`, `isMeshes`, `isTrianglesOnly`, and the `Mlt*` classes | 20 KB         |

Use `@maplibre/mlt-js/columns` (`dist/columns.js`) for a renderer that builds its own GPU buffers,
such as the viewer. Use `@maplibre/mlt-js` (`dist/index.js`) for the `VectorTileLike` API of
MapLibre, for GeoJSON, or for features one at a time. For comparison, `mlt-wasm` is 127 KB (brotli).

The decoder requires ES2022, `TextDecoder` and `BigUint64Array`, as in current browsers, web
workers and Node.js 18 or newer. It takes uncompressed tile bytes, so a gzip-compressed tile must
be decompressed first.

## Usage

```sh
npm install @maplibre/mlt-js
```

In a browser without a bundler, import the bundle directly or map it with an import map:

```html
<script type="importmap">
  { "imports": { "@maplibre/mlt-js": "./dist/columns.js" } }
</script>
```

`@maplibre/mlt-js` resolves to `dist/index.js` (`dist/index.d.ts`), and `@maplibre/mlt-js/columns`
to `dist/columns.js` (`dist/columns.d.ts`):

```ts
import { decodeTile } from '@maplibre/mlt-js';

const tile = decodeTile(data);

for (const [name, layer] of Object.entries(tile.layers)) {
    for (let i = 0; i < layer.length; i++) {
        const feature = layer.feature(i);
        console.log(feature.type);           // 0 (Mesh) | 1 | 2 | 3
        console.log(feature.id);             // number | undefined
        console.log(feature.properties);     // read from the decoded columns
        console.log(feature.loadGeometry()); // Point[][]
    }
}
```

### Columns

`decodeTileColumns` gives each layer as the typed arrays it decodes to, for consumers that
build their own buffers, such as a renderer. It makes no object per feature.

```ts
import { columnValue, decodeTileColumns } from '@maplibre/mlt-js/columns';

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
    // a v2 layer's m-value columns are in layer.mValues
}
```

`decodeTileColumns(data, { layers: ['roads', 'water'] })` decodes only the layers with those names,
every one that has it, since layer names need not be unique. The other layers are skipped undecoded.

Each column is `{ name, type, values, present? }`. `present` is a bitmap with one bit per
feature, LSB-first, left out when every feature has a value. `bool` comes as `0`/`1`, and
64-bit integers, ids included, as `Float64Array`, so values above `Number.MAX_SAFE_INTEGER`
lose precision.

The typed arrays of a layer are its own: they stay valid after the next tile is decoded.

A 3D layer's `zStep` is the power of ten of its z grid's step in metres, and its `zDatum` is what
`z = 0` stands for: `"elevation"`, Terrain-RGB's base of -10000 m, or `"ground"`, so that z is a
height above it. `toElevation(z, zStep, zDatum)` converts one z to metres, exactly as `mlt-core` does.

A v2 layer's `mValues` hold its m-value columns, packed over the vertices or, in a `Meshes` layer,
the triangles of the features whose `present` bit is set, with strings as dictionary codes;
`mValueSlice(layer, column, f)` reads one feature's slice, and `MltFeature3D.loadMValues(name)`
does the same on `decodeTile3D`.

A `Meshes` layer's features are all `MltGeometryType.Mesh`: `geometryOffsets` starts each feature's
run of vertices, and `triangleOffsets` each feature's triangles in the layer-wide `indexBuffer`.
A mesh on the `"ground"` datum meets the terrain at the centre of its vertices' `(x, y)` bounding box.

```ts
import { decodeTileColumns, toElevation } from '@maplibre/mlt-js/columns';

for (const layer of decodeTileColumns(data).layers) {
    const g = layer.geometry;
    if (!g.meshes) continue;
    // one draw call per layer: upload g.vertices as ivec3 and g.indexBuffer as the index buffer,
    // and scale z in the shader by 10 ** g.zStep metres
    const metres = toElevation(g.vertices[2], g.zStep, g.zDatum);
}
```

### Features

`featureGeometry(layer, i)` reads feature `i` of a column layer without knowing the offset levels.
Its points, lines and polygon rings are views of the layer's vertices, still in tile coordinates,
and a polygon's rings come ready for earcut. A mesh's triangles (`kind: "mesh"`) come per feature,
counted from the feature's own vertices. `geometryStarts(layer.geometry)` gives where each
feature's geometries and each geometry's vertices start, for batched renderers such as deck.gl's
binary layers. Both work as in [`mlt-wasm`](../rust/mlt-wasm/README.md#features).

```ts
import { featureGeometry } from '@maplibre/mlt-js';

const g = featureGeometry(layer, i);
if (g.kind === 'polygon') {
    for (const { vertices, holeIndices } of g.polygons) {
        const triangles = earcut(vertices, holeIndices, layer.geometry.dimension);
    }
}
```

### 3D

`decodeTile3D` reads a tile whose every layer has z coordinates. Its features have the same
`loadGeometry()` and `loadPolygons()`, with every vertex an `[x, y, z]` array. A mesh's
`loadGeometry()` is one ring of its vertices, and `loadTriangles()` indexes into it. It throws
when any layer has no z coordinates, for which `decodeTile` reads the tile in 2D, and when any
layer uses the `TessPolygons` or `TessPolygonsWithOutlines` geometry layout. `decodeTileColumns`
reads every layout, in 2D and 3D.

```ts
import { decodeTile3D, MltGeometryType } from '@maplibre/mlt-js';

const tile = decodeTile3D(data);
for (const [name, layer] of Object.entries(tile.layers)) {
    for (let i = 0; i < layer.length; i++) {
        const feature = layer.feature(i);
        if (feature.mltType === MltGeometryType.Mesh) {
            const vertices = feature.loadGeometry()[0]; // [x, y, z][]
            const triangles = feature.loadTriangles();  // Uint32Array
        } else {
            const polygons = feature.loadPolygons();    // [x, y, z][][][]
        }
    }
}
```

### GeoJSON

`tileGeoJson(data)` gives the whole tile as one GeoJSON `FeatureCollection` in tile coordinates,
in the same form as `mlt decode --format geojson`. The tests use it to compare the decoder with
the Rust decoder's output for every fixture. Each feature has its layer name in
`properties._layer`, and a mesh has its triangles in `triangles`.

## Meshes viewer

`viewer/meshes.html` draws the v2 `Meshes` layers of a `.pmtiles` archive or of a directory of
`z/x/y.mlt` tiles with three.js, and decodes them with `dist/columns.js`. It is the viewer of
`mlt-wasm`, and shows the decode time per tile. Start the server, which serves the repository
root on port 8080 (`PORT` changes it):

```sh
npm run serve
```

Then open `http://localhost:8080/js/viewer/meshes.html?url=<archive>` or
`http://localhost:8080/js/viewer/meshes.html?tiles=<base>`, adding `&attribution=<text>` to credit
the data and `&quality=high`, `phone` or `low` to pick the detail. `just rust::mesh-site`
builds the same as a static site over a `CityGML` input.

## Development

The sources in `src/` run without a build. `dist/` is committed, so rebuild it after a change:

```sh
npm run build      # writes dist/index.{js,d.ts} and dist/columns.{js,d.ts}, and prints sizes
npm run typecheck  # checks dist/*.d.ts and test/types.test-d.ts with tsc (strict NodeNext)
npm test           # decodes every fixture in test/ and compares it with the Rust decoder's output
npm run bench      # decode speed and heap use per tile
node bench/compare_wasm.js   # the same tiles with mlt-wasm, which needs rust/mlt-wasm/pkg/
```

Run `npm install` to install the build and type-checking devDependencies (`esbuild`, `typescript`).

## Performance

Decode time per tile with `decodeTileColumns`, median of 5 rounds, Node.js on one machine:

| Tile                 | mlt-js   | mlt-wasm (SIMD128) |
| -------------------- | -------- | ------------------ |
| Hamburg z16, 88 KB   | 0.672 ms | 0.672 ms           |
| Hamburg z15, 61 KB   | 0.427 ms | 0.449 ms           |
| Hamburg z14, 255 KB  | 1.644 ms | 1.737 ms           |
| Hamburg z13, 196 KB  | 1.449 ms | 1.747 ms           |
| CityGML fixture z16  | 0.013 ms | 0.018 ms           |

The decoder reuses one scratch buffer for the temporary values of a tile, which it keeps to at
most 1 MB, so the decoded typed arrays are most of what a tile allocates.

## License

Licensed under either of [Apache License, Version 2.0](./LICENSE-APACHE) or
[MIT license](./LICENSE-MIT) at your option.
