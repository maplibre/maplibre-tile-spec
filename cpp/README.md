# maplibre-tile-spec/cpp

A C++ implementation of the MapLibre Tile (MLT) vector tile format.

## Status

CMake and Bazel build support.

## Build

```bash
git submodule update --init --recursive
cmake -GNinja -Bbuild -S.
cmake --build build --target mlt-cpp-test mlt-cpp-json
```

## Test

```
build/test/mlt-cpp-test
```

## Use

To decode a tile:

- Deserialize the tileset metadata
- Create a `Decoder`
- Call `decode` with the metadata and a view on the raw tile data

To use the standard packing of metadata and data within a tile, use `decodeTile`.

```cpp
#include <mlt/decoder.hpp>
#include <mlt/metadata/tileset.hpp>

...

auto metadata = mlt::metadata::tileset::read({metadataBuffer.data(), metadataBuffer.size()});

mlt::Decoder decoder;
const auto tile = decoder.decode({mltBuffer.data(), mltBuffer.size()}, metadata);

const auto tile2 = decoder.decodeTile({tileData.data(), tileData.size()});
```

### Invalid tiles

Tiles are treated as untrusted input. Decoding a tile which is malformed throws a `std::runtime_error`.
The sizes in a tile can be far larger than the tile, e.g. when run-length encoded, so decoding is also limited to a
memory budget of 64 MiB per tile, as with the Rust decoder. Use `Decoder::setMemoryLimit` to change it.

## Tools

A simple application which dumps a tile/metadata file pair to JSON format.

```bash
build/tool/mlt-cpp-json ../test/expected/0x01/bing/4-12-6.mlt
```

## Fuzzing

The decoder can be fuzzed with [libFuzzer](https://llvm.org/docs/LibFuzzer.html), which requires Clang.
There is a target for whole tiles, and one for the FastPFOR streams within them.

```bash
CXX=clang++ cmake -Bbuild-fuzz -S. -DCMAKE_BUILD_TYPE=RelWithDebInfo \
    -DMLT_WITH_FUZZERS=ON -DMLT_WITH_TESTS=OFF -DMLT_WITH_TOOLS=OFF \
    -DCMAKE_CXX_FLAGS="-fsanitize=fuzzer-no-link,address,undefined -fno-sanitize-recover=undefined"
cmake --build build-fuzz
mkdir corpus
build-fuzz/fuzz/mlt-cpp-fuzz-tile corpus ../test/expected/0x01/simple -max_len=65536 -rss_limit_mb=2000
```

Inputs which crashed the decoder belong in `test/test_robustness.cpp`.

## Implementation Notes

### Encoder

- The C++ encoder supports detecting nullable columns in cases where the Java implementation (as of 04/2026) always encodes nullable columns with trivial presence streams.  The `EncoderConfig.forceNullableColumns` option must be set in some cases for parity.
- Java uses closed rings in generateMixed because it is building JTS Polygon objects, and JTS polygon rings are expected to be closed. But when Java converts geometry to MLT streams, it deliberately strips the duplicated closing vertex, to be synthesized on decoding.  The C++ encoder does not use JTS and encodes the geometry as provided, so closing vertices should not be provided.
- Like Java and Rust, the C++ encoder winds polygon rings as MVT does: an exterior ring has a positive area by the surveyor's formula, and a hole a negative one. A ring that winds against that is reversed, keeping its first vertex, before it is stored and before it is tessellated. Rings that enclose no area are left alone.
