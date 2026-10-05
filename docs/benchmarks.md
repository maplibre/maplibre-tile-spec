---
description: Decoding speed and tile size of MLT v1 and v2 compared with MVT
---

<style>
  /* One dataset switch for the whole page, kept in view while its tables scroll by. */
  .md-typeset .tabbed-set > .tabbed-labels {
    position: sticky;
    top: 2.4rem;
    z-index: 2;
    background-color: var(--md-default-bg-color);
    transition: top 250ms;
  }
  body:has(.md-header[hidden]) .md-typeset .tabbed-set > .tabbed-labels { top: 0; }
</style>

# Benchmarks

MLT v1 and v2 compared with MVT on whole-planet archives.
Pick a dataset below; the [method](#method) applies to all of them.

=== "OpenMapTiles"

    #### Decoding speed

    Time to decode every property and geometry, as a share of the time for gzipped MVT.

    --8<-- "diagrams/decode-speed-omt.svg"

    | Decode time | MVT | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
    |---|---:|---:|---:|---:|---:|---:|
    | All tiles | 64% | 100% | 40% | 65% | 44% | 68% |
    | Median tile | 40% | 100% | 52% | 117% | 55% | 115% |
    | Largest 1% of tiles | 70% | 100% | 26% | 42% | 28% | 43% |

    #### Size

    Whole-planet archive size, as a share of gzipped MVT.

    --8<-- "diagrams/planet-size-omt.svg"

    | | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
    |---|---:|---:|---:|---:|---:|
    | Planet | 50.5 GB | 51.7 GB | 43.1 GB | 46.7 GB | 41.2 GB |
    | Median tile | 422 B | 524 B | 428 B | 442 B | 392 B |
    | 99th percentile tile | 18.4 kB | 16.4 kB | 14.0 kB | 15.4 kB | 13.6 kB |
    | Largest tile | 727.9 kB | 570.5 kB | 497.6 kB | 550.3 kB | 486.1 kB |

    Large tiles gain the most.
    The median of the largest 1% of tiles is 31 kB as MVT, and MLT stores it this many times smaller:

    | | vs MVT | vs MVT + gzip |
    |---|---:|---:|
    | MLT v1 | 2.13x | 1.21x |
    | MLT v2 | 2.24x | 1.27x |

    #### Encoder options

    Planet size and decode time with one `mlt convert` option added, relative to the same MLT version without it.
    Size is measured without gzip, and `-` marks an option only v2 has.

    | Added option | Size v1 | Size v2 | Decode v1 | Decode v2 |
    |---|---:|---:|---:|---:|
    | `--no-shared-dict` | +25.4% | +25.3% | +15% | +27% |
    | `--no-fastpfor` | +2.6% | +3.1% | +12% | +11% |
    | `--no-fsst` | +0.7% | +0.8% | -2% | -3% |
    | `--sort none` | +0.2% | +0.2% | +1% | 0% |
    | `--sort all` | -0.4% | -0.4% | +1% | +1% |
    | `--no-alp` | - | +0.0% | - | 0% |
    | `--no-float-dict` | - | +0.0% | - | 0% |
    | `--no-bitpacking` | - | +0.7% | - | +1% |
    | `--delta2` | - | -0.5% | - | 0% |
    | `--rans-vertices` | - | -5.4% | - | +16% |
    | `--tessellate` | +21.9% | +24.9% | +27% | +14% |
    | `--tessellate --triangles-only` | - | +23.2% | - | +10% |

=== "Protomaps"

    #### Decoding speed

    Time to decode every property and geometry, as a share of the time for gzipped MVT.

    --8<-- "diagrams/decode-speed-protomaps.svg"

    | Decode time | MVT | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
    |---|---:|---:|---:|---:|---:|---:|
    | All tiles | 55% | 100% | 27% | 55% | 27% | 55% |
    | Median tile | 37% | 100% | 49% | 113% | 49% | 108% |
    | Largest 1% of tiles | 56% | 100% | 18% | 39% | 19% | 40% |

    #### Size

    Whole-planet archive size, as a share of gzipped MVT.

    --8<-- "diagrams/planet-size-protomaps.svg"

    | | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
    |---|---:|---:|---:|---:|---:|
    | Planet | 138.2 GB | 145.9 GB | 131.1 GB | 133.1 GB | 124.7 GB |
    | Median tile | 357 B | 415 B | 371 B | 350 B | 337 B |
    | 99th percentile tile | 12.3 kB | 11.9 kB | 10.8 kB | 11.4 kB | 10.6 kB |
    | Largest tile | 485.9 kB | 473.1 kB | 450.8 kB | 473.9 kB | 449.9 kB |

    Large tiles gain the most.
    The median of the largest 1% of tiles is 18 kB as MVT, and MLT stores it this many times smaller:

    | | vs MVT | vs MVT + gzip |
    |---|---:|---:|
    | MLT v1 | 1.58x | 1.05x |
    | MLT v2 | 1.65x | 1.10x |

    #### Encoder options

    Planet size and decode time with one `mlt convert` option added, relative to the same MLT version without it.
    Size is measured without gzip, and `-` marks an option only v2 has.

    | Added option | Size v1 | Size v2 | Decode v1 | Decode v2 |
    |---|---:|---:|---:|---:|
    | `--no-shared-dict` | +0.8% | +0.8% | +1% | +2% |
    | `--no-fastpfor` | +2.9% | +3.4% | +15% | +15% |
    | `--no-fsst` | +0.2% | +0.2% | 0% | -1% |
    | `--sort none` | +0.1% | +0.1% | +1% | +1% |
    | `--sort all` | -0.1% | -0.1% | +1% | +1% |
    | `--no-alp` | - | +0.1% | - | 0% |
    | `--no-float-dict` | - | +0.0% | - | 0% |
    | `--no-bitpacking` | - | +0.3% | - | 0% |
    | `--delta2` | - | -0.6% | - | +1% |
    | `--rans-vertices` | - | -5.3% | - | +88% |
    | `--tessellate` | +32.4% | +35.8% | +145% | +68% |
    | `--tessellate --triangles-only` | - | +33.5% | - | +59% |

## Method

Decoding is timed on the same tiles sampled from each archive, up to 5,000 per zoom level, as the best of 15 runs on one core of an AMD Ryzen 9 3900.
MVT is read with [fast-mvt](https://crates.io/crates/fast-mvt), the fastest known MVT library, and MLT with `decode_all`, so no "unfair" gains from lazy per-attribute decoding.
Performance depends on the specifics of the tile source.

Sizes cover whole archives, and bytes of a tile stored more than once in an archive are counted once.
Gzip is Zlib level 6 per tile, the level both source archives were written with.

### Datasets

The [Protomaps](https://protomaps.com/) basemap build of 2026-10-02 (z0-15, 136 M distinct tiles) and the [OpenMapTiles](https://openmaptiles.org/) 3.11 planet by MapTiler of 2020-02-10 (z0-14, 34 M distinct tiles).
The `mlt convert` at [`89ceaef3`](https://github.com/maplibre/maplibre-tile-spec/commit/89ceaef3c49a06c8e0c253c40d8500c38b60fd88) was used.
