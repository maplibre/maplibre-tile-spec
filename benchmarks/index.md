# Benchmarks

Measured on whole-planet [OpenMapTiles](<https://openmaptiles.org/>) and [Protomaps](<https://protomaps.com/>) archives, described in [datasets](<#datasets>).

## Decoding speed

Time to decode every property and geometry of the same sampled tiles, as a share of the time for gzipped MVT. Tiles are sampled at up to 5,000 per zoom level timed as the best of 15 runs on one core of an AMD Ryzen 9 3900. MVT is read with [fast-mvt](<https://crates.io/crates/fast-mvt>), the fastest known MVT library, and MLT with `decode_all`, so no "unfair" gains from lazy per-attribute decoding. Performance depends on the specifics of the tile source.

| Decode time | MVT | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| All tiles | 64% | 100% | 40% | 65% | 44% | 68% |
| Median tile | 40% | 100% | 52% | 117% | 55% | 115% |
| Largest 1% of tiles | 70% | 100% | 26% | 42% | 28% | 43% |

Decode time with one `mlt convert` option added, relative to the same MLT version without it (`-` where only v2 has the option):

| Added option | MLT v1 | MLT v2 |
| --- | ---: | ---: |
| `--no-shared-dict` | \+15% | \+27% |
| `--no-fastpfor` | \+12% | \+11% |
| `--no-fsst` | \-2% | \-3% |
| `--sort none` | \+1% | 0% |
| `--sort all` | \+1% | \+1% |
| `--no-alp` | \- | 0% |
| `--no-float-dict` | \- | 0% |
| `--no-bitpacking` | \- | \+1% |
| `--delta2` | \- | 0% |
| `--rans-vertices` | \- | \+16% |
| `--tessellate` | \+27% | \+14% |
| `--tessellate --triangles-only` | \- | \+10% |

| Decode time | MVT | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| All tiles | 55% | 100% | 27% | 55% | 27% | 55% |
| Median tile | 37% | 100% | 49% | 113% | 49% | 108% |
| Largest 1% of tiles | 56% | 100% | 18% | 39% | 19% | 40% |

Decode time with one `mlt convert` option added, relative to the same MLT version without it (`-` where only v2 has the option):

| Added option | MLT v1 | MLT v2 |
| --- | ---: | ---: |
| `--no-shared-dict` | \+1% | \+2% |
| `--no-fastpfor` | \+15% | \+15% |
| `--no-fsst` | 0% | \-1% |
| `--sort none` | \+1% | \+1% |
| `--sort all` | \+1% | \+1% |
| `--no-alp` | \- | 0% |
| `--no-float-dict` | \- | 0% |
| `--no-bitpacking` | \- | 0% |
| `--delta2` | \- | \+1% |
| `--rans-vertices` | \- | \+88% |
| `--tessellate` | \+145% | \+68% |
| `--tessellate --triangles-only` | \- | \+59% |

## Size

Whole-planet archives, as a share of the same tiles stored as gzipped MVT.

|  | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
| --- | ---: | ---: | ---: | ---: | ---: |
| Planet | 50.5 GB | 51.7 GB | 43.1 GB | 46.7 GB | 41.2 GB |
| Median tile | 422 B | 524 B | 428 B | 442 B | 392 B |
| 99th percentile tile | 18.4 kB | 16.4 kB | 14.0 kB | 15.4 kB | 13.6 kB |
| Largest tile | 727.9 kB | 570.5 kB | 497.6 kB | 550.3 kB | 486.1 kB |

The median tile among the largest 1% (from 31 kB as MVT) is this many times smaller:

|  | vs MVT | vs MVT + gzip |
| --- | ---: | ---: |
| MLT v1 | 2.13x | 1.21x |
| MLT v2 | 2.24x | 1.27x |

Planet size with one `mlt convert` option added, relative to the same MLT version without it (no gzip, `-` where only v2 has the option):

| Added option | MLT v1 | MLT v2 |
| --- | ---: | ---: |
| `--no-shared-dict` | \+25.4% | \+25.3% |
| `--no-fastpfor` | \+2.6% | \+3.1% |
| `--no-fsst` | \+0.7% | \+0.8% |
| `--sort none` | \+0.2% | \+0.2% |
| `--sort all` | \-0.4% | \-0.4% |
| `--no-alp` | \- | \+0.0% |
| `--no-float-dict` | \- | \+0.0% |
| `--no-bitpacking` | \- | \+0.7% |
| `--delta2` | \- | \-0.5% |
| `--rans-vertices` | \- | \-5.4% |
| `--tessellate` | \+21.9% | \+24.9% |
| `--tessellate --triangles-only` | \- | \+23.2% |

|  | MVT + gzip | MLT v1 | MLT v1 + gzip | MLT v2 | MLT v2 + gzip |
| --- | ---: | ---: | ---: | ---: | ---: |
| Planet | 138.2 GB | 145.9 GB | 131.1 GB | 133.1 GB | 124.7 GB |
| Median tile | 357 B | 415 B | 371 B | 350 B | 337 B |
| 99th percentile tile | 12.3 kB | 11.9 kB | 10.8 kB | 11.4 kB | 10.6 kB |
| Largest tile | 485.9 kB | 473.1 kB | 450.8 kB | 473.9 kB | 449.9 kB |

The median tile among the largest 1% (from 18 kB as MVT) is this many times smaller:

|  | vs MVT | vs MVT + gzip |
| --- | ---: | ---: |
| MLT v1 | 1.58x | 1.05x |
| MLT v2 | 1.65x | 1.10x |

Planet size with one `mlt convert` option added, relative to the same MLT version without it (no gzip, `-` where only v2 has the option):

| Added option | MLT v1 | MLT v2 |
| --- | ---: | ---: |
| `--no-shared-dict` | \+0.8% | \+0.8% |
| `--no-fastpfor` | \+2.9% | \+3.4% |
| `--no-fsst` | \+0.2% | \+0.2% |
| `--sort none` | \+0.1% | \+0.1% |
| `--sort all` | \-0.1% | \-0.1% |
| `--no-alp` | \- | \+0.1% |
| `--no-float-dict` | \- | \+0.0% |
| `--no-bitpacking` | \- | \+0.3% |
| `--delta2` | \- | \-0.6% |
| `--rans-vertices` | \- | \-5.3% |
| `--tessellate` | \+32.4% | \+35.8% |
| `--tessellate --triangles-only` | \- | \+33.5% |

## Datasets

The [Protomaps](<https://protomaps.com/>) basemap build of 2026-10-02 (z0-15, 136 M distinct tiles) and the [OpenMapTiles](<https://openmaptiles.org/>) 3.11 planet by MapTiler of 2020-02-10 (z0-14, 34 M distinct tiles). The `mlt convert` at [`89ceaef3`](<https://github.com/maplibre/maplibre-tile-spec/commit/89ceaef3c49a06c8e0c253c40d8500c38b60fd88>) was used. Bytes of tile in the archive are only counted once. For re-encoding we use Zlib level 6 per tile, since this is the level both source archives were written with.
