# MapLibre Tile Specification

MLT (MapLibre Tile) is a vector tile format that extends [MVT](<https://github.com/mapbox/vector-tile-spec>), storing geometries and attributes as separate columns for improved compression and richer data support. MLT is natively supported by [MapLibre GL JS](<https://maplibre.org/maplibre-gl-js/>) and [MapLibre Native](<https://maplibre.org/maplibre-native/>), and tiles can be served using the [Martin tile server](<https://maplibre.org/martin/>).

## Why MLT

MLT is redesigned from the ground up for efficiency. Each geometry and attribute is stored as a separate column, with compression optimized for its data type. This makes tiles smaller, fast to decode with SIMD, and efficient to load into GPU buffers. On whole-planet archives, **uncompressed** MLT v2 is 4-8% smaller than gzipped MVT and decodes 2.3 to 3.7 times faster. With gzip, v2 is 10-18% smaller than gzipped MVT. See [benchmarks](<https://maplibre.org/maplibre-tile-spec/benchmarks/index.md>) for details.

MVT limits a feature to 2D vertices and single-value properties. Elevation and per-vertex values like lane count are lost, and lists or maps get flattened into columns like `name:en` and `name:de`. [MLT v2](<https://maplibre.org/maplibre-tile-spec/specification/v2/index.md>) removes these limits; see [how v2 differs from v1](<https://maplibre.org/maplibre-tile-spec/specification/v2/#differences-from-v1>):

Each layer in a tile carries its own [format version](<https://maplibre.org/maplibre-tile-spec/overview/#frames>), so new formats can be added without breaking existing readers. One tile can mix them, so future formats can add 3D and other kinds of data, including non-visual data, alongside existing layers.

> [!TIP]
>
> **MLT 0x03**
>
> A [3D extension to MLT](<https://github.com/maplibre/maplibre-tile-spec/discussions/1182>) will expand these capabilities, and is in the requirements engineering stage. Join the MapLibre [Slack](<https://maplibre.org/community/>) to learn more and share your ideas.

## Documentation

> [!NOTE]
>
> This is a live specification that evolves continuously. Features marked as are under active development and may change in future versions. Stable features are those without experimental tags.

| Page | Contents |
| --- | --- |
| [Data Model](<https://maplibre.org/maplibre-tile-spec/overview/index.md>) | How tiles, frames, layers, features, columns and streams fit together. Start here. |
| [MLT v1](<https://maplibre.org/maplibre-tile-spec/specification/v1/index.md>) | The stable wire format: 2D features with scalar properties, compatible with MVT. |
| [MLT v2](<https://maplibre.org/maplibre-tile-spec/specification/v2/index.md>) | The next wire format: v1 content in a smaller layout, plus Z and M values and nested properties. |
| [Encoding Algorithms](<https://maplibre.org/maplibre-tile-spec/encodings/index.md>) | The compression schemes used by both versions. |
| [Implementation Guide](<https://maplibre.org/maplibre-tile-spec/implementation-guide/index.md>) | Choosing encodings and decoding into memory. |
| [Implementation Status](<https://maplibre.org/maplibre-tile-spec/implementation-status/index.md>) | Tools, libraries and datasets that support MLT. |
| [Benchmarks](<https://maplibre.org/maplibre-tile-spec/benchmarks/index.md>) | Decoding speed and tile size compared with MVT. |
| [Tile Inspector](<https://maplibre.org/maplibre-tile-spec/inspector/index.md>) | Decode a tile in the browser and walk through its bytes. |
