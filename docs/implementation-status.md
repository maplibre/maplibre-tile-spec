# Implementation Status

## Integrations

| Integration | Status |
|---|---|
| [MapLibre Native](https://maplibre.org/maplibre-native/) | Supported since MapLibre Android 12.1.0, MapLibre iOS 6.2.0. PR: [#3246](https://github.com/maplibre/maplibre-native/pull/3246). |
| [MapLibre GL JS](https://maplibre.org/maplibre-gl-js/) | Supported since version 5.12.0. PR: [#6570](https://github.com/maplibre/maplibre-gl-js/pull/6570). |
| [Planetiler](https://github.com/onthegomap/planetiler) | Can generate tiles in MLT since version [0.10.0](https://github.com/onthegomap/planetiler/releases/tag/v0.10.0). |
| [PMTiles](https://github.com/protomaps/pmtiles) | Can store MLT, see [PMTiles v3 spec - Tile Type](https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md#tile-type-tt). |
| [Martin tile server](https://maplibre.org/martin/) | Can detect and serve MLT since version [1.3.0](https://github.com/maplibre/martin/releases/tag/martin-v1.3.0). PR: [#2512](https://github.com/maplibre/martin/pull/2512).<br>Can automatically convert between MLT and MVT depending on headers since [1.9.0](https://github.com/maplibre/martin/releases/tag/martin-v1.9.0). [docs](https://maplibre.org/martin/using-guides/mlt/) |
| [QGIS plugin](https://github.com/maplibre/maplibre-tile-spec/tree/main/qgis) | Opens MLT files in QGIS via the Rust `mlt` parser. See [qgis/README.md](https://github.com/maplibre/maplibre-tile-spec/blob/main/qgis/README.md). |
| [visgl/loaders.gl](https://loaders.gl/docs/modules/mlt/formats/mlt) and [module](https://loaders.gl/docs/modules/mlt) | Decodes MLT tilesets, supported since version 4.4 via pre-bundled mlt-decoder.cjs |

## Implementations

| Implementation | Language | Type | Notes |
|---|---|---|---|
| [maplibre-tile-spec/cpp](https://github.com/maplibre/maplibre-tile-spec/tree/main/cpp) | C++ | Decoder | Used by MapLibre Native |
| [maplibre-tile-spec/js](https://github.com/maplibre/maplibre-tile-spec/tree/main/js) | TypeScript | Decoder | Used by MapLibre GL JS |
| [maplibre-tile-spec/java](https://github.com/maplibre/maplibre-tile-spec/tree/main/java) | Java | Encoder | |
| [maplibre-tile-spec/rust/mlt](https://github.com/maplibre/maplibre-tile-spec/blob/main/rust/mlt/README.md) | Rust | Decoder / Encoder | Includes bundled `mlt` CLI, TUI, and conversion tooling. |
| [maplibre-tile-spec/rust/mlt-core](https://github.com/maplibre/maplibre-tile-spec/blob/main/rust/mlt-core/README.md) | Rust | Decoder / Encoder | Used in Martin. Includes bundled `mlt` CLI, TUI, and conversion tooling. |
| [maplibre-tile-spec/rust/mlt-wasm](https://github.com/maplibre/maplibre-tile-spec/tree/main/rust/mlt-wasm) | Rust / WASM | Decoder | Uses `mlt-core` |
| [maplibre-tile-spec/rust/mlt-py](https://github.com/maplibre/maplibre-tile-spec/tree/main/rust/mlt-py) | Rust / [PyO3](https://pyo3.rs/). | Decoder / Encoder | Exposes `mlt-core` to the python communtiy via [PyO3](https://pyo3.rs/). |
| [maplibre-tile-spec/rust/mlt-ffi](https://github.com/maplibre/maplibre-tile-spec/tree/main/rust/mlt-ffi) | Rust / FFI | Decoder / Encoder | Foreign-function bindings for `mlt-core` to the java/kotlin, cpp and c community |

## Ongoing work — tools adding MLT support

| Tool | Status | Tracking issue | Dependency / notes |
|---|---|---|---|
| [felt/tippecanoe](https://github.com/felt/tippecanoe) | Landing soon? | https://github.com/felt/tippecanoe/issues/380 | Via C++ encoder |
| [systemed/tilemaker](https://github.com/systemed/tilemaker) | Landing soon? | https://github.com/systemed/tilemaker/issues/856 | Via C++ encoder |
| [georust/geozero](https://github.com/georust/geozero) | Tracking | https://github.com/georust/geozero/issues/227 | Awaiting Rust encoder |
| [developmentseed/tipg](https://github.com/developmentseed/tipg) | Tracking | https://github.com/developmentseed/tipg/issues/248 | Awaiting hypothetical `ST_AsMLT` in PostGIS |
| [versatiles-org/versatiles-rs](https://github.com/versatiles-org/versatiles-rs) | Tracking | https://github.com/versatiles-org/versatiles-rs/issues/111 and https://github.com/versatiles-org/versatiles-spec/issues/11 | Rust tiler |
| [maptiler/tileserver-gl](https://github.com/maptiler/tileserver-gl) | Tracking | https://github.com/maptiler/tileserver-gl/issues/2194 | js + Go codebase |
| NASA/CesiumGS renderers | Mention |  https://github.com/NASA-AMMOS/3DTilesRendererJS/issues/982#issuecomment-4467367202 and https://github.com/CesiumGS/cesium/issues/2132 | Minor feature requests in MVT threads |

## Existing/Upcoming MLT datasets

| Dataset | Endpoint / Link | Notes |
|---|---|---|
| [maplibre/demotiles](https://github.com/maplibre/demotiles) | `demotiles.maplibre.org/tiles-mlt/plain/{z}/{x}/{y}.mlt` | Also available as TileJSON |
| [Overture Maps](https://github.com/OvertureMaps/overture-tiles/) | https://github.com/OvertureMaps/overture-tiles/issues/50 | Consideration for switching to MLT |
| [Shademap](https://ted-piotrowski.github.io/mapbox-gl-shadow-simulator/examples/maplibre.html) | `cfw.shademap.app/planet/{z}/{x}/{y}.mlt` and `cfw.shademap.app/buildings/{z}/{x}/{y}.mlt` | Buildings and planet layers |
| [OpenRailwayMap-vector](https://github.com/hiddewie/OpenRailwayMap-vector) | https://github.com/hiddewie/OpenRailwayMap-vector/issues/919 | MLT support tracking issue |
