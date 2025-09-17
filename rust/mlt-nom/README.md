## Tile Structure
Top level structure of a tile is a sequence of layers, where each layer consists of `(size, tag, data)` tuples:
- `size: varint` - size of the data block in bytes, including the size of the `tag` field
- `tag: varint` - identifies the block type, e.g. `0x01 = feature table v1`, `0x02 = raster layer`, `0x03 = routing table`, etc. We only define `0x01` for now.
- `data: u8[]` - the actual data block of the specified size

This approach allows us to easily extend the format in the future by adding new block types, while keeping backward compatibility. Parsers can skip unknown block types by reading the `size` and moving forward accordingly. For now, we only define `0x01` for vector layers, and possibly a few more if needed.

Note the ordering -- `tag` is after the `size` because it is possible to treat it as a single byte for now until the parser supports more than 127 types, and can efficiently skip unknown ones without doing a more expensive varint parsing.

## Layer 0x01 - MVT compatibility
Structure of the data if the `tag` above is 0x01. We should focus this tag on MVT compatibility, offering exactly what we had in MVT, but allowing for a clearly defined set of encodings and other optimizations like tesselation.  No new data formats (per vertix data, nested data, 3d geometries, etc).  No extendable encodings - once finalized, 0x01 will only allow what has been specified. This will ensure that if a decoder declares "0x01" support, it will parse every specification-compliant 0x01 layer. For any new features and encodings we will simply use a new tag ID, likely reusing most of the existing encoding/decoding code.

- `name: string` - Name of the layer
- `columnCount: varint` - Number of columns in the layer
- each column is defined as:
    - `columnType: varint` - same idea as `tag` above, e.g. `1 = id`, `2 = geometry`, `3 = int property`, etc.
    - TODO...
