# Introduction

The MLT format is mainly inspired by the [MVT format](https://github.com/mapbox/vector-tile-spec), but has been redesigned from the ground up to improve the following areas:

- **Improved compression ratio** - up to 6x on large tiles, based on a column oriented layout with (custom) lightweight encodings
- **Better decoding performance** - fast lightweight encodings which can be used in combination with SIMD/vectorization instructions
- **Support for linear referencing and m-values** to efficiently support the upcoming next generation source formats such as Overture Maps (GeoParquet)
- **Support 3D coordinates**, i.e. elevation
- **Support complex types**, including nested properties, lists and maps
- **Improved processing performance**: Based on an in-memory format that can be processed efficiently on the CPU and GPU and loaded directly into GPU buffers partially (like polygons in WebGL) or completely (in case of WebGPU compute shader usage) without additional processing
