# MapLibre Tile Specification v1

This document specifies the byte layout of an MLT v1 layer. The data model it assumes, tiles, extents, layers, features, columns and streams, is described on the [data model](<https://maplibre.org/maplibre-tile-spec/overview/index.md>) page.

## Tile Layout

A tile is a concatenation of tagged layer records. There is no tile header. Layers are encoded independently and can be concatenated without re-encoding.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01%2Fpoint.mlt&amp;at=size>) - a single `Point` and the layer around it.

```text
tile   := layer*
layer  := [varint size] [u8 tag] [u8 body[size - 1]]
```

- `size` counts the `tag` byte plus the body, so the next layer starts `size` bytes after the `size` varint.
- `tag` names the format the body is written in. `0x01` is the format specified on this page.
- A decoder MUST skip a layer whose `tag` it does not recognize, using `size` to find the next one.

A decoder MUST NOT assume layer names are unique or ordered.

## Layer Body

A v1 layer body is a header, then every column's metadata, then every column's data:

```text
body := [string name]
        [varint extent]
        [varint column_count]
        column_meta * column_count
        column_data * column_count
```

Strings are a `VarInt` byte length followed by that many UTF-8 bytes:

```text
string := [varint length] [u8 bytes[length]]
```

`name` MUST NOT be empty.

`extent` defines the coordinate space size for the tile's geometry and MUST NOT be zero. Geometry coordinates are signed integers in vector-tile grid coordinates, typically near the `0..=extent` range but not restricted to it. Values MAY be negative or exceed `extent` for geometry that crosses tile boundaries. Encoders MAY default to `4096` when a user does not specify the extent.

Metadata and data are in two separate sections, both in `column_count` order.

A geometry column and a nullable string column `name`:

A layer MUST contain exactly one geometry column and at most one id column.

### Column Metadata

Each column's metadata is a single type byte, optionally followed by a name and, for shared dictionaries, its children:

```text
column_meta := [u8 column_type]
               [string name]                              -- unless the type is Id or Geometry
               [varint child_count] [child_meta*]         -- SharedDict only
child_meta  := [u8 column_type] [string name]
```

Id and Geometry columns are named implicitly by their type and carry no name field.

#### Column Types

The low bit of the type byte is the nullability flag: an odd code is the nullable variant of the even code below it. A nullable column writes a `Present` stream before its data.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01-rust%2Fprops_mixed_np.mlt>) - seven of the column types in one layer.

| Code | Type | Nullable variant | Description |
| ---: | --- | ---: | --- |
| `0` | `Id` | `1` (`OptId`) | Feature id, up to 32 bits |
| `2` | `LongId` | `3` (`OptLongId`) | Feature id, up to 64 bits |
| `4` | `Geometry` | \- | The layer's geometry column, never nullable |
| `10` | `Bool` | `11` |  |
| `16` | `Int32` | `17` |  |
| `18` | `UInt32` | `19` |  |
| `20` | `Int64` | `21` |  |
| `22` | `UInt64` | `23` |  |
| `24` | `Float` | `25` | IEEE 754 binary32 |
| `26` | `Double` | `27` | IEEE 754 binary64 |
| `28` | `String` | `29` | UTF-8 |
| `30` | `SharedDict` | \- | A string dictionary and the columns that index into it |

Codes `5`-`9`, `12`-`15` and `31` are unassigned. A decoder MUST reject a column whose type byte is unassigned.

### Column Data

What follows in the data section depends on the column's type. Every `[stream]` below is one stream, laid out as described under [streams](<#streams>).

| Column type | Data layout |
| --- | --- |
| `Id`, `LongId` | `[data stream]` |
| `Geometry` | `[varint stream_count]` `[types stream]` `[stream * (stream_count - 1)]` |
| Scalar (`Bool` ... `Double`) | `[data stream]` |
| `String` | `[varint stream_count]` `[stream * stream_count]` |
| `SharedDict` | see [shared dictionary columns](<#shared-dictionary-columns>) |

A nullable column prefixes its data with a `Present` stream, which is counted in `stream_count` where one is present.

## Streams

A logical column is separated into several physical `streams` (sub-columns), inspired by the ORC file format. These streams are stored contiguously. A stream is a sequence of values of a known length in a continuous memory chunk, all sharing the same type.

Each stream is a header followed by its payload:

```text
stream := [u8 stream_type]        category (bits 7-4) | subtype (bits 3-0)
          [u8 encoding]           logical1 (bits 7-5) | logical2 (bits 4-2) | physical (bits 1-0)
          [varint num_values]
          [varint byte_length]
          [varint runs]           RLE streams only, except Present streams
          [varint num_rle_values] RLE streams only, except Present streams
          [varint bits]           Morton streams only
          [varint shift]          Morton streams only
          [u8 payload[byte_length]]
```

The header of the vertex stream of `line`:

`num_values` is the number of values the stream holds after decoding. `byte_length` is the payload size and allows a decoder to skip the stream.

A `Present` stream is always boolean and derives its RLE parameters from `num_values`.

### Stream Types

The `stream_type` byte names what role the stream plays. The high nibble is the category and the low nibble is a category-specific subtype.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01%2Fprops_shared_dict_one_child_fsst.mlt>) - all four categories, across nine subtypes.

| Category | Code | Subtypes |
| --- | ---: | --- |
| `Present` | `0x0` | \- |
| `Data` | `0x1` | `0` None, `1` Single dictionary, `2` Shared dictionary, `3` Vertex, `4` Morton, `5` FSST |
| `Offset` | `0x2` | `0` Vertex, `1` Index, `2` String, `3` Key |
| `Length` | `0x3` | `0` VarBinary, `1` Geometries, `2` Parts, `3` Rings, `4` Triangles, `5` Symbol, `6` Dictionary |

What each category holds:

- **Present**: enables efficient encoding of sparse columns by indicating value presence via a bit flag. Omitted when the column is not nullable.
- **Data**: the actual column data - `boolean`, `int`, `float` or `string` values, dictionary entries, or geometry coordinates. For fixed-size data types this is the only required stream besides the optional `Present` stream.
- **Length**: the number of elements for variable-sized data types like strings or rings.
- **Offset**: offsets into a data stream when using dictionary encoding, for strings or vertices.

### Encoding Byte

The encoding byte gives the encoding of the payload. The two logical fields form one combination. Any combination not listed below is invalid.

| Combination | logical1 (bits 7-5) | logical2 (bits 4-2) | Meaning |
| --- | --- | --- | --- |
| `None` | `000` | `000` | Values as they are |
| `Delta` | `001` | `000` | ZigZag deltas between consecutive values |
| `DeltaRle` | `001` | `011` | Delta, then RLE |
| `ComponentwiseDelta` | `010` | `000` | Deltas per coordinate component, for vertex streams |
| `Rle` | `011` | `000` | Run-length encoded |
| `MortonDelta` | `100` | `001` | Deltas between the Morton (Z-order) codes of a sorted vertex dictionary |

The physical field (bits 1-0) says how the resulting integers are laid out in bytes:

| Code | Physical |
| --- | --- |
| `00` | None - fixed-width little-endian words |
| `01` | SIMD-FastPFOR, 256-value big-endian blocks |
| `10` | VarInt |

RLE streams store all run lengths first, then all values, and carry `runs` and `num_rle_values` in the header.

The algorithms themselves are specified in [encoding definitions](<https://maplibre.org/maplibre-tile-spec/encodings/index.md>).

## Property Columns

### ID Column

An `id` column is not mandatory. If included, it should be `UInt64` or narrower (`UInt32` if possible) for MVT compatibility. A narrower type enables the use of efficient encodings like SIMD-FastPFOR.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01%2Fid.mlt&amp;at=column%5B0%5D>) - a 32-bit `Id` column.

### Scalar Columns

Boolean, integer and floating-point columns are a `Present` stream when nullable, then one data stream. Boolean data streams are bit-packed, one bit per present value; float and double data streams hold fixed-width IEEE 754 words.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01%2Fprop_i32.mlt&amp;at=column%5B1%5D>) - an `i32` column and its presence stream.

### String Columns

A string column declares how many streams follow, and the number of streams determines its layout:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01%2Fprops_str.mlt&amp;at=column%5B1%5D>) - the byte length of each value, then the values back to back.

| Streams | Layout | Stream order |
| ---: | --- | --- |
| 2 | Plain | `Length/VarBinary`, `Data/None` |
| 3 | Dictionary | `Length/Dictionary`, `Offset/String`, `Data/Single` |
| 4 | FSST | `Length/Symbol`, `Data/FSST`, `Length/Dictionary`, `Data/Single` |
| 5 | FSST dictionary | `Length/Symbol`, `Data/FSST`, `Length/Dictionary`, `Data/Single`, `Offset/String` |

A nullable column's `Present` stream precedes these and is counted in `stream_count`.

In the plain layout the `Length` stream holds the byte length of each present value and the `Data` stream holds their UTF-8 bytes back to back. In a dictionary layout the `Offset/String` stream holds one dictionary index per present value, and the `Length` and `Data` streams describe the distinct values. FSST layouts compress the value bytes with a symbol table; see [FSST](<https://maplibre.org/maplibre-tile-spec/encodings/#fsst>).

The offset stream comes last in the 5-stream layout and before the data stream in the 3-stream layout. An encoder MAY also use the 5-stream layout for an undeduplicated FSST corpus, writing the identity `[0, 1, 2, ...]` as its offsets.

`city` over four features, null on feature 3:

The same column in each layout, then in byte order:

### Shared Dictionary Columns

Several string columns, such as `name:en`, `name:de` and `name:fr`, can share a single dictionary.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01%2Fprops_shared_dict.mlt&amp;at=column%5B1%5D>) - several string columns sharing one dictionary.

```text
shared_dict := [varint stream_count]
               [dictionary streams]     2 (plain) or 4 (FSST), as in the table above
               child * child_count      child_count comes from the column metadata
child       := [varint stream_count]
               [present stream]         only when the child's type is nullable
               [offset stream]          one dictionary index per present value
```

`stream_count` on the column counts every stream that follows: the dictionary streams, one offset stream per child, and one present stream per nullable child. The dictionary streams end at the `Data/Single` or `Data/Shared` stream. The children begin after it.

> [!NOTE]
>
> Some tiles in the wild were written with `stream_count` one too high, by an encoder bug since fixed. Decoders SHOULD accept `expected + 1` as well so those files still parse.

Each child's own `stream_count` is `1`, plus `1` when it is nullable.

`name:de` and `name:en` over four features, `name:en` null on feature 3:

The same two columns, stored with each kind of dictionary, then in byte order:

## Geometry Column

The geometry column uses a Structure of Arrays (SoA) layout (data-oriented design). The `x`, `y` coordinates are stored interleaved in a `VertexBuffer` for efficient CPU processing and direct copying to GPU buffers.

The geometry information is separated into different streams, partly inspired by the [geoarrow](<https://github.com/geoarrow/geoarrow>) specification. This separation enables better compression optimization and faster processing. Pre-tessellated polygon meshes can also be stored directly to avoid runtime triangulation.

A geometry column can consist of the following streams:

| Stream Name | Data Type | Encoding | Mandatory |
| --- | :---: | --- | :---: |
| GeometryType | Byte | Integer | ✓ |
| NumGeometries | UInt32 | Integer |  |
| NumParts | UInt32 | Integer |  |
| NumRings | UInt32 | Integer |  |
| NumTriangles | UInt32 | Integer |  |
| IndexBuffer | UInt32 | Integer |  |
| VertexOffsets | UInt32 | Integer |  |
| VertexBuffer | Int32 or Vertex\[\] | Plain, Dictionary, Morton | ✓ |

Depending on the geometry type, the following streams are used in addition to `GeometryType`:

- **Point**: VertexBuffer
- **LineString**: NumParts, VertexBuffer
- **Polygon**: NumParts (Polygon), NumRings (LinearRing), VertexBuffer
- **MultiPoint**: NumGeometries, VertexBuffer
- **MultiLineString**: NumGeometries, NumParts (LineString), VertexBuffer
- **MultiPolygon**: NumGeometries, NumParts (Polygon), NumRings (LinearRing), VertexBuffer

When LineString and Polygon types are mixed in the same column, LineString vertex counts are stored in the NumRings stream (see [length stream encoding rules](<#length-stream-encoding-rules>) below).

An additional `VertexOffsets` stream is present when using Dictionary or Morton-Dictionary encoding. If geometries (mainly polygons) are pre-tessellated for direct GPU use, `NumTriangles` and `IndexBuffer` streams must be provided.

### Geometry Types

Six geometry types are supported, encoded as unsigned integers:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01%2Fmix_6_pt_line_poly_mpt_mline_mpoly.mlt&amp;at=meta>) - all six kinds in one layer.

| Value | Type | Description |
| --- | --- | --- |
| 0 | Point | Single coordinate |
| 1 | LineString | Sequence of coordinates forming a line |
| 2 | Polygon | Closed rings (exterior + optional interior holes) |
| 3 | MultiPoint | Collection of points |
| 4 | MultiLineString | Collection of line strings |
| 5 | MultiPolygon | Collection of polygons |

### Polygon Rings

Rings take their roles from their order: the first ring of each polygon is its exterior, and any rings after it are its holes. The [topology streams](<#topology-encoding>) say which rings belong to which polygon. Decoders MUST take each polygon's exterior and holes from the topology streams, not from winding.

A polygon ring is stored without its closing vertex, and its ring length does not count it. An encoder MUST NOT store the closing vertex. A decoder that returns polygon rings MUST close each non-empty ring by repeating its first vertex at the end.

**Winding carries no meaning**: each ring MAY be wound either way, independently of the others. Unlike [MVT](<https://github.com/mapbox/vector-tile-spec/tree/master/2.1#4344-polygon-geometry-type>), MLT does not require exteriors to have a positive area or holes a negative one.

### Binary Structure

A geometry column is stored as a sequence of streams, prefixed by a stream count:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01%2Fpoly.mlt&amp;at=column%5B0%5D>) - a polygon, with the topology streams v1 writes.

```text
geometry_column := [varint stream_count]
                   [types stream]                  one geometry type per feature
                   [stream * (stream_count - 1)]   topology, tessellation and vertex streams
```

The streams after the types stream are identified by their `stream_type`, not their position. Each appears at most once.

The streams listed above map to the following physical and logical stream types:

| Physical Type | Logical Type | Specification Name | Content |
| --- | --- | --- | --- |
| `DATA` | `NONE` | GeometryType | Geometry type per feature (see [geometry types](<#geometry-types>)) |
| `LENGTH` | `GEOMETRIES` | NumGeometries | Number of sub-geometries in Multi\* types |
| `LENGTH` | `PARTS` | NumParts | Number of rings per polygon or lines per multi-linestring |
| `LENGTH` | `RINGS` | NumRings | Number of vertices per ring or per linestring segment |
| `LENGTH` | `TRIANGLES` | NumTriangles | Number of triangles per polygon (tessellated) |
| `OFFSET` | `INDEX` | IndexBuffer | Triangle vertex indices (tessellated) |
| `OFFSET` | `VERTEX` | VertexOffsets | Indices into vertex dictionary |
| `DATA` | `VERTEX` | VertexBuffer | Vertex coordinates (x, y pairs) |
| `DATA` | `MORTON` | VertexBuffer | Morton-encoded vertex coordinates |

### Topology Encoding

MLT uses a **length-based** encoding for geometry topology rather than explicit drawing commands. This enables efficient random access to individual features.

#### Conceptual Hierarchy

Each length counts entries of the level below it: geometries count parts, parts count rings, and rings count vertices.

A `Polygon` with a hole, then a `MultiPolygon` of a polygon with a hole and a triangle, from `mix_2_polyh_mpoly`.

#### Which Streams Are Present

The streams included depend on the geometry types in the column:

| Geometry Type | Geometries | Parts | Rings |
| --- | :---: | :---: | :---: |
| Point | \- | \- | \- |
| MultiPoint | ✓ | \- | \- |
| LineString | \- | ✓\* | \- |
| MultiLineString | ✓ | ✓\* | \- |
| Polygon | \- | ✓ | ✓ |
| MultiPolygon | ✓ | ✓ | ✓ |

\*LineString and MultiLineString parts are stored in the Rings stream when Polygons are also present in the same column.

#### Length Stream Encoding Rules

Length values are stored only for geometry types that need them. The key insight is that **simple types have an implicit count of 1**, while **Multi\* types store their sub-geometry counts explicitly**.

The encoding uses a **type threshold** to determine which geometries need explicit lengths:

| Stream | Threshold | Types Needing Explicit Length |
| --- | --- | --- |
| `geometry_offsets` | Polygon | MultiPoint, MultiLineString, MultiPolygon |
| `part_offsets` (Rings stream present) | LineString | Polygon, MultiPoint, MultiLineString, MultiPolygon |
| `part_offsets` (no Rings stream) | Point | LineString |

**Rule**: If a geometry type's value is greater than the threshold type's value (see [geometry types](<#geometry-types>) table), store its length explicitly. Otherwise, the length is implicitly 1.

**Example**: A column with `[Point, MultiPolygon, Polygon]` geometry types:

```text
Encoded geometries lengths: [3]     // Only MultiPolygon needs explicit count
                                    // (MultiPolygon=5 > Polygon=2)

Decoding to offsets:
  Point:        implicit 1  -> offset 0->1   (Point=0, not greater than Polygon=2)
  MultiPolygon: explicit 3  -> offset 1->4   (MultiPolygon=5 > Polygon=2, read from stream)
  Polygon:      implicit 1  -> offset 4->5   (Polygon=2, not greater than Polygon=2)

Result: geometry_offsets = [0, 1, 4, 5]
```

### Vertex Encoding

#### Componentwise Delta Encoding

Vertices are stored as interleaved (x, y) coordinate pairs using **componentwise delta encoding**:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01%2Fline.mlt&amp;at=stream%5B1%5D>) - three vertices, each stored as its step from the one before.

**Encoding steps:**

1. Track previous X and previous Y separately (both start at 0)
2. For each vertex, compute `delta_x = x - prev_x` and `delta_y = y - prev_y`
3. Apply [zigzag encoding](<https://en.wikipedia.org/wiki/Variable-length_quantity#Zigzag_encoding>): `zigzag(n) = (n << 1) ^ (n >> 31)` (maps negatives to positives)
4. Output as [varint](<https://en.wikipedia.org/wiki/Variable-length_quantity>): `[zigzag(Δx₀), zigzag(Δy₀), zigzag(Δx₁), zigzag(Δy₁), ...]`

**Decoding steps:**

1. Read varints in pairs
2. Apply zigzag decoding: `n = (zigzag >> 1) ^ -(zigzag & 1)`
3. Accumulate: `x = prev_x + delta_x`, `y = prev_y + delta_y`

#### Dictionary Encoding (Optional)

When vertices repeat frequently, a dictionary encoding may be used:

The vertex dictionary is sorted by [Hilbert curve](<https://en.wikipedia.org/wiki/Hilbert_curve>) index for spatial locality. The `OFFSET/VERTEX` stream contains indices into this dictionary.

#### Morton Encoding (Optional)

For spatial optimization, vertices can be [Morton-encoded](<https://en.wikipedia.org/wiki/Z-order_curve>) (Z-order curve):

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01-rust%2Fmultipoint_morton_dictionary.mlt&amp;at=stream%5B2%5D>) - vertices as Morton codes.

```text
Coordinate (5, 3):
  X bits: 1 0 1
  Y bits: 0 1 1
  Interleaved: 01 10 11 = 011011₂ = 27

Stored as: Morton code + metadata (numBits, coordinateShift)
```

Morton encoding enables efficient spatial clustering and range queries.

### Integer Stream Encoding

Topology streams (lengths, offsets) use adaptive encoding selected for minimal size. All integer encodings use [variable-length quantity (varint)](<https://en.wikipedia.org/wiki/Variable-length_quantity>) as the base encoding:

| Logical Encoding | Format | Best For |
| --- | --- | --- |
| None | Plain varints | Random values |
| Delta | Delta + zigzag + varint | Monotonic sequences |
| RLE | Run-length encoded | Repeated values |
| DeltaRLE | Delta + RLE | Constant increments |

#### RLE Format

[Run-length encoding](<https://en.wikipedia.org/wiki/Run-length_encoding>) stores runs followed by values:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01%2Fids_rle.mlt&amp;at=column%5B0%5D>) - four equal ids as one run: the run length `4`, then the value `103`.

```text
Input:  [5, 5, 5, 3, 3, 3, 3]
Runs:   [3, 4]        // 3 fives, 4 threes
Values: [5, 3]
Output: [3, 4, 5, 3]  // runs concatenated with values
```

Stream metadata includes `runs` and `num_rle_values` counts for decoding.

### Tessellation Data (Optional)

Pre-tessellated polygons include additional streams for direct GPU rendering:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x01%2Fmix_2_poly_mpoly_tes.mlt&amp;at=stream%5B3%5D>) - the triangles and vertices drawn below.

- **Triangles** (`LENGTH/TRIANGLES`) hold one triangle count per polygon feature.
- **The index buffer** (`OFFSET/INDEX`) holds three vertex indices per triangle.

Each index counts from the first vertex of its own feature.

A `Polygon` triangle, then a `MultiPolygon` of a polygon with a hole and a second triangle, from `mix_2_poly_mpoly_tes`.

### Decoding Examples

```text
Streams:
  META: types = [0]          // Point
  DATA/VERTEX: [200, 400]    // zigzag-encoded deltas

Decode:
  zigzag(200) = 100, zigzag(400) = 200
  Result: Point(100, 200)
```

```text
Streams:
  META: types = [1]                 // LineString
  LENGTH/PARTS: [3]                 // 3 vertices
  DATA/VERTEX: [0,0, 200,0, 0,200]  // encoded deltas

Decode:
  part_offsets = [0, 3]                   // from lengths
  vertices = [(0,0), (100,0), (100,100)]  // delta-decoded
  Result: LineString with 3 vertices
```

```text
Streams:
  META: types = [2]          // Polygon
  LENGTH/PARTS: [2]          // 2 rings (exterior + 1 hole)
  LENGTH/RINGS: [4, 4]       // 4 vertices each ring
  DATA/VERTEX: [...]         // 8 vertices total

Decode:
  part_offsets = [0, 2]      // 1 polygon with 2 rings
  ring_offsets = [0, 4, 8]   // ring boundaries
  Result: Polygon with exterior ring (4 verts) and hole (4 verts)
```

```text
Streams:
  META: types = [5]          // MultiPolygon
  LENGTH/GEOMETRIES: [2]     // 2 polygons
  LENGTH/PARTS: [1, 1]       // 1 ring each
  LENGTH/RINGS: [4, 4]       // 4 vertices each
  DATA/VERTEX: [...]         // 8 vertices total

Decode:
  geometry_offsets = [0, 2]  // spans indices 0-1 in parts
  part_offsets = [0, 1, 2]   // each polygon has 1 ring
  ring_offsets = [0, 4, 8]   // vertex boundaries
  Result: MultiPolygon with 2 simple polygons
```

### Mixed Geometry Columns

A single column can contain mixed geometry types. The decoding logic handles this by:

1. Reading geometry types to determine which length streams to expect
2. Iterating through features, applying type-specific offset rules
3. Using implicit length=1 for simple types (Point, LineString, Polygon) and reading explicit lengths for Multi\* types

**Example**: `[Point, LineString, Polygon]` in one column:

```text
Streams:
  META: types = [0, 1, 2]
  LENGTH/PARTS: [1]          // Polygon: 1 ring
  LENGTH/RINGS: [3, 4]       // LineString: 3 verts, Polygon ring: 4 verts
  DATA/VERTEX: [...]         // 1 + 3 + 4 = 8 vertices

Decode:
  Point: vertices[0]
  LineString: vertices[1..4] // from ring_offsets (Polygon present)
  Polygon: vertices[4..8]    // from ring_offsets
```

The integer and vertex encodings referenced above are specified in detail in the [encoding definitions](<https://maplibre.org/maplibre-tile-spec/encodings/index.md>) document.

## Examples

Every fixture under `test/synthetic/0x01/` has an annotated hexdump and the `.json` a decoder MUST produce from it. The annotations are generated from the tiles.

The same annotation can be produced for any tile with the [`mlt` CLI](<https://github.com/maplibre/maplibre-tile-spec/blob/main/rust/mlt/README.md>):

```bash
mlt hexdump path/to/tile.mlt
```

> [!NOTE]
>
> **Annotated hexdumps**
>
> One point with one id: the tile envelope, the column metadata section and the data section.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (30 B)
> 00000000  1d                                                .                |   size: 29 (varint) - tag + body
> 00000001  01                                                .                |   tag: 0x01 -> Tag01
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  40                                                @                |   extent: 64
> 0000000a  02                                                .                |   column_count: 2
> 0000000b                                                                     |   column_schema[0] Id (1 B)
> 0000000b  00                                                .                |     type: 0x00 Id
>                                                                              |       └ bits 7-1 = 0000000 -> base type = Id
>                                                                              |       └ bit 0 = 0 -> not optional: each feature has a non-NULL value
> 0000000c                                                                     |   column_schema[1] Geometry (1 B)
> 0000000c  04                                                .                |     type: 0x04 Geometry
>                                                                              |       └ bits 7-1 = 0000010 -> base type = Geometry
>                                                                              |       └ bit 0 = 0 -> not optional: each feature has a non-NULL value
> 0000000d                                                                     |   column[0] Id (5 B)
> 0000000d                                                                     |     id (5 B)
> 0000000d  10                                                .                |       stream_type: 0x10 Data(None)
>                                                                              |         └ bits 7-4 = 0001 -> category = Data
>                                                                              |         └ bits 3-0 = 0000 -> subtype = None
> 0000000e  02                                                .                |       encoding: 0x02 logical=Int(None) physical=VarInt
>                                                                              |         └ bits 7-5 = 000 -> logical1 = None
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 0000000f  01                                                .                |       num_values: 1
> 00000010  01                                                .                |       byte_length: 1
> 00000011  64                                                d                |       data [Data(None) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |         decoded: [100]
> 00000012                                                                     |   column[1] Geometry (12 B)
> 00000012  02                                                .                |     stream_count: 2
> 00000013                                                                     |     meta (5 B)
> 00000013  30                                                0                |       stream_type: 0x30 Length(VarBinary)
>                                                                              |         └ bits 7-4 = 0011 -> category = Length
>                                                                              |         └ bits 3-0 = 0000 -> subtype = VarBinary
> 00000014  02                                                .                |       encoding: 0x02 logical=Int(None) physical=VarInt
>                                                                              |         └ bits 7-5 = 000 -> logical1 = None
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 00000015  01                                                .                |       num_values: 1
> 00000016  01                                                .                |       byte_length: 1
> 00000017  00                                                .                |       data [Length(VarBinary) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |         decoded: [0]
> 00000018                                                                     |     stream[0] Data(Vertex) (6 B)
> 00000018  13                                                .                |       stream_type: 0x13 Data(Vertex)
>                                                                              |         └ bits 7-4 = 0001 -> category = Data
>                                                                              |         └ bits 3-0 = 0011 -> subtype = Vertex
> 00000019  42                                                B                |       encoding: 0x42 logical=Vertex(ComponentwiseDelta) physical=VarInt
>                                                                              |         └ bits 7-5 = 010 -> logical1 = ComponentwiseDelta
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 0000001a  02                                                .                |       num_values: 2
> 0000001b  02                                                .                |       byte_length: 2
> 0000001c  1a 54                                             .T               |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 1 values, 2 B]
>                                                                              |         decoded: [13, 42]
> ```
>
> Part and ring length streams.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (47 B)
> 00000000  2e                                                .                |   size: 46 (varint) - tag + body
> 00000001  01                                                .                |   tag: 0x01 -> Tag01
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  40                                                @                |   extent: 64
> 0000000a  01                                                .                |   column_count: 1
> 0000000b                                                                     |   column_schema[0] Geometry (1 B)
> 0000000b  04                                                .                |     type: 0x04 Geometry
>                                                                              |       └ bits 7-1 = 0000010 -> base type = Geometry
>                                                                              |       └ bit 0 = 0 -> not optional: each feature has a non-NULL value
> 0000000c                                                                     |   column[0] Geometry (35 B)
> 0000000c  04                                                .                |     stream_count: 4
> 0000000d                                                                     |     meta (5 B)
> 0000000d  30                                                0                |       stream_type: 0x30 Length(VarBinary)
>                                                                              |         └ bits 7-4 = 0011 -> category = Length
>                                                                              |         └ bits 3-0 = 0000 -> subtype = VarBinary
> 0000000e  02                                                .                |       encoding: 0x02 logical=Int(None) physical=VarInt
>                                                                              |         └ bits 7-5 = 000 -> logical1 = None
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 0000000f  01                                                .                |       num_values: 1
> 00000010  01                                                .                |       byte_length: 1
> 00000011  02                                                .                |       data [Length(VarBinary) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |         decoded: [2]
> 00000012                                                                     |     stream[0] Length(Parts) (5 B)
> 00000012  32                                                2                |       stream_type: 0x32 Length(Parts)
>                                                                              |         └ bits 7-4 = 0011 -> category = Length
>                                                                              |         └ bits 3-0 = 0010 -> subtype = Parts
> 00000013  02                                                .                |       encoding: 0x02 logical=Int(None) physical=VarInt
>                                                                              |         └ bits 7-5 = 000 -> logical1 = None
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 00000014  01                                                .                |       num_values: 1
> 00000015  01                                                .                |       byte_length: 1
> 00000016  02                                                .                |       data [Length(Parts) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |         decoded: [2]
> 00000017                                                                     |     stream[1] Length(Rings) (8 B)
> 00000017  33                                                3                |       stream_type: 0x33 Length(Rings)
>                                                                              |         └ bits 7-4 = 0011 -> category = Length
>                                                                              |         └ bits 3-0 = 0011 -> subtype = Rings
> 00000018  62                                                b                |       encoding: 0x62 logical=Int(Rle(Split { runs: 1, num_rle_values: 2 })) physical=VarInt
>                                                                              |         └ bits 7-5 = 011 -> logical1 = Rle
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 00000019  02                                                .                |       num_values: 2
> 0000001a  02                                                .                |       byte_length: 2
> 0000001b  01                                                .                |       runs: 1
> 0000001c  02                                                .                |       num_rle_values: 2
> 0000001d  02 03                                             ..               |       data [Length(Rings) Int(Rle(Split { runs: 1, num_rle_values: 2 }))/VarInt, 2 values, 2 B]
>                                                                              |         decoded: [3, 3]
> 0000001f                                                                     |     stream[2] Data(Vertex) (16 B)
> 0000001f  13                                                .                |       stream_type: 0x13 Data(Vertex)
>                                                                              |         └ bits 7-4 = 0001 -> category = Data
>                                                                              |         └ bits 3-0 = 0011 -> subtype = Vertex
> 00000020  42                                                B                |       encoding: 0x42 logical=Vertex(ComponentwiseDelta) physical=VarInt
>                                                                              |         └ bits 7-5 = 010 -> logical1 = ComponentwiseDelta
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 00000021  0c                                                .                |       num_values: 12
> 00000022  0c                                                .                |       byte_length: 12
> 00000023  16 68 78 28 13 63 08 58 3b 13 28 27               .hx(.c.X;.('     |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 6 values, 12 B]
>                                                                              |         decoded: [11, 52, 71, 72, 61, 22, 65, 66, 35, 56, 55, 36]
> ```
>
> A plain string column: its `stream_count`, then its length and data streams.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (234 B)
> 00000000  e8 01                                             ..               |   size: 232 (varint) - tag + body
> 00000002  01                                                .                |   tag: 0x01 -> Tag01
> 00000003  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 0000000a  40                                                @                |   extent: 64
> 0000000b  02                                                .                |   column_count: 2
> 0000000c                                                                     |   column_schema[0] Geometry (1 B)
> 0000000c  04                                                .                |     type: 0x04 Geometry
>                                                                              |       └ bits 7-1 = 0000010 -> base type = Geometry
>                                                                              |       └ bit 0 = 0 -> not optional: each feature has a non-NULL value
> 0000000d                                                                     |   column_schema[1] OptStr "val" (5 B)
> 0000000d  1d                                                .                |     type: 0x1D OptStr
>                                                                              |       └ bits 7-1 = 0001110 -> base type = OptStr
>                                                                              |       └ bit 0 = 1 -> optional: a Present stream precedes the data
> 0000000e  03 76 61 6c                                       .val             |     name: "val"
> 00000012                                                                     |   column[0] Geometry (25 B)
> 00000012  02                                                .                |     stream_count: 2
> 00000013                                                                     |     meta (8 B)
> 00000013  30                                                0                |       stream_type: 0x30 Length(VarBinary)
>                                                                              |         └ bits 7-4 = 0011 -> category = Length
>                                                                              |         └ bits 3-0 = 0000 -> subtype = VarBinary
> 00000014  62                                                b                |       encoding: 0x62 logical=Int(Rle(Split { runs: 1, num_rle_values: 6 })) physical=VarInt
>                                                                              |         └ bits 7-5 = 011 -> logical1 = Rle
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 00000015  02                                                .                |       num_values: 2
> 00000016  02                                                .                |       byte_length: 2
> 00000017  01                                                .                |       runs: 1
> 00000018  06                                                .                |       num_rle_values: 6
> 00000019  06 00                                             ..               |       data [Length(VarBinary) Int(Rle(Split { runs: 1, num_rle_values: 6 }))/VarInt, 2 values, 2 B]
>                                                                              |         decoded: [0, 0, 0, 0, 0, 0]
> 0000001b                                                                     |     stream[0] Data(Vertex) (16 B)
> 0000001b  13                                                .                |       stream_type: 0x13 Data(Vertex)
>                                                                              |         └ bits 7-4 = 0001 -> category = Data
>                                                                              |         └ bits 3-0 = 0011 -> subtype = Vertex
> 0000001c  42                                                B                |       encoding: 0x42 logical=Vertex(ComponentwiseDelta) physical=VarInt
>                                                                              |         └ bits 7-5 = 010 -> logical1 = ComponentwiseDelta
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 0000001d  0c                                                .                |       num_values: 12
> 0000001e  0c                                                .                |       byte_length: 12
> 0000001f  16 68 78 28 13 63 08 58 3b 13 28 27               .hx(.c.X;.('     |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 6 values, 12 B]
>                                                                              |         decoded: [11, 52, 71, 72, 61, 22, 65, 66, 35, 56, 55, 36]
> 0000002b                                                                     |   column[1] OptStr "val" (191 B)
> 0000002b  03                                                .                |     stream_count: 3
> 0000002c                                                                     |     present (6 B)
> 0000002c  00                                                .                |       stream_type: 0x00 Present
>                                                                              |         └ bits 7-4 = 0000 -> category = Present
>                                                                              |         └ bits 3-0 = 0000 -> subtype = -
> 0000002d  60                                                `                |       encoding: 0x60 logical=Bool(ByteRle(Split { runs: 1, num_rle_values: 2 })) physical=None
>                                                                              |         └ bits 7-5 = 011 -> logical1 = Rle
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 00 -> physical = None
> 0000002e  06                                                .                |       num_values: 6
> 0000002f  02                                                .                |       byte_length: 2
> 00000030  ff 3f                                             .?               |       data [Present Bool(ByteRle(Split { runs: 1, num_rle_values: 2 }))/None, 6 values, 2 B]
>                                                                              |         decoded: 6 present-bits: 111111
> 00000032                                                                     |     stream[0] Length(VarBinary) (10 B)
> 00000032  30                                                0                |       stream_type: 0x30 Length(VarBinary)
>                                                                              |         └ bits 7-4 = 0011 -> category = Length
>                                                                              |         └ bits 3-0 = 0000 -> subtype = VarBinary
> 00000033  02                                                .                |       encoding: 0x02 logical=Int(None) physical=VarInt
>                                                                              |         └ bits 7-5 = 000 -> logical1 = None
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 00000034  06                                                .                |       num_values: 6
> 00000035  06                                                .                |       byte_length: 6
> 00000036  1f 1e 1d 17 19 1f                                 ......           |       data [Length(VarBinary) Int(None)/VarInt, 6 values, 6 B]
>                                                                              |         decoded: [31, 30, 29, 23, 25, 31]
> 0000003c                                                                     |     stream[1] Data(None) (174 B)
> 0000003c  10                                                .                |       stream_type: 0x10 Data(None)
>                                                                              |         └ bits 7-4 = 0001 -> category = Data
>                                                                              |         └ bits 3-0 = 0000 -> subtype = None
> 0000003d  00                                                .                |       encoding: 0x00 logical=Int(None) physical=None
>                                                                              |         └ bits 7-5 = 000 -> logical1 = None
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 00 -> physical = None
> 0000003e  06                                                .                |       num_values: 6
> 0000003f  a9 01                                             ..               |       byte_length: 169
> 00000041  72 65 73 69 64 65 6e 74 69 61 6c 5f 7a 6f 6e 65   residential_zone |       data [Data(None) Int(None)/None, 6 values, 169 B]
> 00000051  5f 6e 6f 72 74 68 5f 73 65 63 74 6f 72 5f 31 63   _north_sector_1c |
> 00000061  6f 6d 6d 65 72 63 69 61 6c 5f 7a 6f 6e 65 5f 73   ommercial_zone_s |
> 00000071  6f 75 74 68 5f 73 65 63 74 6f 72 5f 32 69 6e 64   outh_sector_2ind |
> 00000081  75 73 74 72 69 61 6c 5f 7a 6f 6e 65 5f 65 61 73   ustrial_zone_eas |
> 00000091  74 5f 73 65 63 74 6f 72 5f 33 70 61 72 6b 5f 7a   t_sector_3park_z |
> 000000a1  6f 6e 65 5f 77 65 73 74 5f 73 65 63 74 6f 72 5f   one_west_sector_ |
> 000000b1  34 77 61 74 65 72 5f 7a 6f 6e 65 5f 6e 6f 72 74   4water_zone_nort |
> 000000c1  68 5f 73 65 63 74 6f 72 5f 35 72 65 73 69 64 65   h_sector_5reside |
> 000000d1  6e 74 69 61 6c 5f 7a 6f 6e 65 5f 73 6f 75 74 68   ntial_zone_south |
> 000000e1  5f 73 65 63 74 6f 72 5f 36                        _sector_6        |
>                                                                              |         decoded: utf-8 "residential_zone_north_sector_1commercial_zone_south_sector_2industrial_zone_east_sector_3park_zone_west_sector_4water_zone_north_sector_5residential_zone_south_sector_6"
> ```
>
> A dictionary and the child columns that index into it, each with its own presence and offset stream.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (104 B)
> 00000000  67                                                g                |   size: 103 (varint) - tag + body
> 00000001  01                                                .                |   tag: 0x01 -> Tag01
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  40                                                @                |   extent: 64
> 0000000a  02                                                .                |   column_count: 2
> 0000000b                                                                     |   column_schema[0] Geometry (1 B)
> 0000000b  04                                                .                |     type: 0x04 Geometry
>                                                                              |       └ bits 7-1 = 0000010 -> base type = Geometry
>                                                                              |       └ bit 0 = 0 -> not optional: each feature has a non-NULL value
> 0000000c                                                                     |   column_schema[1] SharedDict "name:" (16 B)
> 0000000c  1e                                                .                |     type: 0x1E SharedDict
>                                                                              |       └ bits 7-1 = 0001111 -> base type = SharedDict
>                                                                              |       └ bit 0 = 0 -> not optional: each feature has a non-NULL value
> 0000000d  05 6e 61 6d 65 3a                                 .name:           |     name: "name:"
> 00000013  02                                                .                |     child_count: 2
> 00000014                                                                     |     column_schema[0] OptStr "de" (4 B)
> 00000014  1d                                                .                |       type: 0x1D OptStr
>                                                                              |         └ bits 7-1 = 0001110 -> base type = OptStr
>                                                                              |         └ bit 0 = 1 -> optional: a Present stream precedes the data
> 00000015  02 64 65                                          .de              |       name: "de"
> 00000018                                                                     |     column_schema[1] OptStr "en" (4 B)
> 00000018  1d                                                .                |       type: 0x1D OptStr
>                                                                              |         └ bits 7-1 = 0001110 -> base type = OptStr
>                                                                              |         └ bit 0 = 1 -> optional: a Present stream precedes the data
> 00000019  02 65 6e                                          .en              |       name: "en"
> 0000001c                                                                     |   column[0] Geometry (12 B)
> 0000001c  02                                                .                |     stream_count: 2
> 0000001d                                                                     |     meta (5 B)
> 0000001d  30                                                0                |       stream_type: 0x30 Length(VarBinary)
>                                                                              |         └ bits 7-4 = 0011 -> category = Length
>                                                                              |         └ bits 3-0 = 0000 -> subtype = VarBinary
> 0000001e  02                                                .                |       encoding: 0x02 logical=Int(None) physical=VarInt
>                                                                              |         └ bits 7-5 = 000 -> logical1 = None
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 0000001f  01                                                .                |       num_values: 1
> 00000020  01                                                .                |       byte_length: 1
> 00000021  00                                                .                |       data [Length(VarBinary) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |         decoded: [0]
> 00000022                                                                     |     stream[0] Data(Vertex) (6 B)
> 00000022  13                                                .                |       stream_type: 0x13 Data(Vertex)
>                                                                              |         └ bits 7-4 = 0001 -> category = Data
>                                                                              |         └ bits 3-0 = 0011 -> subtype = Vertex
> 00000023  42                                                B                |       encoding: 0x42 logical=Vertex(ComponentwiseDelta) physical=VarInt
>                                                                              |         └ bits 7-5 = 010 -> logical1 = ComponentwiseDelta
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 00000024  02                                                .                |       num_values: 2
> 00000025  02                                                .                |       byte_length: 2
> 00000026  1a 54                                             .T               |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 1 values, 2 B]
>                                                                              |         decoded: [13, 42]
> 00000028                                                                     |   column[1] SharedDict "name:" (64 B)
> 00000028  06                                                .                |     stream_count: 6
> 00000029                                                                     |     dict_stream[0] Length(Dictionary) (5 B)
> 00000029  36                                                6                |       stream_type: 0x36 Length(Dictionary)
>                                                                              |         └ bits 7-4 = 0011 -> category = Length
>                                                                              |         └ bits 3-0 = 0110 -> subtype = Dictionary
> 0000002a  02                                                .                |       encoding: 0x02 logical=Int(None) physical=VarInt
>                                                                              |         └ bits 7-5 = 000 -> logical1 = None
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 10 -> physical = VarInt
> 0000002b  01                                                .                |       num_values: 1
> 0000002c  01                                                .                |       byte_length: 1
> 0000002d  1e                                                .                |       data [Length(Dictionary) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |         decoded: [30]
> 0000002e                                                                     |     dict_stream[1] Data(Shared) (34 B)
> 0000002e  12                                                .                |       stream_type: 0x12 Data(Shared)
>                                                                              |         └ bits 7-4 = 0001 -> category = Data
>                                                                              |         └ bits 3-0 = 0010 -> subtype = Shared
> 0000002f  00                                                .                |       encoding: 0x00 logical=Int(None) physical=None
>                                                                              |         └ bits 7-5 = 000 -> logical1 = None
>                                                                              |         └ bits 4-2 = 000 -> logical2 = None
>                                                                              |         └ bits 1-0 = 00 -> physical = None
> 00000030  01                                                .                |       num_values: 1
> 00000031  1e                                                .                |       byte_length: 30
> 00000032  41 41 41 41 41 41 41 41 41 41 41 41 41 41 41 41   AAAAAAAAAAAAAAAA |       data [Data(Shared) Int(None)/None, 1 values, 30 B]
> 00000042  41 41 41 41 41 41 41 41 41 41 41 41 41 41         AAAAAAAAAAAAAA   |
>                                                                              |         decoded: utf-8 "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
> 00000050                                                                     |     child[0] OptStr (12 B)
> 00000050  02                                                .                |       stream_count: 2
> 00000051                                                                     |       present (6 B)
> 00000051  00                                                .                |         stream_type: 0x00 Present
>                                                                              |           └ bits 7-4 = 0000 -> category = Present
>                                                                              |           └ bits 3-0 = 0000 -> subtype = -
> 00000052  60                                                `                |         encoding: 0x60 logical=Bool(ByteRle(Split { runs: 1, num_rle_values: 2 })) physical=None
>                                                                              |           └ bits 7-5 = 011 -> logical1 = Rle
>                                                                              |           └ bits 4-2 = 000 -> logical2 = None
>                                                                              |           └ bits 1-0 = 00 -> physical = None
> 00000053  01                                                .                |         num_values: 1
> 00000054  02                                                .                |         byte_length: 2
> 00000055  ff 01                                             ..               |         data [Present Bool(ByteRle(Split { runs: 1, num_rle_values: 2 }))/None, 1 values, 2 B]
>                                                                              |           decoded: 1 present-bits: 1
> 00000057                                                                     |       data (5 B)
> 00000057  22                                                "                |         stream_type: 0x22 Offset(String)
>                                                                              |           └ bits 7-4 = 0010 -> category = Offset
>                                                                              |           └ bits 3-0 = 0010 -> subtype = String
> 00000058  02                                                .                |         encoding: 0x02 logical=Int(None) physical=VarInt
>                                                                              |           └ bits 7-5 = 000 -> logical1 = None
>                                                                              |           └ bits 4-2 = 000 -> logical2 = None
>                                                                              |           └ bits 1-0 = 10 -> physical = VarInt
> 00000059  01                                                .                |         num_values: 1
> 0000005a  01                                                .                |         byte_length: 1
> 0000005b  00                                                .                |         data [Offset(String) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |           decoded: [0]
> 0000005c                                                                     |     child[1] OptStr (12 B)
> 0000005c  02                                                .                |       stream_count: 2
> 0000005d                                                                     |       present (6 B)
> 0000005d  00                                                .                |         stream_type: 0x00 Present
>                                                                              |           └ bits 7-4 = 0000 -> category = Present
>                                                                              |           └ bits 3-0 = 0000 -> subtype = -
> 0000005e  60                                                `                |         encoding: 0x60 logical=Bool(ByteRle(Split { runs: 1, num_rle_values: 2 })) physical=None
>                                                                              |           └ bits 7-5 = 011 -> logical1 = Rle
>                                                                              |           └ bits 4-2 = 000 -> logical2 = None
>                                                                              |           └ bits 1-0 = 00 -> physical = None
> 0000005f  01                                                .                |         num_values: 1
> 00000060  02                                                .                |         byte_length: 2
> 00000061  ff 01                                             ..               |         data [Present Bool(ByteRle(Split { runs: 1, num_rle_values: 2 }))/None, 1 values, 2 B]
>                                                                              |           decoded: 1 present-bits: 1
> 00000063                                                                     |       data (5 B)
> 00000063  22                                                "                |         stream_type: 0x22 Offset(String)
>                                                                              |           └ bits 7-4 = 0010 -> category = Offset
>                                                                              |           └ bits 3-0 = 0010 -> subtype = String
> 00000064  02                                                .                |         encoding: 0x02 logical=Int(None) physical=VarInt
>                                                                              |           └ bits 7-5 = 000 -> logical1 = None
>                                                                              |           └ bits 4-2 = 000 -> logical2 = None
>                                                                              |           └ bits 1-0 = 10 -> physical = VarInt
> 00000065  01                                                .                |         num_values: 1
> 00000066  01                                                .                |         byte_length: 1
> 00000067  00                                                .                |         data [Offset(String) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |           decoded: [0]
> ```
