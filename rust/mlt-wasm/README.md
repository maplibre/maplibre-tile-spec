# mlt-wasm

WebAssembly bindings for the [MapLibre Tile (MLT)](https://github.com/maplibre/maplibre-tile-spec) decoder.

Compiles `mlt-core` to WASM via [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) and ships a TypeScript wrapper exposing a `VectorTileLike` API.

## Layout

```
mlt-wasm/
├── src/lib.rs         # wasm-bindgen bindings
├── js/
│   ├── index.ts       # package entry point
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
