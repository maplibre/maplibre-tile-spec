# Implementation Status

## Integrations

| Integration | Status |
|---|---|
| [MapLibre Native](https://maplibre.org/maplibre-native/) | Supported since MapLibre Android 12.1.0, MapLibre iOS 6.2.0. PR: [#3246](https://github.com/maplibre/maplibre-native/pull/3246). |
| [MapLibre GL JS](https://maplibre.org/maplibre-gl-js/) | Supported since version 5.12.0. PR: [#6570](https://github.com/maplibre/maplibre-gl-js/pull/6570). |
| Planetiler | Can generate tiles in MLT since version [0.10.0](https://github.com/onthegomap/planetiler/releases/tag/v0.10.0). |
| PMTiles | Can store MLT, see [PMTiles v3 spec - Tile Type](https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md#tile-type-tt). |
| [Martin tile server](https://maplibre.org/martin/) | Can detect and serve MLT since version [1.3.0](https://github.com/maplibre/martin/releases/tag/martin-v1.3.0). PR: [#2512](https://github.com/maplibre/martin/pull/2512). |
| [QGIS plugin](https://github.com/maplibre/maplibre-tile-spec/tree/main/qgis) | Opens MLT files in QGIS via the Rust `mlt` parser. See [qgis/README.md](https://github.com/maplibre/maplibre-tile-spec/blob/main/qgis/README.md). |

## Implementations

| Implementation | Language | Type | Notes |
|---|---|---|---|
| [maplibre-tile-spec/cpp](https://github.com/maplibre/maplibre-tile-spec/tree/main/cpp) | C++ | Decoder | Used by MapLibre Native |
| [maplibre-tile-spec/js](https://github.com/maplibre/maplibre-tile-spec/tree/main/js) | TypeScript | Decoder | Used by MapLibre GL JS |
| [maplibre-tile-spec/java](https://github.com/maplibre/maplibre-tile-spec/tree/main/java) | Java | Encoder | |
| [maplibre-tile-spec/rust/mlt](https://github.com/maplibre/maplibre-tile-spec/blob/main/rust/mlt/README.md) | Rust | Decoder / Encoder | Includes bundled `mlt` CLI, TUI, and conversion tooling. |
| [maplibre-tile-spec/rust/mlt-wasm](https://github.com/maplibre/maplibre-tile-spec/tree/main/rust/mlt-wasm) | Rust / WASM | Decoder | |
| [maplibre-tile-spec/rust/mlt-core](https://github.com/maplibre/maplibre-tile-spec/blob/main/rust/mlt-core/README.md) | Rust | Decoder / Encoder | Used in Martin. Includes bundled `mlt` CLI, TUI, and conversion tooling. |
| [maplibre-tile-spec/rust/mlt-wasm](https://github.com/maplibre/maplibre-tile-spec/tree/main/rust/mlt-wasm) | Rust / WASM | Decoder | Uses `mlt-core` |
| [maplibre-tile-spec/rust/mlt-py](https://github.com/maplibre/maplibre-tile-spec/tree/main/rust/mlt-py) | Rust / [PyO3](https://pyo3.rs/). | Decoder / Encoder | Exposes `mlt-core` to the python communtiy via [PyO3](https://pyo3.rs/). |
| [maplibre-tile-spec/rust/mlt-ffi](https://github.com/maplibre/maplibre-tile-spec/tree/main/rust/mlt-ffi) | Rust / FFI | Decoder / Encoder | Foreign-function bindings for `mlt-core` to the java/kotlin, cpp and c community |

## Supporting tooling

| Tool | Language | Type | Notes |
|---|---|---|---|
| [maplibre-tile-spec/rust/mlt](https://github.com/maplibre/maplibre-tile-spec/blob/main/rust/mlt/README.md) | Rust | Decoder / Encoder | `mlt` CLI, TUI, and conversion tooling. |
