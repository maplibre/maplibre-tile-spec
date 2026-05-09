# mlt-ffi

[Diplomat](https://github.com/rust-diplomat/diplomat)-based FFI bindings for
`mlt-core`, providing MLT <-> MVT conversion from C, C++, Kotlin, and Java.

## API

| Type               | Purpose                                                          |
| ------------------ | ---------------------------------------------------------------- |
| `MltConverter`     | `mlt_to_mvt(bytes)` and `mvt_to_mlt(bytes, encoder_options)` conversion |
| `MltEncoderOptions`| Builder wrapping `EncoderConfig` — construct with `new()`, toggle flags with setters |
| `MltBuffer`        | Owned result buffer with `.bytes` / `.len` accessors             |
| `ConvertError`     | `InvalidInput` or `EncodingFailed`                               |

## Usage examples

The round-trip tests are the primary documentation for each language:

- **C** — [`tests/c/test_round_trip.c`](tests/c/test_round_trip.c)
- **C++** — [`tests/cpp/test_round_trip.cpp`](tests/cpp/test_round_trip.cpp)
- **Kotlin** — [`tests/kotlin/TestRoundTrip.kt`](tests/kotlin/TestRoundTrip.kt)
- **Java** — [`tests/java/TestRoundTrip.java`](tests/java/TestRoundTrip.java)

## Building

```sh
cargo build --release -p mlt-ffi
```

## Regenerating bindings

```sh
just rust::sync-ffi-bindings
```

Requires `diplomat-tool`, `clang-format`, and `ktlint`.

## Running tests

```sh
just rust::test-ffi-c
just rust::test-ffi-cpp
just rust::test-ffi-kotlin
just rust::test-ffi-java
```
