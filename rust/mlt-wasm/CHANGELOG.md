# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.31](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.30...rust-mlt-wasm-v0.1.31) - 2026-09-25

### Added

- *(docs)* add layer name to the inspector ([#1767](https://github.com/maplibre/maplibre-tile-spec/pull/1767))
- *(docs)* show geometry visualization in inspector ([#1763](https://github.com/maplibre/maplibre-tile-spec/pull/1763))

### Other

- *(rust)* re-add ByteRLE for presence variants ([#1768](https://github.com/maplibre/maplibre-tile-spec/pull/1768))

## [0.1.30](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.29...rust-mlt-wasm-v0.1.30) - 2026-09-23

### Added

- *(rust)* fill the v2 layer header byte's reserved nibble ([#1753](https://github.com/maplibre/maplibre-tile-spec/pull/1753))

### Fixed

- minor issues in the tile inspector ([#1755](https://github.com/maplibre/maplibre-tile-spec/pull/1755))

### Other

- *(rust)* cleanup cmd, rm aws-sdk dep from mlt ([#1757](https://github.com/maplibre/maplibre-tile-spec/pull/1757))

## [0.1.29](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.28...rust-mlt-wasm-v0.1.29) - 2026-09-22

### Added

- *(rust)* code the v2 extent as one nibble ([#1749](https://github.com/maplibre/maplibre-tile-spec/pull/1749))

### Other

- move the default extent in the synthetics to 64 instead of 80 ([#1751](https://github.com/maplibre/maplibre-tile-spec/pull/1751))
- *(java)* drop the 0x02-java synthetic fixtures ([#1744](https://github.com/maplibre/maplibre-tile-spec/pull/1744))
- *(rust)* maybe increase coverage in CI by being more accurate ([#1743](https://github.com/maplibre/maplibre-tile-spec/pull/1743))

## [0.1.28](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.27...rust-mlt-wasm-v0.1.28) - 2026-09-21

### Added

- *(rust)* implement an inspector ([#1735](https://github.com/maplibre/maplibre-tile-spec/pull/1735))
- *(rust)* wasm annotate binding ([#1725](https://github.com/maplibre/maplibre-tile-spec/pull/1725))

## [0.1.27](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.26...rust-mlt-wasm-v0.1.27) - 2026-09-20

### Other

- make autofix not self-cancel on release labels ([#1729](https://github.com/maplibre/maplibre-tile-spec/pull/1729))

## [0.1.26](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.25...rust-mlt-wasm-v0.1.26) - 2026-09-18

### Other

- updated the following local packages: mlt-core

## [0.1.25](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.24...rust-mlt-wasm-v0.1.25) - 2026-09-17

### Other

- updated the following local packages: mlt-core

## [0.1.24](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.23...rust-mlt-wasm-v0.1.24) - 2026-09-14

### Other

- updated the following local packages: mlt-core

## [0.1.23](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.22...rust-mlt-wasm-v0.1.23) - 2026-09-07

### Other

- updated the following local packages: mlt-core

## [0.1.22](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.21...rust-mlt-wasm-v0.1.22) - 2026-09-04

### Other

- updated the following local packages: mlt-core

## [0.1.21](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.20...rust-mlt-wasm-v0.1.21) - 2026-08-31

### Added

- *(rust)* implement basic header and metadata format ([#1571](https://github.com/maplibre/maplibre-tile-spec/pull/1571))

### Fixed

- *(java)* move the Java nested-property fixtures to 0x02-java ([#1582](https://github.com/maplibre/maplibre-tile-spec/pull/1582))

## [0.1.20](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.19...rust-mlt-wasm-v0.1.20) - 2026-08-21

### Other

- add an anti-vibing pre-commit to clean up weird unicode and tabs ([#1566](https://github.com/maplibre/maplibre-tile-spec/pull/1566))

## [0.1.19](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.18...rust-mlt-wasm-v0.1.19) - 2026-08-19

### Other

- updated the following local packages: mlt-core

## [0.1.18](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.17...rust-mlt-wasm-v0.1.18) - 2026-07-18

### Other

- updated the following local packages: mlt-core

## [0.1.17](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.16...rust-mlt-wasm-v0.1.17) - 2026-07-01

### Other

- updated the following local packages: mlt-core

## [0.1.16](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.15...rust-mlt-wasm-v0.1.16) - 2026-06-29

### Added

- *(java)* Nested property values ([#1379](https://github.com/maplibre/maplibre-tile-spec/pull/1379))

### Other

- *(deps-dev)* bump protocol-buffers-schema from 3.6.0 to 3.6.1 in /rust/mlt-wasm ([#1478](https://github.com/maplibre/maplibre-tile-spec/pull/1478))
- *(deps-dev)* bump postcss from 8.5.8 to 8.5.16 in /rust/mlt-wasm ([#1477](https://github.com/maplibre/maplibre-tile-spec/pull/1477))

## [0.1.15](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.14...rust-mlt-wasm-v0.1.15) - 2026-06-20

### Other

- make sure that marked-failing synthetics actually fail ([#1451](https://github.com/maplibre/maplibre-tile-spec/pull/1451))
- *(deps)* bump fast-mvt from 0.3.2 to 0.4.0 in /rust in the all-cargo-version-updates group ([#1452](https://github.com/maplibre/maplibre-tile-spec/pull/1452))

## [0.1.14](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.13...rust-mlt-wasm-v0.1.14) - 2026-06-19

### Other

- *(rust)* rm pub fields, builder pattern ([#1428](https://github.com/maplibre/maplibre-tile-spec/pull/1428))

## [0.1.13](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.12...rust-mlt-wasm-v0.1.13) - 2026-06-18

### Other

- *(deps-dev)* bump flatted from 3.3.3 to 3.4.2 in /ts in the all-npm-security-updates group across 1 directory ([#1184](https://github.com/maplibre/maplibre-tile-spec/pull/1184))
- *(deps-dev)* bump vite from 7.3.1 to 7.3.5 in /rust/mlt-wasm ([#1441](https://github.com/maplibre/maplibre-tile-spec/pull/1441))

## [0.1.12](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.11...rust-mlt-wasm-v0.1.12) - 2026-06-15

### Other

- updated the following local packages: mlt-core

## [0.1.11](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.10...rust-mlt-wasm-v0.1.11) - 2026-06-13

### Other

- updated the following local packages: mlt-core

## [0.1.10](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.9...rust-mlt-wasm-v0.1.10) - 2026-06-08

### Other

- *(deps-dev)* bump vitest, @vitest/coverage-v8 and @vitest/ui in /rust/mlt-wasm ([#1426](https://github.com/maplibre/maplibre-tile-spec/pull/1426))

## [0.1.9](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.8...rust-mlt-wasm-v0.1.9) - 2026-05-15

### Other

- updated the following local packages: mlt-core

## [0.1.8](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.7...rust-mlt-wasm-v0.1.8) - 2026-05-14

### Other

- updated the following local packages: mlt-core

## [0.1.7](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.6...rust-mlt-wasm-v0.1.7) - 2026-05-05

### Other

- updated the following local packages: mlt-core

## [0.1.6](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.5...rust-mlt-wasm-v0.1.6) - 2026-04-29

### Other

- *(rust)* rm "01" from TileLayer01, StagedLayer01 ([#1345](https://github.com/maplibre/maplibre-tile-spec/pull/1345))

## [0.1.5](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.4...rust-mlt-wasm-v0.1.5) - 2026-04-18

### Fixed

- *(rust)* geo builds on wasm, remove unnecessary feature gates ([#1297](https://github.com/maplibre/maplibre-tile-spec/pull/1297))

### Other

- *(rust)* rm RawStreamData and EncodedStreamData ([#1309](https://github.com/maplibre/maplibre-tile-spec/pull/1309))
- *(rust)* Coord32 cleanup, dep update ([#1304](https://github.com/maplibre/maplibre-tile-spec/pull/1304))
- *(rust)* update geo, simplify tessellation ([#1305](https://github.com/maplibre/maplibre-tile-spec/pull/1305))
- *(java)* Extend synthetic tests to include rings ([#1292](https://github.com/maplibre/maplibre-tile-spec/pull/1292))
- Add offline docs.rs-style workspace docs check to Rust CI and fix surfaced rustdoc links ([#1295](https://github.com/maplibre/maplibre-tile-spec/pull/1295))

## [0.1.4](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.3...rust-mlt-wasm-v0.1.4) - 2026-04-13

### Other

- *(rust)* move frames/v01 -> decoder, adj use ([#1246](https://github.com/maplibre/maplibre-tile-spec/pull/1246))
- *(rust)* mv tessellation to core ([#1220](https://github.com/maplibre/maplibre-tile-spec/pull/1220))
- more tessellated synthetics ([#1218](https://github.com/maplibre/maplibre-tile-spec/pull/1218))
- *(rust)* implement feature/property iterator and more type state ([#1198](https://github.com/maplibre/maplibre-tile-spec/pull/1198))

## [0.1.3](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.2...rust-mlt-wasm-v0.1.3) - 2026-03-23

### Other

- *(rust)* migrate to Rust fastpfor ([#1190](https://github.com/maplibre/maplibre-tile-spec/pull/1190))

## [0.1.2](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.1...rust-mlt-wasm-v0.1.2) - 2026-03-17

### Other

- *(rust)* memory budgeting, codecs  ([#1168](https://github.com/maplibre/maplibre-tile-spec/pull/1168))
- *(rust)* introduce EncDec decode states ([#1166](https://github.com/maplibre/maplibre-tile-spec/pull/1166))
- *(rust)* add stateful decoder ([#1163](https://github.com/maplibre/maplibre-tile-spec/pull/1163))
- *(rust)* rename to IdValues and GeometryValues ([#1159](https://github.com/maplibre/maplibre-tile-spec/pull/1159))
- *(rust)* mv impls out of models, use full wire round-trips ([#1158](https://github.com/maplibre/maplibre-tile-spec/pull/1158))
- *(rust)* rework WASM code to use TileLayer ([#1153](https://github.com/maplibre/maplibre-tile-spec/pull/1153))
- *(rust)* remove unnecessary to_owned calls ([#1151](https://github.com/maplibre/maplibre-tile-spec/pull/1151))
- *(rust)* introduce staging types in Rust layer implementation ([#1149](https://github.com/maplibre/maplibre-tile-spec/pull/1149))
- *(rust)* introduce staging types ([#1148](https://github.com/maplibre/maplibre-tile-spec/pull/1148))
- *(rust)* refactor parsing and encoding code ([#1144](https://github.com/maplibre/maplibre-tile-spec/pull/1144))
- *(rust)* get rid of borrowme, add EncDec enum ([#1141](https://github.com/maplibre/maplibre-tile-spec/pull/1141))
- *(rust)* simplify ID model ([#1139](https://github.com/maplibre/maplibre-tile-spec/pull/1139))
- *(rust)* make wasm bench more stable ([#1120](https://github.com/maplibre/maplibre-tile-spec/pull/1120))
- *(rust)* move name into DecodedStrings ([#1108](https://github.com/maplibre/maplibre-tile-spec/pull/1108))

## [0.1.1](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-wasm-v0.1.0...rust-mlt-wasm-v0.1.1) - 2026-03-10

### Other

- updated the following local packages: mlt-core
