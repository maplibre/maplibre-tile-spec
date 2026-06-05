# fast-mvt

`fast-mvt` is an integer-only Mapbox Vector Tile reader and writer for Rust.
Geometry uses `geo-types` with `i32` coordinates. The crate does not project,
scale, round, or handle floating point geometry coordinates; callers provide and
receive tile-space integers.

## Features

| Feature     | Purpose                                                                 |
|-------------|-------------------------------------------------------------------------|
| `reader`    | MVT tile decoding from bytes.                                           |
| `writer`    | MVT tile encoding into bytes.                                           |
| `json`      | Enable serde JSON support for the generated protobuf bindings.          |
| `codegen`   | Regenerate checked-in protobuf bindings from `src/vector_tile.proto`.   |
| `arbitrary` | Derive `arbitrary::Arbitrary` for generated protobuf types for fuzzing. |
| `views`     | Internal feature, do not use. Must be here due to buffa limitations.    |

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

## Encode builder

```rust
use fast_mvt::{MvtGeometry, MvtResult, MvtTileBuilder};

fn write_tile() -> MvtResult<Vec<u8>> {
    let tile = MvtTileBuilder::new();
    let layer = tile.layer("places");

    let mut feature = layer.feature(MvtGeometry::Point((1, 2).into()))?;
    feature.id(7);
    feature.tag("name", "Example")?;
    feature.tag("visible", true)?;
    let layer = feature.finish();

    let tile = layer.finish();
    Ok(tile.finish())
}
```

Opening a layer consumes the tile builder, and opening a feature consumes the
layer builder. Finishing returns the parent builder with the child committed, so
there is no reachable partially committed layer or tile while a child is in
progress. A single-layer tile byte buffer is also a framed layer chunk, so
multiple independently built layer buffers can be concatenated to form a tile.

### Benchmarks

Decoder benchmark decodes all supported fixture tiles and iterate over all
elements. Run the benchmarks with `just bench-decode`.

| Decoder      |    Time |     Compare |
|--------------|--------:|------------:|
| `fast-mvt`   |  453 ms |           - |
| `mvt-reader` | 1165 ms | 157% slower |

| Encoder    |     Time |      Compare |
|------------|---------:|-------------:|
| `fast-mvt` |   987 ms |            - |
| `mvt`      | 11549 ms | 1070% slower |

### Development

* This project is easier to develop with [just](https://just.systems/man/en/), a modern alternative to `make`.
* To get a list of available commands, run `just`.
* To run tests, use `just test`.

## License

* All code is dual licensed under [Apache License v2.0](http://www.apache.org/licenses/LICENSE-2.0) and [MIT license](http://opensource.org/licenses/MIT), at your option.

### Credits

This crate took some ideas from several open source MVT implementations:

- `mvt` by the Minnesota Department of Transportation provided the encode-side
  tile/layer/feature structure and MVT geometry command conformance tests.
- `mvt-reader` by Paul Lange provided the checked-in codegen pattern,
  layer metadata behavior, tag parsing validation, and decode-side geometry
  structure.

### Contribution

Unless you explicitly state otherwise, any code contribution intentionally
submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
