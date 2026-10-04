---
description: A column-oriented vector tile format for map data
---

# MapLibre Tile Specification

MLT (MapLibre Tile) is a vector tile format for map data. It stores the same content as an [MVT](https://github.com/mapbox/vector-tile-spec) tile, layers of features with geometries and properties, in a column-oriented layout.
MLT is natively supported by [MapLibre GL JS](https://maplibre.org/maplibre-gl-js/) and [MapLibre Native](https://maplibre.org/maplibre-native/), and tiles can be served using the [Martin tile server](https://maplibre.org/martin/).

## Why MLT

MLT is mainly inspired by MVT, but has been redesigned from the ground up.
It stores each column on its own, so each gets the lightweight encoding that fits it.
That makes tiles smaller, see [Size](#size), fast to decode with SIMD, and cheap to load into GPU buffers.

--8<-- "diagrams/why-mlt.svg"

MVT limits a feature to 2D vertices and single-value properties.
Elevation and per-vertex values like lane count are lost, and lists or maps get flattened into columns like `name:en` and `name:de`.
[MLT v2](specification/v2.md) removes these limits:

--8<-- "diagrams/v2-feature.svg"

!!! tip "MLT 0x03"
    MLT 0x03, a 3D extension that expands these capabilities, is in the requirements engineering stage.
    Join the MapLibre [Slack](https://maplibre.org/community/) to learn more and share your ideas.

## Decoding speed

Time to decode every property and geometry of the same sampled tiles, as a share of the time for gzipped MVT.
Tiles are sampled at up to 5,000 per zoom level timed as the best of 15 runs on one core of an AMD Ryzen 9 3900.
MVT is read with [fast-mvt](https://crates.io/crates/fast-mvt) (the fastest known MVT library) and MLT with `decode_all` (so no "unfair" gains from being lazy).

=== "OpenMapTiles"

    --8<-- "diagrams/decode-speed-omt.svg"

    | Decode time | MVT | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
    |---|---:|---:|---:|---:|---:|---:|
    | All tiles | 64% | 100% | 40% | 65% | 44% | 68% |
    | Median tile | 40% | 100% | 52% | 117% | 55% | 115% |
    | Largest 1% of tiles | 70% | 100% | 26% | 42% | 28% | 43% |

    Decode time with one `mlt convert` option added, relative to the same MLT version without it (`-` where only v2 has the option):

    | Added option | MLT v1 | MLT v2 |
    |---|---:|---:|
    | `--no-shared-dict` | +15% | +27% |
    | `--no-fastpfor` | +12% | +11% |
    | `--no-fsst` | -2% | -3% |
    | `--sort none` | +1% | 0% |
    | `--sort all` | +1% | +1% |
    | `--no-alp` | - | 0% |
    | `--no-float-dict` | - | 0% |
    | `--packed-dict-codes` | - | -1% |
    | `--delta2` | - | 0% |
    | `--rans-vertices` | - | +16% |
    | `--tessellate` | +27% | +14% |
    | `--tessellate --triangles-only` | - | +10% |

=== "Protomaps"

    --8<-- "diagrams/decode-speed-protomaps.svg"

    | Decode time | MVT | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
    |---|---:|---:|---:|---:|---:|---:|
    | All tiles | 55% | 100% | 27% | 55% | 27% | 55% |
    | Median tile | 37% | 100% | 49% | 113% | 49% | 108% |
    | Largest 1% of tiles | 56% | 100% | 18% | 39% | 19% | 40% |

    Decode time with one `mlt convert` option added, relative to the same MLT version without it (`-` where only v2 has the option):

    | Added option | MLT v1 | MLT v2 |
    |---|---:|---:|
    | `--no-shared-dict` | +1% | +2% |
    | `--no-fastpfor` | +15% | +15% |
    | `--no-fsst` | 0% | -1% |
    | `--sort none` | +1% | +1% |
    | `--sort all` | +1% | +1% |
    | `--no-alp` | - | 0% |
    | `--no-float-dict` | - | 0% |
    | `--packed-dict-codes` | - | 0% |
    | `--delta2` | - | +1% |
    | `--rans-vertices` | - | +88% |
    | `--tessellate` | +145% | +68% |
    | `--tessellate --triangles-only` | - | +59% |


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
