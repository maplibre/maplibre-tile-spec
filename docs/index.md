---
description: A column-oriented vector tile format for map data
---

# MapLibre Tile Specification

--8<-- "live-spec-note"

MLT (MapLibre Tile) is a vector tile format for map data.
It stores the same content as an [MVT](https://github.com/mapbox/vector-tile-spec) tile, layers of features with geometries and properties, in a column-oriented layout.

MLT is natively supported by [MapLibre GL JS](https://maplibre.org/maplibre-gl-js/) and [MapLibre Native](https://maplibre.org/maplibre-native/), and tiles can be served using the [Martin tile server](https://maplibre.org/martin/).

## Why MLT

MLT is mainly inspired by MVT, but has been redesigned from the ground up to improve the following areas:

- **Improved compression ratio** - a planet of MLT v2 tiles is 4–7% smaller than gzipped MVT, and 10–18% smaller with gzip on top ([Size](#size)), based on a column oriented layout with (custom) lightweight encodings
- **Better decoding performance** - fast lightweight encodings which can be used in combination with SIMD/vectorization instructions
- **Support for linear referencing and m-values** to efficiently support the upcoming next generation source formats such as Overture Maps (GeoParquet)
- **Support 3D coordinates**, i.e. elevation
- **Support complex types**, including nested properties, lists and maps
- **Improved processing performance**: Based on an in-memory format that can be processed efficiently on the CPU and GPU and loaded directly into GPU buffers partially (like polygons in WebGL) or completely (in case of WebGPU compute shader usage) without additional processing

The last three are not yet implemented.
They are planned for [MLT v2](specification/v2.md).

## Size

Whole-planet archives, as a share of the same tiles stored as gzipped MVT.

=== "Protomaps"

    --8<-- "diagrams/planet-size-protomaps.svg"

    | | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
    |---|---:|---:|---:|---:|---:|
    | Planet | 136.5 GB | 144.6 GB | 129.0 GB | 131.6 GB | 122.7 GB |
    | Median tile | 357 B | 419 B | 369 B | 351 B | 335 B |
    | 99th percentile tile | 11.3 kB | 10.8 kB | 9.8 kB | 10.4 kB | 9.6 kB |
    | Largest sampled tile | 387.0 kB | 378.9 kB | 355.8 kB | 376.8 kB | 355.2 kB |

=== "OpenMapTiles"

    --8<-- "diagrams/planet-size-omt.svg"

    | | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
    |---|---:|---:|---:|---:|---:|
    | Planet | 50.7 GB | 51.9 GB | 43.2 GB | 47.0 GB | 41.4 GB |
    | Median tile | 431 B | 538 B | 436 B | 454 B | 401 B |
    | 99th percentile tile | 17.7 kB | 15.9 kB | 13.5 kB | 14.9 kB | 13.1 kB |
    | Largest sampled tile | 289.7 kB | 199.1 kB | 168.2 kB | 194.9 kB | 167.4 kB |

Each option changes the planet size of its version's defaults by:

| `mlt convert` option | Protomaps v1 | Protomaps v2 | OpenMapTiles v1 | OpenMapTiles v2 |
|---|---:|---:|---:|---:|
| `--no-shared-dict` | +0.8% | +0.8% | +25.4% | +25.3% |
| `--no-fastpfor` | +2.9% | +3.4% | +2.6% | +3.1% |
| `--no-fsst` | +0.2% | +0.2% | +0.7% | +0.8% |
| `--sort none` | +0.1% | +0.1% | +0.2% | +0.2% |
| `--sort all` | -0.1% | -0.1% | -0.4% | -0.4% |
| `--no-alp` | — | +0.1% | — | +0.0% |
| `--no-float-dict` | — | +0.0% | — | +0.0% |
| `--packed-dict-codes` | — | -0.3% | — | -0.7% |
| `--delta2` | — | -0.6% | — | -0.5% |
| `--rans-vertices` | — | -5.3% | — | -5.4% |
| `--tessellate` | +32.4% | +35.8% | +21.9% | +24.9% |
| `--tessellate --triangles-only` | — | +33.5% | — | +23.2% |

- **Data:** the [Protomaps](https://protomaps.com/) basemap build of 2026-10-02 (z0–15, 136 M distinct tiles) and the [OpenMapTiles](https://openmaptiles.org/) 3.11 planet by MapTiler of 2020-02-10 (z0–14, 34 M distinct tiles).
- **Encoder:** `mlt convert --verify` at [`89ceaef3`](https://github.com/maplibre/maplibre-tile-spec/commit/89ceaef3c49a06c8e0c253c40d8500c38b60fd88), which decodes every layer and compares it to the source.
- **Sample:** up to 5,000 distinct tiles per zoom, drawn uniformly, and each zoom scaled by its distinct-tile count. The 95% bootstrap interval of each share is within ±2% for MLT and ±4.3% for uncompressed MVT.
- **gzip:** zlib level 6 per tile, the level both source archives were written with.

## Documentation

| Page | Contents |
|---|---|
| [Overview](overview.md) | The data model: tiles, extents, layers, features, columns and streams. |
| [Specification v1](specification/v1.md) | The stable wire format. |
| [Specification v2](specification/v2.md) <span class="experimental"></span> | The next wire format, under development. |
| [Encoding Algorithms](encodings.md) | The compression schemes used by both versions. |
| [Implementation Guide](implementation-guide.md) | Choosing encodings and decoding into memory. |
| [Implementation Status](implementation-status.md) | Tools, libraries and datasets that support MLT. |
