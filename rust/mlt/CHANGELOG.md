# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.23](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.22...rust-mlt-v0.1.23) - 2026-07-18

### Fixed

- *(rust)* preserve PMTiles geographic header ([#1503](https://github.com/maplibre/maplibre-tile-spec/pull/1503))

### Other

- *(rust)* update dependencies, use CARGO_BUILD_WARNINGS ([#1515](https://github.com/maplibre/maplibre-tile-spec/pull/1515))
- Add PMTiles compression support to the Rust MVT-to-MLT converter ([#1498](https://github.com/maplibre/maplibre-tile-spec/pull/1498))

## [0.1.22](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.21...rust-mlt-v0.1.22) - 2026-07-01

### Other

- update Cargo.toml dependencies

## [0.1.21](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.20...rust-mlt-v0.1.21) - 2026-06-29

### Other

- update Cargo.toml dependencies

## [0.1.20](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.19...rust-mlt-v0.1.20) - 2026-06-20

### Other

- update Cargo.lock dependencies

## [0.1.19](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.18...rust-mlt-v0.1.19) - 2026-06-19

### Other

- *(rust)* rm pub fields, builder pattern ([#1428](https://github.com/maplibre/maplibre-tile-spec/pull/1428))

## [0.1.18](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.17...rust-mlt-v0.1.18) - 2026-06-18

### Other

- update Cargo.lock dependencies

## [0.1.17](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.16...rust-mlt-v0.1.17) - 2026-06-15

### Added

- *(rust)* pmtiles to pmtiles transcoding ([#1411](https://github.com/maplibre/maplibre-tile-spec/pull/1411))
- *(py)* make `mlt.encode_geojson` configurable ([#1427](https://github.com/maplibre/maplibre-tile-spec/pull/1427))

### Other

- *(rust)* use mimalloc as the global allocator for the mlt CLI ([#1433](https://github.com/maplibre/maplibre-tile-spec/pull/1433))
- *(rust)* split convert into from_files/from_mbtiles/common and add --sort all ([#1434](https://github.com/maplibre/maplibre-tile-spec/pull/1434))

## [0.1.16](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.15...rust-mlt-v0.1.16) - 2026-06-13

### Fixed

- wire `allow_fastpfor` and `allow_fsst` through the mlt implementations ([#1431](https://github.com/maplibre/maplibre-tile-spec/pull/1431))

## [0.1.15](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.14...rust-mlt-v0.1.15) - 2026-06-08

### Fixed

- *(rust)* accept .pbf in convert ([#1417](https://github.com/maplibre/maplibre-tile-spec/pull/1417))

### Other

- *(rust)* use `usize_cast` for casting ([#1414](https://github.com/maplibre/maplibre-tile-spec/pull/1414))
- *(rust)* migrate to fast-mvt crate ([#1415](https://github.com/maplibre/maplibre-tile-spec/pull/1415))
- *(deps)* bump the all-cargo-version-updates group across 1 directory with 7 updates ([#1419](https://github.com/maplibre/maplibre-tile-spec/pull/1419))
- Provide mlt cli version ([#1406](https://github.com/maplibre/maplibre-tile-spec/pull/1406))

## [0.1.14](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.13...rust-mlt-v0.1.14) - 2026-05-15

### Other

- *(deps)* bump hotpath from 0.15.1 to 0.16.0 in /rust in the all-cargo-version-updates group across 1 directory ([#1380](https://github.com/maplibre/maplibre-tile-spec/pull/1380))

## [0.1.13](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.12...rust-mlt-v0.1.13) - 2026-05-14

### Added

- *(rust)* add .pmtiles output format to convert ([#1354](https://github.com/maplibre/maplibre-tile-spec/pull/1354))

## [0.1.12](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.11...rust-mlt-v0.1.12) - 2026-05-05

### Added

- *(rust)* MLT -> MVT write support ([#1369](https://github.com/maplibre/maplibre-tile-spec/pull/1369))

## [0.1.11](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.10...rust-mlt-v0.1.11) - 2026-04-29

### Added

- *(rust)* add secondary Hilbert dictionary sorting on select tiles ([#1349](https://github.com/maplibre/maplibre-tile-spec/pull/1349))

### Other

- *(rust)* IdValues→ParsedId+StagedId, simplify presence ([#1334](https://github.com/maplibre/maplibre-tile-spec/pull/1334))
- *(rust)* Improve the performance of `mlt convert` by caching and special-casing ([#1286](https://github.com/maplibre/maplibre-tile-spec/pull/1286))

## [0.1.10](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.9...rust-mlt-v0.1.10) - 2026-04-18

### Added

- *(rust)* mlt UI to browse .mbtiles ([#1306](https://github.com/maplibre/maplibre-tile-spec/pull/1306))

### Other

- remove ALP everywhere - it was never implemented ([#1128](https://github.com/maplibre/maplibre-tile-spec/pull/1128))
- *(rust)* rm RawStreamData and EncodedStreamData ([#1309](https://github.com/maplibre/maplibre-tile-spec/pull/1309))
- *(rust)* Coord32 cleanup, dep update ([#1304](https://github.com/maplibre/maplibre-tile-spec/pull/1304))
- Add offline docs.rs-style workspace docs check to Rust CI and fix surfaced rustdoc links ([#1295](https://github.com/maplibre/maplibre-tile-spec/pull/1295))

## [0.1.9](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.8...rust-mlt-v0.1.9) - 2026-04-13

### Added

- *(rust)* mlt convert support for full mbtiles conversion ([#1259](https://github.com/maplibre/maplibre-tile-spec/pull/1259))
- *(rust)* mlt convert, encoder ([#1240](https://github.com/maplibre/maplibre-tile-spec/pull/1240))

### Other

- *(rust)* remove a few bad performance choices (+6% encoder gain) ([#1287](https://github.com/maplibre/maplibre-tile-spec/pull/1287))
- benchmark and fix various performance issues in the encoder ([#1273](https://github.com/maplibre/maplibre-tile-spec/pull/1273))
- *(rust)* Hotpath based profiling ([#1269](https://github.com/maplibre/maplibre-tile-spec/pull/1269))
- *(rust)* massive rewrite of the encoder ([#1254](https://github.com/maplibre/maplibre-tile-spec/pull/1254))
- *(rust)* consolidate utils ([#1248](https://github.com/maplibre/maplibre-tile-spec/pull/1248))
- *(rust)* move frames/v01 -> decoder, adj use ([#1246](https://github.com/maplibre/maplibre-tile-spec/pull/1246))
- *(rust)* move more code to encoder ([#1245](https://github.com/maplibre/maplibre-tile-spec/pull/1245))
- *(rust)* mv tessellation to core ([#1220](https://github.com/maplibre/maplibre-tile-spec/pull/1220))
- *(rust)* improve synthetics, geojson ([#1210](https://github.com/maplibre/maplibre-tile-spec/pull/1210))
- add large FastPFOR synthetics ([#1205](https://github.com/maplibre/maplibre-tile-spec/pull/1205))
- *(rust)* implement feature/property iterator and more type state ([#1198](https://github.com/maplibre/maplibre-tile-spec/pull/1198))
- *(rust)* type state to represent fully-decoded layers ([#1171](https://github.com/maplibre/maplibre-tile-spec/pull/1171))

## [0.1.8](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.7...rust-mlt-v0.1.8) - 2026-03-23

### Other

- *(rust)* migrate to Rust fastpfor ([#1190](https://github.com/maplibre/maplibre-tile-spec/pull/1190))

## [0.1.7](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.6...rust-mlt-v0.1.7) - 2026-03-17

### Other

- *(rust)* memory budgeting, codecs  ([#1168](https://github.com/maplibre/maplibre-tile-spec/pull/1168))
- *(rust)* minor noop cleanup ([#1167](https://github.com/maplibre/maplibre-tile-spec/pull/1167))
- *(rust)* rename EncDec variants ([#1165](https://github.com/maplibre/maplibre-tile-spec/pull/1165))
- *(rust)* add stateful decoder ([#1163](https://github.com/maplibre/maplibre-tile-spec/pull/1163))
- *(rust)* get rid of borrowme, add EncDec enum ([#1141](https://github.com/maplibre/maplibre-tile-spec/pull/1141))

## [0.1.6](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.5...rust-mlt-v0.1.6) - 2026-03-10

### Fixed

- *(rust)* migrate part of our lengths from usize to u32 ([#1078](https://github.com/maplibre/maplibre-tile-spec/pull/1078))

### Other

- *(rust)* rework our internal data model ([#1099](https://github.com/maplibre/maplibre-tile-spec/pull/1099))

## [0.1.5](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.4...rust-mlt-v0.1.5) - 2026-03-08

### Other

- *(rust)* bump fastpfor, enable SIMD test, cleanup ([#1052](https://github.com/maplibre/maplibre-tile-spec/pull/1052))

## [0.1.4](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.3...rust-mlt-v0.1.4) - 2026-03-04

### Fixed

- *(rust)* LineString and MultiLineString geometries ([#989](https://github.com/maplibre/maplibre-tile-spec/pull/989))

### Other

- *(rust)* rename IntEncoder and add IntEncoding ([#1019](https://github.com/maplibre/maplibre-tile-spec/pull/1019))
- rename `Encoder` -> `IntegerEncoder` ([#1010](https://github.com/maplibre/maplibre-tile-spec/pull/1010))
- *(synthetic)* add Morton fixture synthetic test ([#960](https://github.com/maplibre/maplibre-tile-spec/pull/960))
- *(rust)* rm json5, inf floats to string ([#1000](https://github.com/maplibre/maplibre-tile-spec/pull/1000))

## [0.1.3](https://github.com/maplibre/maplibre-tile-spec/compare/mlt-v0.1.2...mlt-v0.1.3) - 2026-02-25

### Fixed

- *(rust)* bugs in mlt UI visualizer ([#970](https://github.com/maplibre/maplibre-tile-spec/pull/970))

## [0.1.2](https://github.com/maplibre/maplibre-tile-spec/compare/mlt-v0.1.1...mlt-v0.1.2) - 2026-02-25

### Added

- *(rust)* mlt ls wildcards, exclusions, throttle hover redraws, dependencies ([#967](https://github.com/maplibre/maplibre-tile-spec/pull/967))
- *(rust)* validate mlt-json with ls command ([#963](https://github.com/maplibre/maplibre-tile-spec/pull/963))

### Other

- *(rust)* fix another CI typo ([#959](https://github.com/maplibre/maplibre-tile-spec/pull/959))

## [0.1.1](https://github.com/maplibre/maplibre-tile-spec/compare/mlt-v0.1.0...mlt-v0.1.1) - 2026-02-25

### Added

- *(rust)* add tile preview in cli dir view ([#943](https://github.com/maplibre/maplibre-tile-spec/pull/943))

### Other

- *(rust)* more renames ([#945](https://github.com/maplibre/maplibre-tile-spec/pull/945))
- *(rust)* rename encoding-related types for clarity ([#944](https://github.com/maplibre/maplibre-tile-spec/pull/944))
- *(rust)* optimize some UI code ([#939](https://github.com/maplibre/maplibre-tile-spec/pull/939))
- *(rust)* rename LogicalDecoder and PhysicalDecoder to codecs ([#923](https://github.com/maplibre/maplibre-tile-spec/pull/923))
- *(rust)* add basic benchmark infrastructure ([#912](https://github.com/maplibre/maplibre-tile-spec/pull/912))
- *(rust)* more cleanups ([#914](https://github.com/maplibre/maplibre-tile-spec/pull/914))
- *(ci)* cleanup ci workflows ([#908](https://github.com/maplibre/maplibre-tile-spec/pull/908))
