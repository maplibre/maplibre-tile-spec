# maplibre-tile-spec/cpp

A C++ implementation of the MapLibre Tile (MLT) vector tile format.

## Status

Decoder only, partial support for encodings.

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

## Tools

A simple application which dumps a tile/metadata file pair to JSON format.

```bash
build/tool/mlt-cpp-json ../test/expected/tag0x01/bing/4-12-6.mlt
```
