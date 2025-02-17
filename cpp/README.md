# maplibre-tile-spec/cpp

A C++ implementation of the MapLibre Tile (MLT) vector tile format.

## Status

Decoder only, partial support for encodings.

CMake only build support.

## Build

```bash
cmake -GNinja -Bbuild -S. && cmake --build build --target mlt-cpp-test mlt-cpp-json
```

## Use

To decode a tile:

- Deserialize the protobuf-encoded tileset metadata
- Create a `Decoder`
- Call `decode` with the metadata and a view on the raw tile data

```cpp
#include <mlt/decoder.hpp>
#include <mlt/metadata/tileset.hpp>

// If using protozero to decode protobuf-encoded tileset metadata
#include <mlt/metadata/tileset_protozero.hpp>

...

auto metadataBuffer = ...
auto metadata = mlt::metadata::tileset::read({metadataBuffer.data(), metadataBuffer.size()});

mlt::Decoder decoder;
const auto tileData = decoder.decode({buffer.data(), buffer.size()}, metadata);
```

## Tools

A simple application which dumps a tile/metadata file pair to JSON format.

```bash
build/tool/mlt-cpp-json ../test/expected/bing/4-12-6.mlt
```
