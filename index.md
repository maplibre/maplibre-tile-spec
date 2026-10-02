# MapLibre Tile Specification

> [!NOTE]
>
> This is a live specification that evolves continuously. Features marked as are under active development and may change in future versions. Stable features are those without experimental tags.

MLT (MapLibre Tile) is a vector tile format for map data. It stores the same content as an [MVT](<https://github.com/mapbox/vector-tile-spec>) tile, layers of features with geometries and properties, in a column-oriented layout.

MLT is natively supported by [MapLibre GL JS](<https://maplibre.org/maplibre-gl-js/>) and [MapLibre Native](<https://maplibre.org/maplibre-native/>), and tiles can be served using the [Martin tile server](<https://maplibre.org/martin/>).

## Why MLT

MLT is mainly inspired by MVT, but has been redesigned from the ground up to improve the following areas:

- **Improved compression ratio** - up to 6x on large tiles, based on a column oriented layout with (custom) lightweight encodings
- **Better decoding performance** - fast lightweight encodings which can be used in combination with SIMD/vectorization instructions
- **Support for linear referencing and m-values** to efficiently support the upcoming next generation source formats such as Overture Maps (GeoParquet)
- **Support 3D coordinates**, i.e. elevation
- **Support complex types**, including nested properties, lists and maps
- **Improved processing performance**: Based on an in-memory format that can be processed efficiently on the CPU and GPU and loaded directly into GPU buffers partially (like polygons in WebGL) or completely (in case of WebGPU compute shader usage) without additional processing

The last three are not yet implemented. They are planned for [MLT v2](<https://maplibre.org/maplibre-tile-spec/specification/v2/index.md>).

## Documentation

| Page | Contents |
| --- | --- |
| [Overview](<https://maplibre.org/maplibre-tile-spec/overview/index.md>) | The data model: tiles, extents, layers, features, columns and streams. |
| [Specification v1](<https://maplibre.org/maplibre-tile-spec/specification/v1/index.md>) | The stable wire format. |
| [Specification v2](<https://maplibre.org/maplibre-tile-spec/specification/v2/index.md>) | The next wire format, under development. |
| [Encoding Algorithms](<https://maplibre.org/maplibre-tile-spec/encodings/index.md>) | The compression schemes used by both versions. |
| [Implementation Guide](<https://maplibre.org/maplibre-tile-spec/implementation-guide/index.md>) | Choosing encodings and decoding into memory. |
| [Implementation Status](<https://maplibre.org/maplibre-tile-spec/implementation-status/index.md>) | Tools, libraries and datasets that support MLT. |
