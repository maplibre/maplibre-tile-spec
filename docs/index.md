---
description: A column-oriented vector tile format for map data
---

# MapLibre Tile Specification

MLT (MapLibre Tile) is a vector tile format for map data. It stores the same content as an [MVT](https://github.com/mapbox/vector-tile-spec) tile, layers of features with geometries and properties, in a column-oriented layout.
MLT is natively supported by [MapLibre GL JS](https://maplibre.org/maplibre-gl-js/) and [MapLibre Native](https://maplibre.org/maplibre-native/), and tiles can be served using the [Martin tile server](https://maplibre.org/martin/).

## Why MLT

MLT is mainly inspired by MVT, but has been redesigned from the ground up to improve the following areas:

- **Improved compression ratio** - a planet of MLT v2 tiles is 4-7% smaller than gzipped MVT (10-19% with gzip on top), and its largest tiles are 1.6-2.2x smaller than uncompressed MVT ([Size](#size)), based on a column oriented layout with (custom) lightweight encodings
- **Better decoding performance** - fast lightweight encodings which can be used in combination with SIMD/vectorization instructions
- **Support for linear referencing and m-values** to efficiently support the upcoming next generation source formats such as Overture Maps (GeoParquet)
- **Support 3D coordinates**, i.e. elevation
- **Support complex types**, including nested properties, lists and maps
- **Improved processing performance**: Based on an in-memory format that can be processed efficiently on the CPU and GPU and loaded directly into GPU buffers partially (like polygons in WebGL) or completely (in case of WebGPU compute shader usage) without additional processing

The last three are not yet implemented.
They are planned for [MLT v2](specification/v2.md).

## Size

Whole-planet archives, as a share of the same tiles stored as gzipped MVT.

=== "OpenMapTiles"

    --8<-- "diagrams/planet-size-omt.svg"

    | | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
    |---|---:|---:|---:|---:|---:|
    | Planet | 50.5 GB | 51.7 GB | 43.1 GB | 46.7 GB | 41.2 GB |
    | Median tile | 422 B | 524 B | 428 B | 442 B | 392 B |
    | 99th percentile tile | 18.4 kB | 16.4 kB | 14.0 kB | 15.4 kB | 13.6 kB |
    | Largest tile | 727.9 kB | 570.5 kB | 497.6 kB | 550.3 kB | 486.1 kB |

    The median tile among the largest 1% (from 31 kB as MVT) is this many times smaller:

    | | vs MVT | vs MVT + gzip |
    |---|---:|---:|
    | MLT v1 | 2.13x | 1.21x |
    | MLT v2 | 2.24x | 1.27x |

    Planet size with one `mlt convert` option added, relative to the same MLT version without it (no gzip, `-` where only v2 has the option):

    | Added option | MLT v1 | MLT v2 |
    |---|---:|---:|
    | `--no-shared-dict` | +25.4% | +25.3% |
    | `--no-fastpfor` | +2.6% | +3.1% |
    | `--no-fsst` | +0.7% | +0.8% |
    | `--sort none` | +0.2% | +0.2% |
    | `--sort all` | -0.4% | -0.4% |
    | `--no-alp` | - | +0.0% |
    | `--no-float-dict` | - | +0.0% |
    | `--packed-dict-codes` | - | -0.7% |
    | `--delta2` | - | -0.5% |
    | `--rans-vertices` | - | -5.4% |
    | `--tessellate` | +21.9% | +24.9% |
    | `--tessellate --triangles-only` | - | +23.2% |

=== "Protomaps"

    --8<-- "diagrams/planet-size-protomaps.svg"

    | | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
    |---|---:|---:|---:|---:|---:|
    | Planet | 138.2 GB | 145.9 GB | 131.1 GB | 133.1 GB | 124.7 GB |
    | Median tile | 357 B | 415 B | 371 B | 350 B | 337 B |
    | 99th percentile tile | 12.3 kB | 11.9 kB | 10.8 kB | 11.4 kB | 10.6 kB |
    | Largest tile | 485.9 kB | 473.1 kB | 450.8 kB | 473.9 kB | 449.9 kB |

    The median tile among the largest 1% (from 18 kB as MVT) is this many times smaller:

    | | vs MVT | vs MVT + gzip |
    |---|---:|---:|
    | MLT v1 | 1.58x | 1.05x |
    | MLT v2 | 1.65x | 1.10x |

    Planet size with one `mlt convert` option added, relative to the same MLT version without it (no gzip, `-` where only v2 has the option):

    | Added option | MLT v1 | MLT v2 |
    |---|---:|---:|
    | `--no-shared-dict` | +0.8% | +0.8% |
    | `--no-fastpfor` | +2.9% | +3.4% |
    | `--no-fsst` | +0.2% | +0.2% |
    | `--sort none` | +0.1% | +0.1% |
    | `--sort all` | -0.1% | -0.1% |
    | `--no-alp` | - | +0.1% |
    | `--no-float-dict` | - | +0.0% |
    | `--packed-dict-codes` | - | -0.3% |
    | `--delta2` | - | -0.6% |
    | `--rans-vertices` | - | -5.3% |
    | `--tessellate` | +32.4% | +35.8% |
    | `--tessellate --triangles-only` | - | +33.5% |

The [Protomaps](https://protomaps.com/) basemap build of 2026-10-02 (z0-15, 136 M distinct tiles) and the [OpenMapTiles](https://openmaptiles.org/) 3.11 planet by MapTiler of 2020-02-10 (z0-14, 34 M distinct tiles).
The `mlt convert` at [`89ceaef3`](https://github.com/maplibre/maplibre-tile-spec/commit/89ceaef3c49a06c8e0c253c40d8500c38b60fd88) was used.
Bytes of tile in the archive are only counted once.
For re-encoding we use Zlib level 6 per tile, since this is the level both source archives were written with.

## Documentation

--8<-- "live-spec-note"

| Page | Contents |
|---|---|
| [Data Model](overview.md) | Tiles, extents, layers, features, columns and streams. |
| [Specification v1](specification/v1.md) | The stable wire format. |
| [Specification v2](specification/v2.md) <span class="experimental"></span> | The next wire format, under development. |
| [Encoding Algorithms](encodings.md) | The compression schemes used by both versions. |
| [Implementation Guide](implementation-guide.md) | Choosing encodings and decoding into memory. |
| [Implementation Status](implementation-status.md) | Tools, libraries and datasets that support MLT. |
