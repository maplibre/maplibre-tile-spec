# Maplibre Tiles RS

Notes:
* [This](https://baarse.substack.com/p/fast-parquet-reading-from-java-to) may be useful as parquet has similar format
* [proto_tileset.rs](src/metadata/proto_tileset.rs) is a generated via prost - see [build.rs](build.rs)

---

## Structure Overview

```
rust/mlt/
├── src/
│   ├── converter/     # MLT–MVT conversion
│   ├── data/          # Core in-memory tile structures
│   ├── decoder/       # Decoder
│   ├── encoder/       # Encoder
│   ├── metadata/      # Spec definitions for streams, tilesets, proto structs
│   ├── vector/        # SIMD/vector helper types and ops
│   ├── error.rs       # MltError, MltResult
│   ├── lib.rs         # Crate root and public API
│   └── README.md
├── build.rs
├── Cargo.toml
├── justfile           # Job runner
└── README.md
```

---

## Running Tests and Tasks

This crate uses [`just`](https://github.com/casey/just) as a task runner. Run the following from inside the `rust/mlt` directory:

```sh
# Run all tests
just test

# Format files
just fmt

# Lint check
just clippy
```

You can also run a specific test directly:

```sh
cargo test test_decode
```
