# Implementation Status

## Integrations

| Integration | Status |
|---|---|
| MapLibre Native | Supported since MapLibre Android 12.1.0, MapLibre iOS 6.2.0. PR: [#3246](https://github.com/maplibre/maplibre-native/pull/3246). |
| MapLibre GL JS | Supported since version 5.12.0. PR: [#6570](https://github.com/maplibre/maplibre-gl-js/pull/6570). |
| Planetiler | Can generate tiles in MLT since version [0.10.0](https://github.com/onthegomap/planetiler/releases/tag/v0.10.0). |
| PMTiles | Can store MLT, see [PMTiles v3 spec - Tile Type](https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md#tile-type-tt). |
| Martin Tileserver | Can detect and serve MLT since version [1.3.0](https://github.com/maplibre/martin/releases/tag/martin-v1.3.0). PR: [#2512](https://github.com/maplibre/martin/pull/2512) |

## Implementations

| Implementation | Language | Type | Notes |
|---|---|---|---|
| [maplibre-tile-spec/cpp](https://github.com/maplibre/maplibre-tile-spec/tree/main/cpp) | C++ | Decoder | Used by MapLibre Native |
| [maplibre-tile-spec/js](https://github.com/maplibre/maplibre-tile-spec/tree/main/js) | TypeScript | Decoder | Used by MapLibre GL JS |
| [maplibre-tile-spec/java](https://github.com/maplibre/maplibre-tile-spec/tree/main/java) | Java | Encoder | |
| [maplibre-tile-spec/rust/mlt](https://github.com/maplibre/maplibre-tile-spec/tree/main/rust/mlt) | Rust | Decoder / Encoder | |
