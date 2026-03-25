# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-core-v0.5.0...rust-mlt-core-v0.6.0) - 2026-03-23

### Other

- *(rust)* migrate to Rust fastpfor ([#1190](https://github.com/maplibre/maplibre-tile-spec/pull/1190))

## [0.5.0](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-core-v0.4.0...rust-mlt-core-v0.5.0) - 2026-03-17

### Added

- *(rust)* allow resorting in the optimiser ([#1133](https://github.com/maplibre/maplibre-tile-spec/pull/1133))
- *(rust)* migrate the property optimiser to the PGO architecture ([#1118](https://github.com/maplibre/maplibre-tile-spec/pull/1118))
- *(rust)* migrate geometrys to allow for PGO ([#1112](https://github.com/maplibre/maplibre-tile-spec/pull/1112))
- *(rust)* PGO based Id optimiser ([#1105](https://github.com/maplibre/maplibre-tile-spec/pull/1105))

### Other

- *(rust)* memory budgeting, codecs  ([#1168](https://github.com/maplibre/maplibre-tile-spec/pull/1168))
- *(rust)* minor noop cleanup ([#1167](https://github.com/maplibre/maplibre-tile-spec/pull/1167))
- *(rust)* introduce EncDec decode states ([#1166](https://github.com/maplibre/maplibre-tile-spec/pull/1166))
- *(rust)* rename EncDec variants ([#1165](https://github.com/maplibre/maplibre-tile-spec/pull/1165))
- *(rust)* add stateful decoder ([#1163](https://github.com/maplibre/maplibre-tile-spec/pull/1163))
- *(rust)* rm TryFrom EncodedGeometry,EncodedId ([#1162](https://github.com/maplibre/maplibre-tile-spec/pull/1162))
- *(rust)* rm FromDecoded, use encode(...) ([#1161](https://github.com/maplibre/maplibre-tile-spec/pull/1161))
- *(rust)* finish big refactoring ([#1160](https://github.com/maplibre/maplibre-tile-spec/pull/1160))
- *(rust)* rename to IdValues and GeometryValues ([#1159](https://github.com/maplibre/maplibre-tile-spec/pull/1159))
- *(rust)* mv impls out of models, use full wire round-trips ([#1158](https://github.com/maplibre/maplibre-tile-spec/pull/1158))
- *(chore)* remove into_static, test fixes ([#1154](https://github.com/maplibre/maplibre-tile-spec/pull/1154))
- *(rust)* rework WASM code to use TileLayer01 ([#1153](https://github.com/maplibre/maplibre-tile-spec/pull/1153))
- *(rust)* remove unnecessary to_owned calls ([#1151](https://github.com/maplibre/maplibre-tile-spec/pull/1151))
- *(rust)* introduce staging types in Rust layer implementation ([#1149](https://github.com/maplibre/maplibre-tile-spec/pull/1149))
- *(rust)* introduce staging types ([#1148](https://github.com/maplibre/maplibre-tile-spec/pull/1148))
- *(rust)* code structure and remove NameRef ([#1147](https://github.com/maplibre/maplibre-tile-spec/pull/1147))
- *(rust)* refactor parsing and encoding code ([#1144](https://github.com/maplibre/maplibre-tile-spec/pull/1144))
- *(rust)* get rid of borrowme, add EncDec enum ([#1141](https://github.com/maplibre/maplibre-tile-spec/pull/1141))
- *(rust)* simplify ID model ([#1139](https://github.com/maplibre/maplibre-tile-spec/pull/1139))
- *(rust)* replace FromEncoded with Decode and decode_into ([#1134](https://github.com/maplibre/maplibre-tile-spec/pull/1134))
- *(rust)* move impls out of model, scalars ([#1132](https://github.com/maplibre/maplibre-tile-spec/pull/1132))
- *(rust)* rm borrowme from Layer01 ([#1129](https://github.com/maplibre/maplibre-tile-spec/pull/1129))
- *(rust)* remove unused defaults ([#1127](https://github.com/maplibre/maplibre-tile-spec/pull/1127))
- *(rust)* use enum_dispatch where appropriate, Cow for name ([#1125](https://github.com/maplibre/maplibre-tile-spec/pull/1125))
- *(rust)* streamline str decoding ([#1121](https://github.com/maplibre/maplibre-tile-spec/pull/1121))
- *(rust)* string performance benchmark ([#1123](https://github.com/maplibre/maplibre-tile-spec/pull/1123))
- refactor!(rust): make `FromEncoded` and `FromDecoded` `pub(crate)` ([#1119](https://github.com/maplibre/maplibre-tile-spec/pull/1119))
- *(rust)* use strum::IntoStaticStr ([#1122](https://github.com/maplibre/maplibre-tile-spec/pull/1122))
- *(rust)* simplify Morton code decoding with SIMD ([#1114](https://github.com/maplibre/maplibre-tile-spec/pull/1114))
- *(rust)* cleanup after big move ([#1113](https://github.com/maplibre/maplibre-tile-spec/pull/1113))
- *(rust)* clean up all module and mod files ([#1111](https://github.com/maplibre/maplibre-tile-spec/pull/1111))
- *(rust)* rename mod layer to frames ([#1110](https://github.com/maplibre/maplibre-tile-spec/pull/1110))
- *(rust)* rm PropValue, consistent Cows ([#1109](https://github.com/maplibre/maplibre-tile-spec/pull/1109))
- *(rust)* move name into DecodedStrings ([#1108](https://github.com/maplibre/maplibre-tile-spec/pull/1108))
- *(rust)* change tests to be based on the new trait based optimization api ([#1104](https://github.com/maplibre/maplibre-tile-spec/pull/1104))

## [0.4.0](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-core-v0.3.0...rust-mlt-core-v0.4.0) - 2026-03-10

### Added

- plumbing for profile optimization ([#1101](https://github.com/maplibre/maplibre-tile-spec/pull/1101))
- *(rust)* trait based automatic optimiser ([#1093](https://github.com/maplibre/maplibre-tile-spec/pull/1093))

### Fixed

- *(rust)* migrate part of our lengths from usize to u32 ([#1078](https://github.com/maplibre/maplibre-tile-spec/pull/1078))

### Other

- *(rust)* rework our internal data model ([#1099](https://github.com/maplibre/maplibre-tile-spec/pull/1099))
- *(rust)* rename structs to model ([#1100](https://github.com/maplibre/maplibre-tile-spec/pull/1100))
- *(rust)* handle u32->usize, int overflows ([#1097](https://github.com/maplibre/maplibre-tile-spec/pull/1097))
- *(rust)* move all structs to structs.rs ([#1094](https://github.com/maplibre/maplibre-tile-spec/pull/1094))
- renamings in the encoder ([#1090](https://github.com/maplibre/maplibre-tile-spec/pull/1090))
- *(rust)* refactoring step 3 ([#1088](https://github.com/maplibre/maplibre-tile-spec/pull/1088))
- *(rust)* refactor name storage ([#1087](https://github.com/maplibre/maplibre-tile-spec/pull/1087))
- *(rust)* more core renames, rm EncodedValues::typ ([#1084](https://github.com/maplibre/maplibre-tile-spec/pull/1084))
- *(rust)* some more renames of internals ([#1082](https://github.com/maplibre/maplibre-tile-spec/pull/1082))
- *(rust)* noop, only renames for future refactoring ([#1081](https://github.com/maplibre/maplibre-tile-spec/pull/1081))
- make sure we write u32 varints instead of u64 varints ([#1080](https://github.com/maplibre/maplibre-tile-spec/pull/1080))
- use simd for morton decoding ([#1069](https://github.com/maplibre/maplibre-tile-spec/pull/1069))

## [0.3.0](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-core-v0.2.0...rust-mlt-core-v0.3.0) - 2026-03-08

### Added

- *(rust)* property encoder optimizer ([#1042](https://github.com/maplibre/maplibre-tile-spec/pull/1042))
- *(rust)* Id optimizer impl ([#1043](https://github.com/maplibre/maplibre-tile-spec/pull/1043))
- *(rust)* Geometry optimizer ([#1045](https://github.com/maplibre/maplibre-tile-spec/pull/1045))

### Other

- *(rust)* rm Dict and FsstDict from SharedDict encoding ([#1068](https://github.com/maplibre/maplibre-tile-spec/pull/1068))
- *(rust)* major rework of the shared dict ([#1066](https://github.com/maplibre/maplibre-tile-spec/pull/1066))
- *(rust)* a bit more geotype cleanup ([#1059](https://github.com/maplibre/maplibre-tile-spec/pull/1059))
- *(rust)* cleanup geotype code ([#1058](https://github.com/maplibre/maplibre-tile-spec/pull/1058))
- *(rust)* move geotype code to its own mod ([#1053](https://github.com/maplibre/maplibre-tile-spec/pull/1053))
- *(rust)* Fuzz the return path for roundtrip-ability ([#1044](https://github.com/maplibre/maplibre-tile-spec/pull/1044))
- *(rust)* bump fastpfor, enable SIMD test, cleanup ([#1052](https://github.com/maplibre/maplibre-tile-spec/pull/1052))
- move property tests to be based on the public API ([#1038](https://github.com/maplibre/maplibre-tile-spec/pull/1038))

## [0.2.0](https://github.com/maplibre/maplibre-tile-spec/compare/mlt-core-v0.1.2...rust-mlt-core-v0.2.0) - 2026-03-04

### Added

- *(rust)* enable configuring the shared dictionarys fully ([#1015](https://github.com/maplibre/maplibre-tile-spec/pull/1015))
- *(rust)* shared dictionary encoding support ([#1006](https://github.com/maplibre/maplibre-tile-spec/pull/1006))
- *(rust)* add rust morton encoding ([#1005](https://github.com/maplibre/maplibre-tile-spec/pull/1005))
- *(rust)* implement `arbitrary::Arbitrary` for simpler fuzzing setups ([#991](https://github.com/maplibre/maplibre-tile-spec/pull/991))

### Fixed

- *(encoders)* F64 encoded as f64 ([#1009](https://github.com/maplibre/maplibre-tile-spec/pull/1009))
- *(rust)* LineString and MultiLineString geometries ([#989](https://github.com/maplibre/maplibre-tile-spec/pull/989))
- *(rust)* encoding of normalized mixed multi + regular geometrys panics ([#981](https://github.com/maplibre/maplibre-tile-spec/pull/981))

### Other

- *(rust)* rework string parsing ([#1026](https://github.com/maplibre/maplibre-tile-spec/pull/1026))
- *(rust)* move optional into EncodedPropValue ([#1024](https://github.com/maplibre/maplibre-tile-spec/pull/1024))
- *(rust)* prop strings mod ([#1023](https://github.com/maplibre/maplibre-tile-spec/pull/1023))
- *(rust)* rename IntEncoder and add IntEncoding ([#1019](https://github.com/maplibre/maplibre-tile-spec/pull/1019))
- *(rust)* move prop internals to "big enum" arch ([#1014](https://github.com/maplibre/maplibre-tile-spec/pull/1014))
- clean up how FromDecoded interacts with strings ([#1011](https://github.com/maplibre/maplibre-tile-spec/pull/1011))
- rename `Encoder` -> `IntegerEncoder` ([#1010](https://github.com/maplibre/maplibre-tile-spec/pull/1010))
- *(rust)* start fixing rust synthetics ([#1002](https://github.com/maplibre/maplibre-tile-spec/pull/1002))
- *(synthetic)* add Morton fixture synthetic test ([#960](https://github.com/maplibre/maplibre-tile-spec/pull/960))
- More testing around geometry combinations ([#982](https://github.com/maplibre/maplibre-tile-spec/pull/982))
- *(rust)* remove AVX failing test ([#999](https://github.com/maplibre/maplibre-tile-spec/pull/999))
- *(rust)* rm json5, inf floats to string ([#1000](https://github.com/maplibre/maplibre-tile-spec/pull/1000))
- *(rust)* Add an encoding benchmark ([#997](https://github.com/maplibre/maplibre-tile-spec/pull/997))

## [0.1.2](https://github.com/maplibre/maplibre-tile-spec/compare/mlt-core-v0.1.1...mlt-core-v0.1.2) - 2026-02-25

### Added

- Release done to test if some of the release automation is working. Should fix `cargo-binstall mlt`

## [0.1.1](https://github.com/maplibre/maplibre-tile-spec/compare/mlt-core-v0.1.0...mlt-core-v0.1.1) - 2026-02-25

### Added

- *(rust)* Stream encoding optimizer ([#950](https://github.com/maplibre/maplibre-tile-spec/pull/950))
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
