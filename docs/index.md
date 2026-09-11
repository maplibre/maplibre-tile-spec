# MapLibre Tile Specification

--8<-- "live-spec-note"

MLT (MapLibre Tile) is a vector tile format for map data.
It stores the same content as an [MVT](https://github.com/mapbox/vector-tile-spec) tile, layers of features with geometries and properties, in a column-oriented layout.

MLT is natively supported by [MapLibre GL JS](https://maplibre.org/maplibre-gl-js/) and [MapLibre Native](https://maplibre.org/maplibre-native/), and tiles can be served using the [Martin tile server](https://maplibre.org/martin/).

## Why MLT

MLT is mainly inspired by MVT, but has been redesigned from the ground up to improve the following areas:

- **Improved compression ratio** - up to 6x on large tiles, based on a column oriented layout with (custom) lightweight encodings
- **Better decoding performance** - fast lightweight encodings which can be used in combination with SIMD/vectorization instructions
- **Support for linear referencing and m-values** to efficiently support the upcoming next generation source formats such as Overture Maps (GeoParquet)
- **Support 3D coordinates**, i.e. elevation
- **Support complex types**, including nested properties, lists and maps
- **Improved processing performance**: Based on an in-memory format that can be processed efficiently on the CPU and GPU and loaded directly into GPU buffers partially (like polygons in WebGL) or completely (in case of WebGPU compute shader usage) without additional processing

The last three are not yet implemented.
They are planned for [MLT v2](specification-v2.md), see [Planned features](specification-v2.md#planned-features).

## Documentation

| Page | Contents |
|---|---|
| [Overview](overview.md) | The data model: tiles, extents, layers, features, columns and streams. |
| [Specification v1](specification.md) | The stable wire format. |
| [Specification v2](specification-v2.md) <span class="experimental"></span> | The next wire format, under development. |
| [Encoding Algorithms](encodings.md) | The compression schemes used by both versions. |
| [Implementation Status](implementation-status.md) | Tools, libraries and datasets that support MLT. |
