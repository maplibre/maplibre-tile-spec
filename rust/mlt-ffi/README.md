# mlt-ffi

[Diplomat](https://github.com/rust-diplomat/diplomat)-based FFI bindings for`mlt-core`, providing MLT <-> MVT conversion and per-feature layer encoding from C, C++ and Kotlin.

## API

| Type                | Purpose                                                                                           |
|---------------------|---------------------------------------------------------------------------------------------------|
| `MltConverter`      | `mlt_to_mvt(bytes)`, `mlt_to_mvt_with_limit(bytes, max_bytes)` and `mvt_to_mlt(bytes, options)`   |
| `MltLayerBuilder`   | Writes a layer one feature at a time and encodes it into an `MltBuffer` without going through MVT |
| `MltEncoderOptions` | Builder wrapping `EncoderConfig` - construct with `new()`, toggle flags with setters              |
| `MltBuffer`         | Owned byte buffer with `.bytes` / `.len` accessors, `clear()`, and appending encodes              |
| `ConvertError`      | Opaque error with `.kind` (`ConvertErrorKind`) and `.message` accessors                           |
| `MltWireVersion`    | `V01` (default) or the experimental `V02`                                                         |
| `MltGeometryType`   | The geometry type of one `MltLayerBuilder` feature                                                |
| `ConvertErrorKind`  | `InvalidInput`, `EncodingFailed` or `InvalidFeature`                                              |

## Usage examples

The round-trip tests are the primary documentation for each language:

- **C** - [`tests/c/test_round_trip.c`](tests/c/test_round_trip.c)
- **C++** - [`tests/cpp/test_round_trip.cpp`](tests/cpp/test_round_trip.cpp)
- **Kotlin** - [`tests/kotlin/TestRoundTrip.kt`](tests/kotlin/TestRoundTrip.kt)
- **Java** - [`tests/java/org/maplibre/mlt/ffi/RoundTripTest.java`](tests/java/org/maplibre/mlt/ffi/RoundTripTest.java)

## Building

```sh
cargo build --release -p mlt-ffi
```

## Regenerating bindings

```sh
just rust::sync-ffi-bindings
```

Requires `diplomat-tool`, `clang-format`, `ktlint`, and `jextract`.

## Running tests

```sh
just rust::test-ffi-c
just rust::test-ffi-cpp
just rust::test-ffi-kotlin
just rust::test-ffi-java
```
