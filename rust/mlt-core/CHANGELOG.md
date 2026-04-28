# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-core-v0.8.0...rust-mlt-core-v0.9.0) - 2026-04-28

### Fixed

- *(rust)* terrible fsst compression ratio due to reusing compressor results for unrelated shared dicts ([#1333](https://github.com/maplibre/maplibre-tile-spec/pull/1333))

### Other

- *(rust)* consolidate encoders with common buffers ([#1352](https://github.com/maplibre/maplibre-tile-spec/pull/1352))
- *(rust)* compute more stats before sort to enable deduplicating presence in v2 ([#1348](https://github.com/maplibre/maplibre-tile-spec/pull/1348))
- *(rust)* simplify StagingId ([#1347](https://github.com/maplibre/maplibre-tile-spec/pull/1347))
- *(rust)* new encoding methods ([#1346](https://github.com/maplibre/maplibre-tile-spec/pull/1346))
- *(rust)* rm "01" from TileLayer01, StagedLayer01 ([#1345](https://github.com/maplibre/maplibre-tile-spec/pull/1345))
- *(rust)* remove unused staging code and eq impl ([#1344](https://github.com/maplibre/maplibre-tile-spec/pull/1344))
- *(rust)* follow up to Presence cleanup ([#1337](https://github.com/maplibre/maplibre-tile-spec/pull/1337))
- *(rust)* simplify debug formatting ([#1336](https://github.com/maplibre/maplibre-tile-spec/pull/1336))
- *(rust)* IdValues→ParsedId+StagedId, simplify presence ([#1334](https://github.com/maplibre/maplibre-tile-spec/pull/1334))
- *(rust)* push fsst training up one level to not gain false statistics ([#1335](https://github.com/maplibre/maplibre-tile-spec/pull/1335))
- *(rust)* rework presence to use bitvec ([#1329](https://github.com/maplibre/maplibre-tile-spec/pull/1329))

## [0.8.0](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-core-v0.7.0...rust-mlt-core-v0.8.0) - 2026-04-18

### Fixed

- fix(rust) from_tile tessellates when appropriate ([#1314](https://github.com/maplibre/maplibre-tile-spec/pull/1314))
- *(rust)* geo builds on wasm, remove unnecessary feature gates ([#1297](https://github.com/maplibre/maplibre-tile-spec/pull/1297))

### Other

- *(rust)* cleanup Morton structs ([#1310](https://github.com/maplibre/maplibre-tile-spec/pull/1310))
- *(rust)* cleanup dict ranges ([#1313](https://github.com/maplibre/maplibre-tile-spec/pull/1313))
- remove ALP everywhere - it was never implemented ([#1128](https://github.com/maplibre/maplibre-tile-spec/pull/1128))
- *(rust)* use Coord32 instead of tuple ([#1312](https://github.com/maplibre/maplibre-tile-spec/pull/1312))
- *(rust)* rm RawStreamData and EncodedStreamData ([#1309](https://github.com/maplibre/maplibre-tile-spec/pull/1309))
- *(rust)* Coord32 cleanup, dep update ([#1304](https://github.com/maplibre/maplibre-tile-spec/pull/1304))
- *(rust)* update geo, simplify tessellation ([#1305](https://github.com/maplibre/maplibre-tile-spec/pull/1305))
- Add offline docs.rs-style workspace docs check to Rust CI and fix surfaced rustdoc links ([#1295](https://github.com/maplibre/maplibre-tile-spec/pull/1295))
- docs(rust) Fix broken doc links ([#1293](https://github.com/maplibre/maplibre-tile-spec/pull/1293))

## [0.7.0](https://github.com/maplibre/maplibre-tile-spec/compare/rust-mlt-core-v0.6.0...rust-mlt-core-v0.7.0) - 2026-04-13

### Added

- *(rust)* trigram based shared dictionary selection ([#1283](https://github.com/maplibre/maplibre-tile-spec/pull/1283))
- *(rust)* allow delta-rle picking in stream selection ([#1258](https://github.com/maplibre/maplibre-tile-spec/pull/1258))
- *(rust)* impl memory budget reset ([#1257](https://github.com/maplibre/maplibre-tile-spec/pull/1257))
- *(rust)* enable FPF for id encoding ([#1243](https://github.com/maplibre/maplibre-tile-spec/pull/1243))
- *(rust)* mlt convert, encoder ([#1240](https://github.com/maplibre/maplibre-tile-spec/pull/1240))
- *(rust)* enhance property iteration ([#1225](https://github.com/maplibre/maplibre-tile-spec/pull/1225))
- *(rust)* SharedDict/Plain string enc support ([#1213](https://github.com/maplibre/maplibre-tile-spec/pull/1213))

### Fixed

- don't unwrap in `adjust_alloc` ([#1275](https://github.com/maplibre/maplibre-tile-spec/pull/1275))
- *(rust)* overflow-panic found by the fuzzer ([#1268](https://github.com/maplibre/maplibre-tile-spec/pull/1268))
- *(rust)* fix geometry decoding fuzz bug ([#1264](https://github.com/maplibre/maplibre-tile-spec/pull/1264))
- *(rust)* fix decoding panic found by fuzz ([#1261](https://github.com/maplibre/maplibre-tile-spec/pull/1261))
- *(rust)* speed up encoding by 30% due to dup sort bug ([#1260](https://github.com/maplibre/maplibre-tile-spec/pull/1260))
- *(rust)* disallow mem leaks in rust, fix test ([#1256](https://github.com/maplibre/maplibre-tile-spec/pull/1256))
- *(rust)* test helper accidentally exposed ([#1250](https://github.com/maplibre/maplibre-tile-spec/pull/1250))

### Other

- Use `impl Iterator` in more places ([#1288](https://github.com/maplibre/maplibre-tile-spec/pull/1288))
- *(rust)* remove a few bad performance choices (+6% encoder gain) ([#1287](https://github.com/maplibre/maplibre-tile-spec/pull/1287))
- *(rust)* resolve most of the encoding quality + performance benefits of java ([#1281](https://github.com/maplibre/maplibre-tile-spec/pull/1281))
- *(rust)* Don't allocate for calculating the hilbert params and resulting parameters ([#1285](https://github.com/maplibre/maplibre-tile-spec/pull/1285))
- *(rust)* improve fuzzer ([#1278](https://github.com/maplibre/maplibre-tile-spec/pull/1278))
- *(rust)* make sure we benchmark ourselves against the compressed variants ([#1194](https://github.com/maplibre/maplibre-tile-spec/pull/1194))
- *(rust)* make staging differ optional from non-opt columns ([#1276](https://github.com/maplibre/maplibre-tile-spec/pull/1276))
- *(rust)* cache morton computation ([#1277](https://github.com/maplibre/maplibre-tile-spec/pull/1277))
- benchmark and fix various performance issues in the encoder ([#1273](https://github.com/maplibre/maplibre-tile-spec/pull/1273))
- fix weird unessary allocation ([#1272](https://github.com/maplibre/maplibre-tile-spec/pull/1272))
- *(rust)* reuse buffers, casting ([#1267](https://github.com/maplibre/maplibre-tile-spec/pull/1267))
- *(rust)* Hotpath based profiling ([#1269](https://github.com/maplibre/maplibre-tile-spec/pull/1269))
- *(rust)* add StreamCtx to simplify callbacks ([#1263](https://github.com/maplibre/maplibre-tile-spec/pull/1263))
- *(rust)* remove unused auto_u32/u64 ([#1265](https://github.com/maplibre/maplibre-tile-spec/pull/1265))
- *(rust)* rm obsolete encode_* funcs ([#1262](https://github.com/maplibre/maplibre-tile-spec/pull/1262))
- *(rust)* massive rewrite of the encoder ([#1254](https://github.com/maplibre/maplibre-tile-spec/pull/1254))
- *(rust)* re-enable the fuzzer in CI ([#1244](https://github.com/maplibre/maplibre-tile-spec/pull/1244))
- *(rust)* improve decoding benchmarks ([#1247](https://github.com/maplibre/maplibre-tile-spec/pull/1247))
- *(rust)* clean up utils ([#1251](https://github.com/maplibre/maplibre-tile-spec/pull/1251))
- *(rust)* consolidate utils ([#1248](https://github.com/maplibre/maplibre-tile-spec/pull/1248))
- *(rust)* move frames/v01 -> decoder, adj use ([#1246](https://github.com/maplibre/maplibre-tile-spec/pull/1246))
- *(rust)* move more code to encoder ([#1245](https://github.com/maplibre/maplibre-tile-spec/pull/1245))
- *(rust)* move all encoding code to separate dir ([#1241](https://github.com/maplibre/maplibre-tile-spec/pull/1241))
- *(rust)* streamline StagedStrings and StagedSharedDict ([#1236](https://github.com/maplibre/maplibre-tile-spec/pull/1236))
- *(rust)* add tests with no presence streams ([#1233](https://github.com/maplibre/maplibre-tile-spec/pull/1233))
- *(rust)* encoder rework, rm profiles ([#1232](https://github.com/maplibre/maplibre-tile-spec/pull/1232))
- *(rust)* mv tessellation to core ([#1220](https://github.com/maplibre/maplibre-tile-spec/pull/1220))
- *(rust)* improve synthetics, geojson ([#1210](https://github.com/maplibre/maplibre-tile-spec/pull/1210))
- add large FastPFOR synthetics ([#1205](https://github.com/maplibre/maplibre-tile-spec/pull/1205))
- *(rust)* use -ize spelling ([#1200](https://github.com/maplibre/maplibre-tile-spec/pull/1200))
- *(rust)* implement feature/property iterator and more type state ([#1198](https://github.com/maplibre/maplibre-tile-spec/pull/1198))
- *(rust)* type state to represent fully-decoded layers ([#1171](https://github.com/maplibre/maplibre-tile-spec/pull/1171))
- *(rust)* cleanup Results, minor styling ([#1192](https://github.com/maplibre/maplibre-tile-spec/pull/1192))

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
- *(rust)* rework WASM code to use TileLayer ([#1153](https://github.com/maplibre/maplibre-tile-spec/pull/1153))
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
- rename `Encoder` -> `PhysicalCodecs` ([#1010](https://github.com/maplibre/maplibre-tile-spec/pull/1010))
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
