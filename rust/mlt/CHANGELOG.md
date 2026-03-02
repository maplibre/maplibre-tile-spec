# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.4](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-v0.1.3...rust-mlt-v0.1.4) - 2026-03-02

### Fixed

- *(rust)* LineString and MultiLineString geometries ([#989](https://github.com/maplibre/maplibre-tile-spec/pull/989))

### Other

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
