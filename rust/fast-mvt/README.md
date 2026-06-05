# fast-mvt

`fast-mvt` is an integer-only Mapbox Vector Tile reader and writer for Rust.
Geometry uses `geo-types` with `i32` coordinates. The crate does not project,
scale, round, or handle floating point geometry coordinates; callers provide and
receive tile-space integers.

## Features

The default feature set is `reader`, `writer`, and `json`.

| Feature     | Purpose                                                                 |
|-------------|-------------------------------------------------------------------------|
| `reader`    | Decode MVT bytes and inspect layers, features, geometries, and tags.    |
| `writer`    | Encode owned tile values, using `dup-indexer` for key/value dictionaries. |
| `json`      | Enable serde JSON support for the generated protobuf bindings.          |
| `views`     | Enable generated zero-copy protobuf views used by `reader`.             |
| `codegen`   | Regenerate checked-in protobuf bindings from `src/vector_tile.proto`.   |
| `arbitrary` | Derive `arbitrary::Arbitrary` for generated protobuf types for fuzzing. |

The generated protobuf files are checked in, so normal builds do not require
`protoc`. Run `just autofix` in this directory to refresh generated code after
`buffa` upgrades.

## Decode traversal

```rust
use fast_mvt::{MvtReaderRef, MvtResult};

fn read_tile(bytes: &[u8]) -> MvtResult<()> {
    let reader = MvtReaderRef::new(bytes)?;

    for layer in reader.layers() {
        for feature in layer.features() {
            let geometry = feature.geometry()?;
            println!("geo: {geometry:?}");
            let id = feature.id();
            println!("id: {id:?}");

            for property in feature.properties() {
                let (key, value) = property?;
                println!("{key} = {value:?}");
            }
        }
    }

    Ok(())
}
```

## Benchmarks

Decoder benchmark decodes all supported fixture tiles and iterate over all
elements. Run the benchmarks with `just bench-decode`.

| Decoder      |    Time |     Compare |
|--------------|--------:|------------:|
| `fast-mvt`   |  453 ms |           - |
| `mvt-reader` | 1165 ms | 157% slower |

| Encoder    |     Time |      Compare |
|------------|---------:|-------------:|
| `fast-mvt` |   685 ms |            - |
| `mvt`      | 12.54 s  | 1731% slower |

## Credits

This crate is derived from and informed by several open source MVT
implementations:

- `mvt` by the Minnesota Department of Transportation provided the encode-side
  tile/layer/feature structure and MVT geometry command conformance tests.
- `mvt-reader` by Paul Lange provided the checked-in codegen pattern,
  layer metadata behavior, tag parsing validation, and decode-side geometry
  structure.
- `geozero` by Pirmin Kalberer and contributors provided the `dup-indexer`
  dictionary pattern, integer command helpers, and integer polygon area
  orientation approach.
- `maplibre-tile-spec` `mlt-core` MVT tests provided fixture round-trip
  coverage and ring-close behavior.

The implementation here has been adapted for this crate's integer-only and
low-allocation API goals.
