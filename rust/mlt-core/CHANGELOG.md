# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/maplibre/maplibre-tile-spec/compare/mlt-core-v0.1.0...mlt-core-v0.1.1) - 2026-02-25

### Added

- *(rust)* Stream encoding optimiser ([#950](https://github.com/maplibre/maplibre-tile-spec/pull/950))
- *(rust)* Synthetic generation ([#947](https://github.com/maplibre/maplibre-tile-spec/pull/947))
- *(rust)* FastPFOR encoding support ([#936](https://github.com/maplibre/maplibre-tile-spec/pull/936))
- *(rust)* implement geometry encoding functionality ([#933](https://github.com/maplibre/maplibre-tile-spec/pull/933))
- *(rust)* string encoding support ([#937](https://github.com/maplibre/maplibre-tile-spec/pull/937))
- *(rust)* Point Geometry encoding ([#932](https://github.com/maplibre/maplibre-tile-spec/pull/932))
- *(rust)* Property encoding ([#929](https://github.com/maplibre/maplibre-tile-spec/pull/929))
- *(rust)* impl stream encoding ([#924](https://github.com/maplibre/maplibre-tile-spec/pull/924))
- *(rust)* impl `encode_componentwise_delta_vec2s` ([#930](https://github.com/maplibre/maplibre-tile-spec/pull/930))
- *(rust)* Id encoder implementation ([#906](https://github.com/maplibre/maplibre-tile-spec/pull/906))

### Fixed

- *(rust)* simplify int parsing errors ([#921](https://github.com/maplibre/maplibre-tile-spec/pull/921))

### Other

- *(rust)* improve rust synthetics generation ([#951](https://github.com/maplibre/maplibre-tile-spec/pull/951))
- *(rust)* Reword the docs ([#946](https://github.com/maplibre/maplibre-tile-spec/pull/946))
- *(rust)* more renames ([#945](https://github.com/maplibre/maplibre-tile-spec/pull/945))
- *(rust)* rename encoding->encoder
- *(rust)* rename encoding-related types for clarity ([#944](https://github.com/maplibre/maplibre-tile-spec/pull/944))
- *(rust)* optimize some UI code ([#939](https://github.com/maplibre/maplibre-tile-spec/pull/939))
- *(rust)* simplify the proptest strategy ([#935](https://github.com/maplibre/maplibre-tile-spec/pull/935))
- *(rust)* remove unused variables and simplify buffer handling ([#925](https://github.com/maplibre/maplibre-tile-spec/pull/925))
- *(rust)* rename LogicalDecoder and PhysicalDecoder to codecs ([#923](https://github.com/maplibre/maplibre-tile-spec/pull/923))
- *(rust)* improve error handling and messaging in error types ([#920](https://github.com/maplibre/maplibre-tile-spec/pull/920))
- *(rust)* rename `UnsupportedLogicalTechniqueCombination` ([#919](https://github.com/maplibre/maplibre-tile-spec/pull/919))
- *(rust)* Shift id encoding to the stream ([#918](https://github.com/maplibre/maplibre-tile-spec/pull/918))
- *(rust)* add basic benchmark infrastructure ([#912](https://github.com/maplibre/maplibre-tile-spec/pull/912))
- *(rust)* cleanup errors and some namespaces ([#917](https://github.com/maplibre/maplibre-tile-spec/pull/917))
- *(rust)* cleanup around the property mod ([#915](https://github.com/maplibre/maplibre-tile-spec/pull/915))
- *(rust)* more cleanups ([#914](https://github.com/maplibre/maplibre-tile-spec/pull/914))
- *(rust)* rename raw -> encoded ([#910](https://github.com/maplibre/maplibre-tile-spec/pull/910))
- *(rust)* refactor and add plumbing for encoding ([#909](https://github.com/maplibre/maplibre-tile-spec/pull/909))
