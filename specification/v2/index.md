# MapLibre Tile Specification v2

> [!NOTE]
>
> This is a live specification that evolves continuously. Features marked as are under active development and may change in future versions. Stable features are those without experimental tags.

> [!WARNING]
>
> **MLT v2 is experimental.** The layout on this page can change without a version bump, and no compatibility is promised between releases. Do not store v2 tiles in a cache that cannot be invalidated, and do not ship a v2 decoder that cannot be updated.
>
> v2 is implemented in the Rust `mlt-core` crate behind the `unstable-v2` cargo feature. This page describes that implementation. Where the two disagree, the implementation is authoritative.

## Differences from v1

v2 extends v1's [data model](<https://maplibre.org/maplibre-tile-spec/overview/index.md>) with Z and M values and nested properties, and adds a few minor restrictions. It also changes the byte layout to make tiles smaller.

- **Z-values.** Every vertex can carry an elevation. See [Z Coordinates](<#z-coordinates>).
- **M-values.** Every vertex can carry any number of values, such as road lane count and colors, or a timestamp per GPS fix. See [M-Values](<#m-values>).
- **Nested properties.** Maps and lists stay maps and lists instead of being flattened into columns like `name:en` and `name:de`. See [Nested Properties](<#nested-properties>).
- **Smaller metadata.** A stream needs one byte instead of four or more, since its role, value count and byte length are implied wherever they can be derived.

- **Extent in four bits.** Powers of two from `64` to `2097152` fit in the layer header byte. See [Extent](<#extent>).
- **Uniform geometry type.** A layer where every feature has the same geometry type stores that type in the header and no geometry type stream. See [Uniform Geometry Type](<#uniform-geometry-type>).
- **Cheaper nulls.** The presence of a column is stored as a bitmap, run list or sparse bitmap, whichever is smallest, and can be shared between columns. See [Presence Encodings](<#presence-encodings>).
- **More encodings.** Bit packing, `Int8` and `UInt8` columns, [ALP](<https://maplibre.org/maplibre-tile-spec/encodings/#alp>) for floats, [front coding](<https://maplibre.org/maplibre-tile-spec/encodings/#front-coding>) for strings and optional [rANS](<https://maplibre.org/maplibre-tile-spec/encodings/#rans>) for vertices.

Over whole planets, an uncompressed v2 tile is 9-10% smaller than v1 and 4-8% smaller than gzipped MVT. Gzipped v2 is 10-18% smaller than gzipped MVT. See [Size](<https://maplibre.org/maplibre-tile-spec/#size>) and [Decoding speed](<https://maplibre.org/maplibre-tile-spec/#decoding-speed>).

## Tile Layout

Unchanged from [v1](<https://maplibre.org/maplibre-tile-spec/specification/v1/#tile-layout>). A tile is a concatenation of tagged layer records. A v2 layer has `tag = 0x02`.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fpoint.mlt&amp;at=size>) - a single `Point` and the layer around it.

```text
tile   := layer*
layer  := [varint size] [u8 tag] [u8 body[size - 1]]
```

A tile MAY contain both v1 and v2 layers. A decoder MUST skip a layer whose tag it does not know.

## Layer Body

```text
body := [string name]                    non-empty, UTF-8, VarInt length prefix
        [u8 layer_header]                see Layer Header Byte
        [varint feature_count]
        [u8 layer_layout]
        [shared presence field] * n      n = the layout byte's shared presence count
        geometry_section
        [varint column_counts]           see Column Counts
        column * column_count
        m_value_section                  only when the header byte's m-value bit is set
```

The body MUST end exactly at the layer's `size`. Trailing bytes are an error.

`feature_count` is the default value count for every stream in the layer. See [Value Count](<#value-count>).

### Layer Header Byte

| Bits | Field |
| --- | --- |
| 7 | An [m-value section](<#m-values>) ends the body |
| 6-4 | [Uniform geometry type](<#uniform-geometry-type>) |
| 3-0 | [Extent code](<#extent>) |

`A0` from [`mvalues`](<#examples>):

A decoder needs these fields before it can read the rest of the layer.

#### Extent

v2 stores only power-of-two extents, so the nibble holds `log2(extent) - 6`:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fextent_512.mlt&amp;at=header>) - the extent nibble encoding `512`.

| Code | `0x0` | `0x1` | ... | `0xF` |
| --- | --- | --- | --- | --- |
| Extent | \\(2^6 = 64\\) | \\(2^7 = 128\\) | ... | \\(2^{21} = 2097152\\) |

Every code is assigned, so no extent nibble is rejected. An encoder given an extent v2 cannot code MUST reject the layer rather than round it.

#### Uniform Geometry Type

| Code | Meaning |
| ---: | --- |
| `0x0` | The [geometry section](<#geometry-section>) leads with a types stream |
| `0x1`-`0x6` | No geometry stream since every feature has one [geometry type](<#geometry-types>) `code - 1` |
| `0x7` | Reserved, MUST be rejected |

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fpoint.mlt&amp;at=header>) - every feature is a `Point`, so no types stream is written.

To optimize for the common case of uniform geometries per layer, we allow skipping to write the respective metadata. MUST agree with the topology the [geometry layout](<#geometry-layout>) declares.

### Column Counts

We have two kinds of columns, so we need to know how many columns of each type there are.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fmvalues_sp_prop.mlt&amp;at=column_counts>) - one property column and one m-value column, counted apart.

`column_count` is the number of ids and properties, not counting geometry or m-values. `m_value_count` is the number of [m-value columns](<#m-values>).

To optimize the transfer size, if the header byte's m-value bit - is `0`, `column_counts` is `column_count` as a plain varint and `m_value_count` is `0`. - is `1`, `column_counts` is the Morton code of both counts as a varint. Bit `i` of `column_count` sits at bit `2i` and bit `i` of `m_value_count` at bit `2i + 1`. A layer with up to 15 counted columns and up to 7 m-value columns fits both counts in one byte. `m_value_count` MUST be non-zero when the bit is set, so a code whose odd bits are all clear MUST be rejected.

Five counted columns and two m-value columns:

### Layer Layout Byte

| Bits | Field |
| --- | --- |
| 7 | Shared encoding flag: each [shared presence field](<#shared-presence-fields>) names its encoding |
| 6-4 | Number of shared presence fields, `0`-`7` |
| 3-0 | [Geometry layout](<#geometry-layout>) |

A set encoding flag with a count of `0` MUST be rejected.

### Presence Encodings

A presence field says which of `N` values are present. `N` is `feature_count` for a column, and the parent's value count for a [nested node](<#node-presence-nibble>).

The same three encodings store every field of one bit per value: a presence field, a boolean column's data stream and a nested node's presence stream. Their codes are the members of the `Bool` [family](<#families>).

| Code | Encoding | Payload |
| ---: | --- | --- |
| `0` | `Bitmap` | `ceil(N / 8)` bytes, LSB-first: bit `i % 8` of byte `i / 8` is value `i`. Bits past `N` in the final byte are padding and MUST be ignored. |
| `1` | `Runs` | Alternating varint run lengths. The first run counts absent values and MAY be `0`. The lengths MUST sum to `N`. |
| `2` | `Sparse` | A summary bitmap of `ceil(B / 8)` bytes, where `B = ceil(N / 8)`, then the non-zero bytes of the `Bitmap` form in order. Summary bit `i`, LSB-first, is set when byte `i` of the `Bitmap` form is non-zero. |

`Runs` and `Sparse` are self-delimiting, so no length is stored for either. `Sparse` reads its summary, takes the population count `k` of it, then takes `k` bytes.

A bitmap costs `ceil(N / 8)` bytes whatever it holds, so it is the smallest form only while `N` is small or the present values are scattered. A column present over one run of features, or on a handful of them, carries far less than that, and the other two encodings charge for what it carries rather than for `N`.

Six present values among `N = 32`, at `10` to `15`:

Encoders MUST write whichever of the three stores the field smallest, breaking a tie toward the lowest code. This holds for a boolean column's data stream and a nested node's presence stream as for a presence field. Decoders MUST reject run lengths that do not sum to `N`, a `Sparse` summary bit set at or past `B`, and a stored `Sparse` byte that is `0`. Bits past `N` in the last `Sparse` byte are padding and MUST be ignored, as for `Bitmap`.

### Shared Presence Fields

Several columns MAY reference the same presence field.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fprops_sp.mlt&amp;at=shared_presence>) - two columns referencing one shared field.

```text
shared_presence := [u8 encoding] only when the layout byte's encoding flag is set; a Bool encoding byte, see below
                   [payload]     per that encoding, self-delimiting
```

The `n` shared fields follow the layout byte back to back, in index order. A shared field has no presence nibble to carry its encoding. Without the layout byte's encoding flag every shared field is a `Bitmap`, and no byte naming it is stored. With it, every shared field leads with the byte naming its encoding. That byte is the [encoding byte](<#encoding-byte>) of a `Bool` stream, so the code sits in bits 6-4 and every other bit MUST be `0`: `0x00`, `0x10` or `0x20`.

Encoders MUST pick each shared field's encoding as for an inline one. Encoders MUST set the encoding flag only when some shared field's encoding is not `Bitmap`.

A column references a shared field through its [presence nibble](<#presence-nibble>). Encoders SHOULD only share a field that more than one column references. Encoders SHOULD store an unshared field inline. Shared fields are ordered by the first column that references each.

`a` and `b` from [`props_sp`](<#examples>), both referencing shared field `0`:

## Geometry Section

The geometry column is not counted in `column_count`. It always follows the shared presence fields. Which streams it contains is given by the geometry layout nibble.

```text
geometry_section := [types stream]              count = feature_count, only when the header
                                                byte names no uniform geometry type
                    [geo lengths]               \
                    [part lengths]               } present per the layout
                    [ring lengths]              /
                    [triangle lengths]          \ tessellated layouts only
                    [index buffer]              /
                    [vertex stream]             every vertex, or the distinct ones
                    [vertex offsets]            dictionary layouts only
```

Streams appear in exactly this order. Each stream's role is given by its position.

### Geometry Types

| Code | `0x0` | `0x1` | `0x2` | `0x3` | `0x4` | `0x5` |
| --- | --- | --- | --- | --- | --- | --- |
| Type | `Point` | `LineString` | `Polygon` | `MultiPoint` | `MultiLineString` | `MultiPolygon` |

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fmix_6_pt_line_poly_mpt_mline_mpoly.mlt&amp;at=types>) - all six kinds in one layer, so the types stream carries each.

The types stream holds one of these per feature. A layer whose features all share one writes the [uniform geometry type](<#uniform-geometry-type>) nibble instead.

### Geometry Layout

The low nibble of the layer layout byte:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fpoly.mlt&amp;at=layout>) - the `Polygons` layout: part and ring lengths, no vertex offsets.

Every layout leads with the types stream, unless the header byte names a [uniform geometry type](<#uniform-geometry-type>). The streams it writes after that, in column order:

| Code | Name | GeoLengths | PartLengths | RingLengths | TriLengths | IndexBuffer | Vertices | VertexOffsets |
| ---: | --- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| `0x0` | Points |  |  |  |  |  | all |  |
| `0x1` | PointsDict |  |  |  |  |  | distinct | ● |
| `0x2` | MultiPoints | ● |  |  |  |  | all |  |
| `0x3` | MultiPointsDict | ● |  |  |  |  | distinct | ● |
| `0x4` | Lines |  | ● |  |  |  | all |  |
| `0x5` | LinesDict |  | ● |  |  |  | distinct | ● |
| `0x6` | MultiLines | ● | ● |  |  |  | all |  |
| `0x7` | MultiLinesDict | ● | ● |  |  |  | distinct | ● |
| `0x8` | Polygons |  | ● | ● |  |  | all |  |
| `0x9` | PolygonsDict |  | ● | ● |  |  | distinct | ● |
| `0xA` | MultiPolygons | ● | ● | ● |  |  | all |  |
| `0xB` | MultiPolygonsDict | ● | ● | ● |  |  | distinct | ● |
| `0xC` | TessPolygons |  |  |  | ● | ● | all |  |
| `0xD` | TessPolygonsWithOutlines | ● | ● | ● | ● | ● | all |  |

`all` is every vertex in sequence, `distinct` the dictionary the vertex offsets index into.

`0xE` and `0xF` are unassigned and MUST be rejected.

There is no layout with ring lengths but without part lengths. A tessellated layer either has no outline topology (`0xC`) or all three outline streams (`0xD`). A tessellated layer that needs only some of the outline streams uses `0xD` and writes the others as empty streams.

The meaning of the topology streams, the length threshold rules, componentwise delta encoding, Hilbert-sorted vertex dictionaries and Morton codes are unchanged from [v1](<https://maplibre.org/maplibre-tile-spec/specification/v1/#geometry-column>).

### Polygon Rings

Rings take their roles from their order: the first ring of each polygon is its exterior, and any rings after it are its holes. The part and ring lengths of the [geometry layout](<#geometry-layout>) say which rings belong to which polygon. Decoders MUST take each polygon's exterior and holes from these lengths, not from winding.

A polygon ring is stored without its closing vertex, and its ring length does not count it. An encoder MUST NOT store the closing vertex. A decoder that returns polygon rings MUST close each non-empty ring by repeating its first vertex at the end.

**Winding carries no meaning**: each ring MAY be wound either way, independently of the others. Unlike [MVT](<https://github.com/mapbox/vector-tile-spec/tree/master/2.1#4344-polygon-geometry-type>), MLT does not require exteriors to have a positive area or holes a negative one.

### Tessellation

A tessellated layer stores the triangles its polygons were cut into, next to their vertices.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fmix_2_poly_mpoly_tes.mlt&amp;at=tri_lengths>) - the triangles and vertices drawn below.

Two streams carry the triangles:

- **Triangle lengths** hold one triangle count per polygon feature, in feature order. A `MultiPolygon` has one count for all its polygons together. A feature that is not a polygon has no count.
- **The index buffer** holds three vertex indices per triangle. The triangles of each feature follow those of the one before it.

Each index is a position in the vertex stream. Indices count from the layer's first vertex, not the feature's, so the vertex stream and the index buffer go to a GPU as they are. An index MUST name a vertex of the layer.

A feature's triangles are one contiguous run of the index buffer. Feature `i` starts at three times the sum of the triangle lengths before it, and spans three times its own length.

A tessellated layer either stores only the triangles (`0xC`) or keeps the outlines next to them (`0xD`).

A `Polygon` triangle, then a `MultiPolygon` of a polygon with a hole and a second triangle, from [`mix_2_poly_mpoly_tes`](<#examples>).

| Vertex | `0` | `1` | `2` | `3` | `4` | `5` | `6` | `7` | `8` | `9` | `10` | `11` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Point | `55,5` | `58,28` | `75,22` | `7,20` | `21,31` | `26,9` | `15,20` | `20,15` | `18,25` | `69,57` | `71,66` | `73,64` |
| Cut from | feature 0 |  |  | feature 1, shell |  |  | feature 1, hole |  |  | feature 1, 2nd polygon |  |  |

```text
triangle lengths: [1, 7]
index buffer:     [1, 0, 2,                       feature 0: 1 triangle, entries 0..3
                   3, 6, 8,  7, 6, 3,  4, 3, 8,   feature 1: 7 triangles, entries 3..24
                   7, 3, 5,  5, 4, 8,  8, 7, 5,
                   10, 9, 11]
```

> [!NOTE]
>
> **Converting from v1**
>
> [v1](<https://maplibre.org/maplibre-tile-spec/specification/v1/#tessellation-data-optional>) counts each index from the first vertex of its own feature instead. Its indices convert to v2's by adding that vertex's position, which v1 reads from the outlines.

#### Outlines

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fmix_2_poly_mpoly_tri.mlt&amp;at=layout>) - the same two features as triangles alone, layout `0xC`.

`0xC` stores the triangles and nothing else about a polygon's shape:

| Stream | Holds | Count |
| --- | --- | --- |
| Types | `Polygon` or `MultiPolygon` per feature, unless the header byte names a uniform type | `feature_count` |
| Triangle lengths | One triangle count per feature | `feature_count` |
| Index buffer | Three vertex indices per triangle | Explicit |
| Vertices | The vertices the triangles index | Explicit |

Every feature MUST be a `Polygon` or a `MultiPolygon`, so every feature has a triangle count. A decoder MUST reject any other type.

No topology stream says which vertices belong to which feature. The index buffer is the only reader of the vertex stream, so a vertex belongs to whichever triangles name it.

A feature decodes as a `MultiPolygon` holding one polygon per triangle, in index buffer order. Each polygon is a single ring: the triangle's three vertices in index order, then the first one again to close it. A `Polygon` feature decodes as a `MultiPolygon` too, since its shell and holes are not stored. A feature with a triangle count of `0`, such as a polygon whose vertices are collinear, decodes as an empty `MultiPolygon`.

A decoder cannot stroke an outline, count a feature's vertices, or rebuild the geometry the encoder was given. That is why `0xC` cannot carry [m-values](<#m-values>).

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fmix_2_poly_mpoly_tes.mlt&amp;at=layout>) - the same two features with their outlines kept, layout `0xD`.

`0xD` keeps the full outline topology next to the triangles. Each feature decodes as the geometry it was encoded from, and the triangles come on top.

The vertex stream is the outline vertices, in feature order, as the topology streams lay them out. A ring's [closing vertex is not stored](<#polygon-rings>), so no index names it. A feature that is not a polygon stores no triangle count and owns no run of the index buffer.

A layer needs `0xD` to stroke its polygon outlines, to hold anything but polygons, or to carry [m-values](<#m-values>).

### Z Coordinates

A vertex stream MAY hold `(x, y, z)` triples instead of `(x, y)` pairs. The two variants are differentiated by bit 0 of its [encoding byte](<#encoding-byte>)'s extension field being `1`. A `z_step` byte is the stream's [parameter](<#parameters>). `byte_length` does not count it.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fz_point.mlt&amp;at=vertices>) - a point 12 m up, its `z` one word after `x` and `y`.

\\\[ \\text{elevation} = -10000\\,\\text{m} + z \\cdot 10^{\\,z\\\_step - 3}\\,\\text{m} \\\]

| `z_step` | `0` | `1` | `2` | `3` | `4` | `5` | `6` | `7` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Step | 1 mm | 1 cm | 1 dm | 1 m | 10 m | 100 m | 1 km | 10 km |

`8`-`255` are unassigned and MUST be rejected.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fz_poly_hole.mlt&amp;at=vertices>) - a polygon with a hole at a `z_step` of `2`, one unit per decimeter.

`z` is a signed 32-bit word like `x` and `y`, so at every step the grid spans all of Terrain-RGB's -10000 m to 1667721.5 m. Negative `z` is similarly possible.

The stream holds `3 * num_values` words for `num_values` vertices, with `x`, `y` and `z` interleaved.

`z_mvalues` at a `z_step` of 1 dm:

Logical `None`, `Delta` and `Componentwise Delta` read as they do over pairs, with `Componentwise Delta` running over three components. A Morton code spans only `x` and `y`, so a vertex stream with bit 0 set and logical `Morton` MUST be rejected. `Componentwise Delta2` is defined only over pairs, so a vertex stream with bit 0 set and logical `Componentwise Delta2` MUST be rejected. Bit 1 of the extension field MUST be `0` on every vertex stream.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fz_mix.mlt&amp;at=vertices>) - all six geometry types, each vertex an interleaved triple.

The vertex stream is indexed, so (\\(x\_1, y\_1, z\_1, x\_2, y\_2, z\_2, ..., x\_i, y\_i, z\_i\\)) has the index \\(i\\):

- Under a dictionary layout, the vertex stream holds the distinct triples and the vertex offsets pointing to them. Two vertices share an entry only when their `x`, `y` and `z` are identical. [View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fz_points_hilbert.mlt&amp;at=vertex_dict>) - five points in four entries: a repeat at the same height shares one, a new height does not.
- Under a tessellated layout, the index buffer names triples. A `TessPolygons` feature holds the `z` of each triangle corner, in index buffer order. [View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fz_poly_hole_tri.mlt&amp;at=tri_indexes>) - a polygon with a hole, cut into six triangles over six `(x, y, z)` vertices.
- An [m-value](<#m-values>) runs over the same vertex sequence it does without `z`. [View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fz_mvalues.mlt&amp;at=vertices>) - a line whose three vertices carry both a `z` and an m-value.

A decoder that renders in 2D MAY read `x` and `y` and step over `z`.

> [!NOTE]
>
> **Why `-10000m` and powers of ten**
>
> `z = 0` sits at the base of [Terrain-RGB](<https://docs.mapbox.com/data/tilesets/reference/mapbox-terrain-rgb-v1/>), the default encoding of a MapLibre `raster-dem` source. No offset is needed to compare a `z` with the terrain under it.
>
> Elevation data is in decimal meters. This means a power-of-ten step can be represented exactly. `12.34 m` is `z = 1001234` at 1 cm, where a \\(2^{-7}\\) m grid would read it back as `12.34375 m`. At 1 dm, `z` is Terrain-RGB's `R * 65536 + G * 256 + B`.

> [!NOTE]
>
> **Why interleaved**
>
> Because one vertex is three adjacent words, a decoded vertex buffer can direct-to-GPU without having to index into another stream to match the z value. A layer without `z` is not impacted due to the extension bit mechanism.

## Attribute Columns

`column_count` columns follow the geometry section.

```text
column := [u8 column_type]
          [string name]              unless the data type is Id or LongId
          [presence field]           only when the presence nibble names an encoding
          [data streams]             one, or a set, per the data type
```

An inline presence field is written in the [encoding](<#presence-encodings>) its nibble names.

### Column Names

A layer has one namespace of column names, so a name identifies exactly one column of it and a style expression such as `["get", "foo"]` resolves to that column:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fprops_sp_id.mlt&amp;at=column%5B0%5D>) - an id, which has no name, beside a named `u32` column.

| Column | Name |
| --- | --- |
| A counted column of values | Its name field |
| A [shared dictionary](<#shared-dictionary-columns>) child | The group's prefix followed by the child's name, concatenated with nothing between |
| A [nested column](<#nested-properties>) | Its name field |
| An [m-value column](<#m-values>) | Its name field |

A name a second column repeats MUST be rejected, whichever kinds of column repeat it, and an encoder MUST NOT write such a layer. An `Id` or `LongId` column has no name field, and the geometry column is not a counted column, so neither takes a name. A shared dictionary group's own name is a prefix rather than a column name, and MAY repeat one. A [struct](<#struct-nodes>) field name and a [map](<#map-nodes>) key name a value inside their column, not a column, and are unique only where their own sections say.

### Column Type Byte

| Bits | Field |
| --- | --- |
| 7-4 | [Presence](<#presence-nibble>), or the [dictionary kind](<#shared-dictionary-columns>) when the data type is `0xF` |
| 3-0 | Data type |

#### Data Types

| Code | Type | Notes |
| ---: | --- | --- |
| `0x0` | `Id` | Feature id, up to 32 bits. No name field. |
| `0x1` | `LongId` | Feature id, up to 64 bits. No name field. |
| `0x2` | `Bool` |  |
| `0x3` | `Int8` |  |
| `0x4` | `UInt8` |  |
| `0x5` | `Int32` |  |
| `0x6` | `UInt32` |  |
| `0x7` | `Int64` |  |
| `0x8` | `UInt64` |  |
| `0x9` | `Float` | IEEE 754 binary32 |
| `0xA` | `Double` | IEEE 754 binary64 |
| `0xB` | `String` | Layout given by the leading stream, see [String Columns](<#string-columns>) |
| `0xC` | `Struct` | A fixed set of named fields, see [Nested Properties](<#nested-properties>) |
| `0xD` | `List` | A repeated value, see [Nested Properties](<#nested-properties>) |
| `0xE` | `Map` | String keys chosen per value, see [Nested Properties](<#nested-properties>) |
| `0xF` | Shared dictionary | See [Shared Dictionary Columns](<#shared-dictionary-columns>) |

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fprops_mixed_np.mlt>) - seven of the data types in one layer.

There are no nullable variants of these codes. Nullability is given by the presence nibble. There is no geometry code, since the geometry column is not a counted column.

A layer MUST contain at most one `Id` or `LongId` column.

#### Presence Nibble

| Nibble | Meaning |
| ---: | --- |
| `0` | Every feature has a value. Nothing is stored. |
| `1`-`3` | An inline presence field, in the [encoding](<#presence-encodings>) whose code is `nibble - 1`, follows the type byte and the name, if any. |
| `4`-`10` | The layer's shared field at index `nibble - 4`. |
| `11`-`15` | Reserved. MUST be rejected. |

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fprop_i32.mlt&amp;at=present>) - an optional column, so the nibble names an inline bitmap.

The encoding rides in the nibble rather than in a byte of its own because a bitmap is still the right answer for most columns, and a tag byte would charge every one of those for the two that are not.

A shared reference at or past the count in the layout byte MUST be rejected.

The presence nibble determines the column's value count: `feature_count` for nibble `0`, otherwise the number of present values the field names.

### Scalar Columns

Boolean, integer and id columns have a single data stream.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fprop_i32_delta_np.mlt&amp;at=column%5B0%5D>) - a non-optional `i32` column.

A float or double column has a single data stream, unless its logical encoding is `Dict`. A `Dict` float column has a second stream holding the distinct values that the first stream's codes index into.

### String Columns

The extension bits of the leading stream's encoding byte give the column's layout:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fprops_str_plain_np.mlt&amp;at=column%5B0%5D>) - the plain layout: the lengths, then the strings back to back.

| Extension | Layout | Streams, in order |
| ---: | --- | --- |
| `00` | Plain | Lengths, Values |
| `01` | Dict | Codes, DictLengths, DictValues |
| `10` | FSST | Lengths, SymbolLengths, SymbolTable, Corpus |
| `11` | FsstDict | Codes, DictLengths, SymbolLengths, SymbolTable, Corpus |

`Lengths` and `Codes` hold one value per present value. The remaining streams carry their own counts, or, for byte blobs, take their count from `byte_length`. When the `DictValues` or `Corpus` stream's encoding byte names [front coding](<https://maplibre.org/maplibre-tile-spec/encodings/#front-coding>), the preceding lengths stream holds `2N` values: `N` shared-prefix lengths, then `N` suffix lengths.

`city` over four features, null on feature 3:

The same column in each layout, then in byte order:

### Shared Dictionary Columns

Data type `0xF` introduces a dictionary followed by the columns that index into it. Its high nibble gives the dictionary kind instead of a presence:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fprops_shared_dict_bp.mlt&amp;at=column%5B0%5D>) - child columns reading one dictionary.

| Nibble | Kind | Corpus streams, in order |
| ---: | --- | --- |
| `0` | Plain | DictLengths, DictValues |
| `1` | FSST | DictLengths, SymbolLengths, SymbolTable, Corpus |

Other nibbles are reserved and MUST be rejected.

```text
shared_dict := [u8 column_type]        low nibble 0xF
               [string name]           the group's shared prefix
               [varint child_count]
               [corpus streams]        per the kind above
               child * child_count
child       := [u8 column_type]        data type MUST be String
               [string name]
               [presence field]        only when the presence nibble names an encoding
               [codes stream]          one dictionary index per present value
```

`name:de` and `name:en` over four features, `name:en` null on feature 3:

The same two columns, stored with each kind of dictionary, then in byte order:

Each child has its own presence nibble and MAY reference any of the layer's shared fields.

Front coding of the dictionary is given by the encoding byte of the last corpus stream, as for a lone string column.

### Nested Properties

A property whose value is a map or a list, nestable.

v2 shreds a nested value the way [ORC](<https://orc.apache.org/specification/ORCv1/>) does. The column is a tree of nodes. Every leaf of the tree holds one flat stream set, encoded exactly as a [column](<#columns>) of its data type. The structure lives in the presence and length streams of the interior nodes, never beside the values. A key that every feature shares is written once, in the tree, rather than once per feature.

`obj` from [`nested_struct`](<#examples>), `rank` missing on feature 1:

The tree is written depth first, each node's streams where the node sits, following v2's rule that a column's metadata and its data are adjacent.

```text
nested_column := [u8 column_type]        presence nibble over 0xC, 0xD or 0xE
                 [string name]
                 [presence field]        only when the presence nibble names an encoding
                 body                    of the kind the type byte named
```

A nested column is one entry of `column_count`, however many leaves it shreds into. Its name comes from the layer's one namespace of [column names](<#column-names>). The type byte is an ordinary [column type byte](<#column-type-byte>): the high nibble is the column's [presence](<#presence-nibble>) over the layer's features, and MAY name a shared field. The low nibble MUST be `0xC`, `0xD` or `0xE`; a scalar root is an ordinary column and MUST be written as one.

`items` from [`nested_list_struct`](<#examples>), in byte order:

#### Node Type Byte

Every node below the root begins with one:

| Bits | Field |
| --- | --- |
| 7-4 | [Node presence](<#node-presence-nibble>) |
| 3-0 | [Data type](<#data-types>) |

`0x2`-`0xB` name a leaf, `0xC`-`0xE` name an interior node. `0x0`, `0x1` and `0xF` MUST be rejected: a feature id belongs to a feature, and a shared dictionary introduces counted columns.

##### Node Presence Nibble

| Nibble | Meaning |
| ---: | --- |
| `0` | Every value the parent hands this node is present. Nothing is stored. |
| `1` | A presence stream follows the node type byte, and the field name if there is one. |
| `2`-`15` | Reserved, MUST be rejected. |

A node below the root cannot use the layer's shared fields. Those are `feature_count` bits long, and only the root of a nested column runs over features.

A presence stream is a [`Bool` stream](<#bool-streams>), one bit per value the parent hands the node. It is a stream rather than the bare field a column writes, because under a [`List`](<#list-nodes>) or a [`Map`](<#map-nodes>) the number of bits is not known until the lengths have been decoded, and the encoding byte can carry that count.

#### Counts

Each node is handed a number of values by its parent, its **parent count**:

| Node | Parent count |
| --- | --- |
| The root | The column's value count: `feature_count`, or the number of present values its presence field names |
| A struct field | The struct node's present count |
| A list element | The sum of the list node's lengths |
| A map key or map value | The sum of the map node's lengths |

A node's **present count** is its parent count under nibble `0`, and its presence stream's population count otherwise. That is what its data streams hold one value each of.

A nullable list of `{id, tag}` structs, `tag` nullable, over a present, an empty, a null and a present list:

A node's streams take their [implied count](<#value-count>) from this context, exactly as a column's do, with one exception. The sum of a lengths stream is not known until its payload is decoded, which a decoder may defer, so nothing implies a count at or below the first `List` or `Map` on the path from the root. The boundary sits at that node rather than below it: the node's own streams are already past it. There, as in an [m-value column](<#m-values>), bit 7 of the [encoding byte](<#encoding-byte>) MUST be `1` on each of:

| Stream | Written by |
| --- | --- |
| The [lengths stream](<#list-nodes>) | Every `List` and `Map` node, the first one included |
| The leading [key stream](<#map-nodes>) | Every `Map` node |
| The [presence stream](<#node-presence-nibble>) | Every node that has one, the first `List` or `Map` included |
| The leading data stream | Every leaf |

A `List` or a `Map` writes a count on its own lengths and presence streams even where its parent count would imply one, so that reading a node's header never depends on what sat above the node. An encoder SHOULD set bit 7 on every other stream that can carry one, and a decoder MUST accept a count wherever one is written. The streams after a string stream set's leading one read as they do in a [string column](<#string-columns>). A byte blob is the exception it always is: bit 7 MUST be `0` and its count stays `byte_length`.

A decoder that has decoded both a lengths stream and the counts below it MUST reject a disagreement.

A nested column MUST NOT nest more than 8 levels deep, counting the root as the first.

#### Struct Nodes

`0xC`. A fixed set of named, individually typed fields.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fnested_struct.mlt&amp;at=column%5B0%5D>) - a struct column with two leaves.

```text
struct_body := [varint field_count]      non-zero
               field * field_count
field       := [u8 node_type]
               [string field_name]
               [presence stream]         only when the node presence nibble is 1
               body
```

A field reads like a counted column: its type byte, then its name, then its nulls, then its data. Field names MUST be unique within their struct. A field present on every value of its struct stores no presence at all, which is what makes a struct the cheap shredding of a stable key set: the key is paid for once, in the tree.

#### List Nodes

`0xD`. A repeated value of one type.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fnested_list_struct.mlt&amp;at=column%5B0%5D>) - a list of structs.

```text
list_body := [lengths stream]            Int family, one length per present list
             [u8 node_type]              the element node
             [presence stream]           only when the node presence nibble is 1
             body
```

The lengths stream holds element counts, not offsets, as the geometry section's do.

`items` from [`nested_list_struct`](<#examples>), a list of `{id, tag}` structs:

The element node has no name.

A list that is null and a list that is empty are different: a null list has its presence bit clear and no length, an empty list has its presence bit set and a length of `0`.

#### Map Nodes

`0xE`. String keys chosen per value, over one value type.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fnested_map_str.mlt&amp;at=column%5B0%5D>) - a map with string values.

```text
map_body := [lengths stream]             Int family, one length per present map
            [key streams]                a string column's stream set, one key per entry
            [u8 node_type]               the value node
            [presence stream]            only when the node presence nibble is 1
            body
```

The keys are the streams a [string column](<#string-columns>) holds, laid out per the extension bits of the leading one, holding one key per entry of every present map. They carry no node type byte and no presence: a key is neither null nor of any other type, so neither is representable. The value node has no name.

`tags` from [`nested_map_str`](<#examples>), a map of strings:

A `Map` and a `Struct` express the same thing when every key holds the same type. A `Struct` spends one presence stream per key and nothing per entry; a `Map` spends one key per entry and nothing per key. Encoders MUST pick between them by comparing the stored size, as they do for every other encoding. A value whose keys hold different types is only a `Struct`.

The first three `tags` of [`nested_map_shapes`](<#examples>), both ways:

#### Leaf Nodes

`0x2`-`0xB`. A leaf holds exactly the data streams a [column](<#columns>) of the same data type holds: one stream for a boolean or integer, one or two for a float, and the set its leading stream's extension bits name for a string. It reads them against its own count, and is otherwise the same column.

#### Where Nested Columns May Appear

| Position | Nested |
| --- | --- |
| A counted column | Allowed |
| A node inside a nested column | Allowed |
| An [m-value column](<#m-values>) | MUST be rejected |
| A [shared dictionary](<#shared-dictionary-columns>) child | MUST be rejected, a child MUST be `String` |

### M-Values

An m-value is a measurement taken at a vertex rather than at a feature:

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fmvalues.mlt&amp;at=m_value%5B0%5D>) - two m-value columns over the vertices.

- a distance along a road,
- a timestamp per GPS fix,
- a width that varies along a river.

A layer stores them as named columns that run over its vertices instead of its features.

The section is present only when bit 7 of the [layer header byte](<#layer-header-byte>) is set, and is then the last thing in the layer body. Its column count comes from the layer's [column counts](<#column-counts>) varint, so the section itself starts with its first column.

```text
m_value_section := m_value_column * m_value_count
m_value_column  := [u8 column_type]           presence nibble over data type, as for a column
                   [string name]
                   [presence field]           only when the presence nibble names an encoding
                   [data streams]             one, or a set, per the data type
```

An m-value column reads the same [column type byte](<#column-type-byte>), the same [presence nibble](<#presence-nibble>) and the same data streams as a [column](<#columns>). Only its value count differs. Its name comes from the layer's one namespace of [column names](<#column-names>), which the counted columns share. An m-value column is not counted in `column_count`.

#### Data Types in M-Value Columns

`Bool` through `String`, codes `0x2`-`0xB`, carry exactly the streams they carry as a property column. `0x0`, `0x1` and `0xC`-`0xF` are reserved and MUST be rejected. A vertex carries a measurement, not a structure.

[View example](<https://maplibre.org/maplibre-tile-spec/inspector/app/?fixture=0x02%2Fmvalues.mlt>) - an `i32` and a `u32` m-value column.

#### The Vertex Sequence

A layer's vertex sequence is every vertex of every feature, in feature order, as the [geometry section](<#geometry-section>)'s topology streams lay them out. Each feature holds one contiguous run of it, of `vertex_count(f)` vertices.

An m-value column holds one value per vertex of that run, in the same order.

`dist` and `height` from [`mvalues`](<#examples>), with `height` null on feature 1:

- A polygon ring's [closing vertex is not stored](<#polygon-rings>), so it has no m-value.
- Under a dictionary layout the sequence is the one the vertex offsets stream spells out, not the distinct vertices the vertex stream holds. Two vertices that share a dictionary entry still have an m-value each.
- Under `TessPolygonsWithOutlines` the sequence is the outline vertices, which the index buffer indexes into.

#### Nulls

M-value columns are nullable, but they are at the feature level, not the vertex level. This means a single vertex cannot be null. A feature's m-value can be null though.

#### Value Count

A column's value count is the sum of `vertex_count(f)` over the features whose presence bit is set, which is every feature under presence nibble `0`.

That count is only known once the geometry topology has been decoded, which a decoder may defer, so it is not an implied count. An m-value column's leading data stream MUST set bit 7 of its [encoding byte](<#encoding-byte>) and write the count explicitly. The remaining streams of a string or float-dictionary column carry their own counts, as they do on a counted column, and a decoder that reads a non-blob one without an explicit count takes the leading stream's count as the implied one. An encoder SHOULD write an explicit count on every stream that can carry one, since none of those counts are implied here. A byte blob is the exception. Bit 7 MUST be `0` on it and its count stays `byte_length`, as on any other column. A decoder that has decoded both the geometry and an m-value column MUST reject a count that disagrees with the geometry.

Values are one flat sequence across feature boundaries. Delta encoding and RLE run through them without a break at each feature.

#### Geometry Layouts

An m-value section requires a [geometry layout](<#geometry-layout>) whose topology gives every feature's vertex count.

| Layout | M-values |
| --- | --- |
| `0x0` Points, `0x1` PointsDict | MUST be rejected |
| `0x2`-`0xB` | Allowed |
| `0xC` TessPolygons | MUST be rejected |
| `0xD` TessPolygonsWithOutlines | Allowed |

A point layer holds one vertex per feature, so a vertex-scoped column would be a property column with extra rules. Encode it as a property column. `TessPolygons` carries no outline topology, so no feature's vertex count can be read from it. A tessellated layer that needs m-values uses `0xD`.

## Streams

```text
stream := [u8 encoding_byte]
          [varint num_values]   only when bit 7 of the encoding byte is set
          [varint byte_length]  unless a Bool stream or logical `None` over physical `00`, see Byte Length
          [parameters]          per the logical encoding and extension bits, see Parameters
          [u8 payload[byte_length]]
```

### Encoding Byte

| Bits | Field |
| --- | --- |
| 7 | An explicit `num_values` varint follows |
| 6-4 | Logical encoding, numbered within the stream's [family](<#families>) |
| 3-2 | [Physical encoding](<#physical-field>), interpreted per logical encoding |
| 1-0 | Extension.<br>For string collums leading stream, this is the layout.<br>For a vertex streams, bit 0 being set means [`(x, y, z)`](<#z-coordinates>) instead of `(x, y)`.<br>MUST be `0` on every other stream. |

The vertex stream of `z_mvalues`:

#### Value Count

Bit 7 is set only when the stream's value count differs from the implied count. The implied count is:

- `byte_length`, for a byte blob
- the leading stream's count, for every stream after the leading one of a [string](<#string-columns>) stream set or a float `Dict` column
- `feature_count`, for every stream of the [geometry section](<#geometry-section>)
- `feature_count`, for the data stream of a column with presence nibble `0`
- the number of present values the presence field names, for the data stream of any other column
- the [parent or present count](<#counts>), for the streams of a nested node above the first `List` or `Map`

Bit 7 MUST be `0` on a byte blob. Bit 7 MUST be `1` on a [shared dictionary](<#shared-dictionary-columns>)'s corpus streams other than its blobs, which have no implied count. Bit 7 MUST be `1` on an [m-value column](<#m-values>)'s leading data stream, which has none either. Bit 7 MUST be `1` on the streams a [nested node](<#counts>) at or below a `List` or `Map` writes, which have none either.

A vertex stream counts vertices: one per `(x, y)` pair, [`(x, y, z)`](<#z-coordinates>) triple or Morton code.

For an RLE stream the value count is the decoded element count. The number of `(run, value)` pairs is not stored. A decoder reads pairs until `byte_length` is exhausted.

#### Byte Length

`byte_length` is present unless the stream is a [`Bool` stream](<#bool-streams>) or logical `None` over [physical](<#physical-field>) `00`. A `Bool` stream's payload delimits itself, so it never writes one. On logical `None` the payload is the elements as they are, so that pattern says the length follows from the value count and the element width the stream's type fixes:

| Stream | `byte_length` |
| --- | --- |
| The data stream of an `Int64`, `UInt64` or `LongId` column | `num_values * 8` |
| Any other `Int` or `Str` stream, which holds 32-bit words | `num_values * 4` |
| A `Vertex` stream of `(x, y)` pairs | `num_values * 8` |
| A `Vertex` stream of [`(x, y, z)` triples](<#z-coordinates>) | `num_values * 12` |
| A `Float` column's values | `num_values * 4` |
| A `Double` column's values | `num_values * 8` |

A byte blob takes its count from `byte_length`, so it MUST NOT use the pattern. Every other logical encoding that reads the physical field MUST reject it. `RLE`, `DeltaRLE`, `BitPacked` and `rANS` reserve the field as `0` and still write `byte_length`. An encoder SHOULD use it wherever it is allowed, since it is one varint shorter.

### Families

The logical encodings available to a stream depend on what it holds. Each family numbers its own members from `0`.

| Family | Used by | `0` | `1` | `2` | `3` | `4` | `5` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **Int** | Lengths, offsets, ids, integer columns, geometry topology, nested list and map lengths | None | Delta | RLE | DeltaRLE | BitPacked | [Delta2](<https://maplibre.org/maplibre-tile-spec/encodings/#delta2>) |
| **Str** | A string column's leading stream | None | Delta | RLE | DeltaRLE | BitPacked | [Delta2](<https://maplibre.org/maplibre-tile-spec/encodings/#delta2>) |
| **Bool** | A boolean column's data stream, a nested node's presence stream | Bitmap | Runs | Sparse |  |  |  |
| **Float** | A float or double column's data stream | None |  | [Framed, Exception-Free ALP](<https://maplibre.org/maplibre-tile-spec/encodings/#alp>) | Dict |  |  |
| **Vertex** | The geometry vertex stream, of pairs or [triples](<#z-coordinates>) | None | Delta | Componentwise Delta | Morton | [Componentwise Delta2](<https://maplibre.org/maplibre-tile-spec/encodings/#componentwise-delta2>) | [rANS](<https://maplibre.org/maplibre-tile-spec/encodings/#rans>) |
| **Bytes** | Byte blobs: string values, dictionaries, FSST symbol tables | None | [FrontCoded](<https://maplibre.org/maplibre-tile-spec/encodings/#front-coding>) |  |  |  |  |

The `Str` family has the same members as `Int`. It differs only in the use of the extension bits. A code not listed for the stream's family MUST be rejected.

### Bool Streams

A `Bool` stream holds one bit per value, in the [encoding](<#presence-encodings>) its logical field names. Its value count is its [implied count](<#value-count>), or the explicit one when bit 7 is set, and it is the `N` of the encoding. Its physical field and extension bits MUST be `0`. No `byte_length` exists.

### Physical Field

For a stream of integer words, meaning the `Int`, `Str` and `Vertex` families and the code or scaled-integer stream of a `Float` `Dict` or `Alp` encoding:

| Bits | Physical |
| --- | --- |
| `00` | None, without `byte_length`. Only on logical `None`, see [Byte Length](<#byte-length>) |
| `01` | None: fixed-width little-endian words |
| `10` | VarInt |
| `11` | SIMD-FastPFOR, 128-value little-endian blocks |

For a stream of opaque fixed-width elements, meaning raw `Float` values or a `Bytes` blob:

| Bits | Physical |
| --- | --- |
| `00` | Elements as they are, without `byte_length`. Only on logical `None`, and never on a blob, see [Byte Length](<#byte-length>) |
| `01` | Elements as they are |
| `10`, `11` | Unassigned, MUST be rejected |

`RLE`, `DeltaRLE`, `BitPacked`, `rANS` and every `Bool` encoding define their own physical layout. The physical field MUST be `0` for them.

> [!NOTE]
>
> v1 FastPFOR uses 256-value big-endian blocks and v2 FastPFOR uses 128-value little-endian blocks. They are not interchangeable. A decoder MUST select the variant by the layer tag.

### Parameters

Parameters sit between `byte_length`, where written, and the payload:

| Encoding | Parameters |
| --- | --- |
| Framed, Exception-Free ALP | `scale` (byte), `base` (ZigZag varint). See [Framed, Exception-Free ALP](<https://maplibre.org/maplibre-tile-spec/encodings/#alp>). |
| Morton | `bits` (varint), `shift` (varint). The grid the codes are laid on. |
| Any, on a vertex stream with extension bit 0 set | `z_step` (byte). See [Z Coordinates](<#z-coordinates>). |

## Examples

Every fixture under `test/synthetic/0x02/` can be annotated as a hexdump below. If you are building a decoder, you must produce the same `.json` snapshot for validation.

The same annotation can be produced for any tile with the [`mlt` CLI](<https://github.com/maplibre/maplibre-tile-spec/blob/main/rust/mlt/README.md>):

```bash
mlt hexdump path/to/tile.mlt
```

> [!NOTE]
>
> **Annotated hexdumps**
>
> A single `Point` at `(13, 42)` with id `100`.
>
> Every feature is a `Point`, so the header byte's uniform type nibble replaces the types stream. The vertex stream holds one vertex, which `feature_count` implies, so it writes no count.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (21 B)
> 00000000  14                                                .                |   size: 20 (varint) - tag + body
> 00000001  02                                                .                |   tag: 0x02 -> Tag02
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  10                                                .                |   header: extent = 64, every feature is a Point
>                                                                              |     └ bit 7 = 0 -> no m-value section
>                                                                              |     └ bits 6-4 = 001 -> every feature is a Point, no types stream
>                                                                              |     └ bits 3-0 = 0000 -> extent 2^(n+6) = 64
> 0000000a  01                                                .                |   feature_count: 1
> 0000000b  00                                                .                |   layout: 0x00 Points
>                                                                              |     └ bit 7 = 0 -> shared bitfields are bitmaps
>                                                                              |     └ bits 6-4 = 000 -> shared presence bitfields = 0
>                                                                              |     └ bits 3-0 = 0000 -> geometry layout = Points
> 0000000c                                                                     |   geometry (4 B)
> 0000000c                                                                     |     vertices (4 B)
> 0000000c  28                                                (                |       encoding: 0x28 logical=CwDelta physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 1 values from context
>                                                                              |         └ bits 6-4 = 010 -> logical = CwDelta, numbered for vertex stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000000d  02                                                .                |       byte_length: 2
> 0000000e  1a 54                                             .T               |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 1 values, 2 B]
>                                                                              |         decoded: [13, 42]
> 00000010  01                                                .                |   column_count: 1
> 00000011                                                                     |   column[0] Id (4 B)
> 00000011  00                                                .                |     type: 0x00 AllPresent Id
>                                                                              |       └ bits 7-4 = 0000 -> presence = AllPresent
>                                                                              |       └ bits 3-0 = 0000 -> data type = Id
> 00000012                                                                     |     data (3 B)
> 00000012  08                                                .                |       encoding: 0x08 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 1 values from context
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000013  01                                                .                |       byte_length: 1
> 00000014  64                                                d                |       data [Data(None) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |         decoded: [100]
> ```
>
> The `Polygons` geometry layout: part and ring length streams, no `GeoLengths`, no vertex offsets.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (36 B)
> 00000000  23                                                #                |   size: 35 (varint) - tag + body
> 00000001  02                                                .                |   tag: 0x02 -> Tag02
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  30                                                0                |   header: extent = 64, every feature is a Polygon
>                                                                              |     └ bit 7 = 0 -> no m-value section
>                                                                              |     └ bits 6-4 = 011 -> every feature is a Polygon, no types stream
>                                                                              |     └ bits 3-0 = 0000 -> extent 2^(n+6) = 64
> 0000000a  01                                                .                |   feature_count: 1
> 0000000b  08                                                .                |   layout: 0x08 Polygons
>                                                                              |     └ bit 7 = 0 -> shared bitfields are bitmaps
>                                                                              |     └ bits 6-4 = 000 -> shared presence bitfields = 0
>                                                                              |     └ bits 3-0 = 1000 -> geometry layout = Polygons
> 0000000c                                                                     |   geometry (23 B)
> 0000000c                                                                     |     part_lengths (3 B)
> 0000000c  08                                                .                |       encoding: 0x08 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 1 values from context
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000000d  01                                                .                |       byte_length: 1
> 0000000e  02                                                .                |       data [Length(Parts) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |         decoded: [2]
> 0000000f                                                                     |     ring_lengths (5 B)
> 0000000f  88                                                .                |       encoding: 0x88 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000010  02                                                .                |       num_values: 2
> 00000011  02                                                .                |       byte_length: 2
> 00000012  03 03                                             ..               |       data [Length(Rings) Int(None)/VarInt, 2 values, 2 B]
>                                                                              |         decoded: [3, 3]
> 00000014                                                                     |     vertices (15 B)
> 00000014  a8                                                .                |       encoding: 0xA8 logical=CwDelta physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 010 -> logical = CwDelta, numbered for vertex stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000015  06                                                .                |       num_values: 6
> 00000016  0c                                                .                |       byte_length: 12
> 00000017  16 68 78 28 13 63 08 58 3b 13 28 27               .hx(.c.X;.('     |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 6 values, 12 B]
>                                                                              |         decoded: [11, 52, 71, 72, 61, 22, 65, 66, 35, 56, 55, 36]
> 00000023  00                                                .                |   column_count: 0
> ```
>
> The `TessPolygons` geometry layout: triangle lengths, the index buffer and the vertices, and no topology.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (76 B)
> 00000000  4b                                                K                |   size: 75 (varint) - tag + body
> 00000001  02                                                .                |   tag: 0x02 -> Tag02
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  00                                                .                |   header: extent = 64
>                                                                              |     └ bit 7 = 0 -> no m-value section
>                                                                              |     └ bits 6-4 = 000 -> a types stream leads the geometry section
>                                                                              |     └ bits 3-0 = 0000 -> extent 2^(n+6) = 64
> 0000000a  02                                                .                |   feature_count: 2
> 0000000b  0c                                                .                |   layout: 0x0C TessPolygons
>                                                                              |     └ bit 7 = 0 -> shared bitfields are bitmaps
>                                                                              |     └ bits 6-4 = 000 -> shared presence bitfields = 0
>                                                                              |     └ bits 3-0 = 1100 -> geometry layout = TessPolygons
> 0000000c                                                                     |   geometry (63 B)
> 0000000c                                                                     |     types (4 B)
> 0000000c  08                                                .                |       encoding: 0x08 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 2 values from context
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000000d  02                                                .                |       byte_length: 2
> 0000000e  02 05                                             ..               |       data [Length(VarBinary) Int(None)/VarInt, 2 values, 2 B]
>                                                                              |         decoded: [2, 5]
> 00000010                                                                     |     tri_lengths (4 B)
> 00000010  08                                                .                |       encoding: 0x08 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 2 values from context
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000011  02                                                .                |       byte_length: 2
> 00000012  01 07                                             ..               |       data [Length(Triangles) Int(None)/VarInt, 2 values, 2 B]
>                                                                              |         decoded: [1, 7]
> 00000014                                                                     |     tri_indexes (27 B)
> 00000014  88                                                .                |       encoding: 0x88 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000015  18                                                .                |       num_values: 24
> 00000016  18                                                .                |       byte_length: 24
> 00000017  01 00 02 03 06 08 07 06 03 04 03 08 07 03 05 05   ................ |       data [Offset(Index) Int(None)/VarInt, 24 values, 24 B]
> 00000027  04 08 08 07 05 0a 09 0b                           ........         |
>                                                                              |         decoded: [1, 0, 2, 3, 6, 8, 7, 6, 3, 4, 3, 8, 7, 3, 5, 5, 4, 8, 8, 7, 5, 10, 9, 11]
> 0000002f                                                                     |     vertices (28 B)
> 0000002f  a8                                                .                |       encoding: 0xA8 logical=CwDelta physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 010 -> logical = CwDelta, numbered for vertex stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000030  0c                                                .                |       num_values: 12
> 00000031  19                                                .                |       byte_length: 25
> 00000032  6e 0a 06 2e 22 0b 87 01 03 1c 16 0a 2b 15 16 0a   n...".......+... |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 12 values, 25 B]
> 00000042  09 03 14 66 40 04 12 04 03                        ...f@....        |
>                                                                              |         decoded: [55, 5, 58, 28, 75, 22, 7, 20, 21, 31, 26, 9, 15, 20, 20, 15, 18, 25, 69, 57, 71, 66, 73, 64]
> 0000004b  00                                                .                |   column_count: 0
> ```
>
> A polygon with a hole, each of its six stored vertices at a height on a grid of 1 dm.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (46 B)
> 00000000  2d                                                -                |   size: 45 (varint) - tag + body
> 00000001  02                                                .                |   tag: 0x02 -> Tag02
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  30                                                0                |   header: extent = 64, every feature is a Polygon
>                                                                              |     └ bit 7 = 0 -> no m-value section
>                                                                              |     └ bits 6-4 = 011 -> every feature is a Polygon, no types stream
>                                                                              |     └ bits 3-0 = 0000 -> extent 2^(n+6) = 64
> 0000000a  01                                                .                |   feature_count: 1
> 0000000b  08                                                .                |   layout: 0x08 Polygons
>                                                                              |     └ bit 7 = 0 -> shared bitfields are bitmaps
>                                                                              |     └ bits 6-4 = 000 -> shared presence bitfields = 0
>                                                                              |     └ bits 3-0 = 1000 -> geometry layout = Polygons
> 0000000c                                                                     |   geometry (33 B)
> 0000000c                                                                     |     part_lengths (3 B)
> 0000000c  08                                                .                |       encoding: 0x08 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 1 values from context
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000000d  01                                                .                |       byte_length: 1
> 0000000e  02                                                .                |       data [Length(Parts) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |         decoded: [2]
> 0000000f                                                                     |     ring_lengths (5 B)
> 0000000f  88                                                .                |       encoding: 0x88 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000010  02                                                .                |       num_values: 2
> 00000011  02                                                .                |       byte_length: 2
> 00000012  03 03                                             ..               |       data [Length(Rings) Int(None)/VarInt, 2 values, 2 B]
>                                                                              |         decoded: [3, 3]
> 00000014                                                                     |     vertices (25 B)
> 00000014  a9                                                .                |       encoding: 0xA9 logical=CwDelta physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 010 -> logical = CwDelta, numbered for vertex stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 01 -> xyz = true -> (x, y, z) triples, a z step byte follows
> 00000015  06                                                .                |       num_values: 6
> 00000016  15                                                .                |       byte_length: 21
> 00000017  02                                                .                |       z_step: 1 dm
> 00000018  16 68 e0 a0 0c 78 28 14 13 63 14 08 58 ff 04 3b   .h...x(..c..X..; |       data [Data(Vertex) Vertex(Xyz(ZStep(-1), ComponentwiseDelta))/VarInt, 6 values, 21 B]
> 00000028  13 14 28 27 14                                    ..('.            |
>                                                                              |         decoded: [11, 52, 100400, 71, 72, 100410, 61, 22, 100420, 65, 66, 100100, 35, 56, 100110, 55, 36, 100120]
> 0000002d  00                                                .                |   column_count: 0
> ```
>
> Two columns referencing one shared field, which the layout byte counts.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (38 B)
> 00000000  25                                                %                |   size: 37 (varint) - tag + body
> 00000001  02                                                .                |   tag: 0x02 -> Tag02
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  10                                                .                |   header: extent = 64, every feature is a Point
>                                                                              |     └ bit 7 = 0 -> no m-value section
>                                                                              |     └ bits 6-4 = 001 -> every feature is a Point, no types stream
>                                                                              |     └ bits 3-0 = 0000 -> extent 2^(n+6) = 64
> 0000000a  04                                                .                |   feature_count: 4
> 0000000b  10                                                .                |   layout: 0x10 Points
>                                                                              |     └ bit 7 = 0 -> shared bitfields are bitmaps
>                                                                              |     └ bits 6-4 = 001 -> shared presence bitfields = 1
>                                                                              |     └ bits 3-0 = 0000 -> geometry layout = Points
> 0000000c                                                                     |   shared_presence (1 B)
> 0000000c  05                                                .                |     present[0] [Present Bool(None)/None, 4 values, 1 B]
>                                                                              |       decoded: 4 present-bits: 1010
> 0000000d                                                                     |   geometry (10 B)
> 0000000d                                                                     |     vertices (10 B)
> 0000000d  28                                                (                |       encoding: 0x28 logical=CwDelta physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 4 values from context
>                                                                              |         └ bits 6-4 = 010 -> logical = CwDelta, numbered for vertex stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000000e  08                                                .                |       byte_length: 8
> 0000000f  1a 54 00 00 00 00 00 00                           .T......         |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 4 values, 8 B]
>                                                                              |         decoded: [13, 42, 13, 42, 13, 42, 13, 42]
> 00000017  02                                                .                |   column_count: 2
> 00000018                                                                     |   column[0] OptU32 "a" (7 B)
> 00000018  46                                                F                |     type: 0x46 Shared(0) U32
>                                                                              |       └ bits 7-4 = 0100 -> presence = Shared(0)
>                                                                              |       └ bits 3-0 = 0110 -> data type = U32
> 00000019  01 61                                             .a               |     name: "a"
> 0000001b                                                                     |     data (4 B)
> 0000001b  08                                                .                |       encoding: 0x08 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 2 values from context
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000001c  02                                                .                |       byte_length: 2
> 0000001d  00 02                                             ..               |       data [Data(None) Int(None)/VarInt, 2 values, 2 B]
>                                                                              |         decoded: [0, 2]
> 0000001f                                                                     |   column[1] OptU32 "b" (7 B)
> 0000001f  46                                                F                |     type: 0x46 Shared(0) U32
>                                                                              |       └ bits 7-4 = 0100 -> presence = Shared(0)
>                                                                              |       └ bits 3-0 = 0110 -> data type = U32
> 00000020  01 62                                             .b               |     name: "b"
> 00000022                                                                     |     data (4 B)
> 00000022  08                                                .                |       encoding: 0x08 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 2 values from context
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000023  02                                                .                |       byte_length: 2
> 00000024  00 02                                             ..               |       data [Data(None) Int(None)/VarInt, 2 values, 2 B]
>                                                                              |         decoded: [0, 2]
> ```
>
> A `Dict` string column. The extension bits of the leading stream give the layout.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (65 B)
> 00000000  40                                                @                |   size: 64 (varint) - tag + body
> 00000001  02                                                .                |   tag: 0x02 -> Tag02
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  10                                                .                |   header: extent = 64, every feature is a Point
>                                                                              |     └ bit 7 = 0 -> no m-value section
>                                                                              |     └ bits 6-4 = 001 -> every feature is a Point, no types stream
>                                                                              |     └ bits 3-0 = 0000 -> extent 2^(n+6) = 64
> 0000000a  02                                                .                |   feature_count: 2
> 0000000b  00                                                .                |   layout: 0x00 Points
>                                                                              |     └ bit 7 = 0 -> shared bitfields are bitmaps
>                                                                              |     └ bits 6-4 = 000 -> shared presence bitfields = 0
>                                                                              |     └ bits 3-0 = 0000 -> geometry layout = Points
> 0000000c                                                                     |   geometry (6 B)
> 0000000c                                                                     |     vertices (6 B)
> 0000000c  28                                                (                |       encoding: 0x28 logical=CwDelta physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 2 values from context
>                                                                              |         └ bits 6-4 = 010 -> logical = CwDelta, numbered for vertex stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000000d  04                                                .                |       byte_length: 4
> 0000000e  16 68 78 28                                       .hx(             |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 2 values, 4 B]
>                                                                              |         decoded: [11, 52, 71, 72]
> 00000012  01                                                .                |   column_count: 1
> 00000013                                                                     |   column[0] OptStr "val" (46 B)
> 00000013  1b                                                .                |     type: 0x1B Inline(Bitmap) Str
>                                                                              |       └ bits 7-4 = 0001 -> presence = Inline(Bitmap)
>                                                                              |       └ bits 3-0 = 1011 -> data type = Str
> 00000014  03 76 61 6c                                       .val             |     name: "val"
> 00000018  03                                                .                |     present [Present Bool(None)/None, 2 values, 1 B]
>                                                                              |       decoded: 2 present-bits: 11
> 00000019                                                                     |     codes (4 B)
> 00000019  21                                                !                |       encoding: 0x21 logical=Rle physical=implied
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 2 values from context
>                                                                              |         └ bits 6-4 = 010 -> logical = Rle, numbered for string column
>                                                                              |         └ bits 3-2 = 00 -> physical = implied
>                                                                              |         └ bits 1-0 = 01 -> string layout = Dict
> 0000001a  02                                                .                |       byte_length: 2
> 0000001b  02 00                                             ..               |       data [Offset(String) Int(Rle(Interleaved { num_rle_values: 2 }))/VarInt, 2 values, 2 B]
>                                                                              |         decoded: [0, 0]
> 0000001d                                                                     |     dict_lengths (4 B)
> 0000001d  88                                                .                |       encoding: 0x88 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000001e  01                                                .                |       num_values: 1
> 0000001f  01                                                .                |       byte_length: 1
> 00000020  1e                                                .                |       data [Length(Dictionary) Int(None)/VarInt, 1 values, 1 B]
>                                                                              |         decoded: [30]
> 00000021                                                                     |     dict_values (32 B)
> 00000021  04                                                .                |       encoding: 0x04 logical=None physical=WithLen
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> a blob's byte length is its value count
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for byte blob
>                                                                              |         └ bits 3-2 = 01 -> physical = WithLen
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000022  1e                                                .                |       byte_length: 30
> 00000023  41 41 41 41 41 41 41 41 41 41 41 41 41 41 41 41   AAAAAAAAAAAAAAAA |       data [Data(Single) Int(None)/None, 30 values, 30 B]
> 00000033  41 41 41 41 41 41 41 41 41 41 41 41 41 41         AAAAAAAAAAAAAA   |
>                                                                              |         decoded: utf-8 "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
> ```
>
> A double column stored as scaled integers, with the `scale` byte and `base` following the byte length.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (45 B)
> 00000000  2c                                                ,                |   size: 44 (varint) - tag + body
> 00000001  02                                                .                |   tag: 0x02 -> Tag02
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  10                                                .                |   header: extent = 64, every feature is a Point
>                                                                              |     └ bit 7 = 0 -> no m-value section
>                                                                              |     └ bits 6-4 = 001 -> every feature is a Point, no types stream
>                                                                              |     └ bits 3-0 = 0000 -> extent 2^(n+6) = 64
> 0000000a  06                                                .                |   feature_count: 6
> 0000000b  00                                                .                |   layout: 0x00 Points
>                                                                              |     └ bit 7 = 0 -> shared bitfields are bitmaps
>                                                                              |     └ bits 6-4 = 000 -> shared presence bitfields = 0
>                                                                              |     └ bits 3-0 = 0000 -> geometry layout = Points
> 0000000c                                                                     |   geometry (14 B)
> 0000000c                                                                     |     vertices (14 B)
> 0000000c  28                                                (                |       encoding: 0x28 logical=CwDelta physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 6 values from context
>                                                                              |         └ bits 6-4 = 010 -> logical = CwDelta, numbered for vertex stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000000d  0c                                                .                |       byte_length: 12
> 0000000e  1a 54 00 00 00 00 00 00 00 00 00 00               .T..........     |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 6 values, 12 B]
>                                                                              |         decoded: [13, 42, 13, 42, 13, 42, 13, 42, 13, 42, 13, 42]
> 0000001a  01                                                .                |   column_count: 1
> 0000001b                                                                     |   column[0] OptF64 "val" (18 B)
> 0000001b  1a                                                .                |     type: 0x1A Inline(Bitmap) F64
>                                                                              |       └ bits 7-4 = 0001 -> presence = Inline(Bitmap)
>                                                                              |       └ bits 3-0 = 1010 -> data type = F64
> 0000001c  03 76 61 6c                                       .val             |     name: "val"
> 00000020  2d                                                -                |     present [Present Bool(None)/None, 6 values, 1 B]
>                                                                              |       decoded: 6 present-bits: 101101
> 00000021                                                                     |     data (12 B)
> 00000021  28                                                (                |       encoding: 0x28 logical=Alp physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 4 values from context
>                                                                              |         └ bits 6-4 = 010 -> logical = Alp, numbered for float column
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000022  07                                                .                |       byte_length: 7
> 00000023  03                                                .                |       alp_scale: e=2, f=0
> 00000024  c1 03                                             ..               |       alp_base: -225
> 00000026  96 01 fa 01 f7 02 00                              .......          |       data [Data(None) Float(Alp(Alp { e: 2, f: 0, base: -225 }))/VarInt, 4 values, 7 B]
>                                                                              |         decoded: [-75, 25, 150, -225]
> ```
>
> Two vertex-scoped columns over a line layer, one of them null on some features.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (71 B)
> 00000000  46                                                F                |   size: 70 (varint) - tag + body
> 00000001  02                                                .                |   tag: 0x02 -> Tag02
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  a0                                                .                |   header: extent = 64, every feature is a LineString, m-values
>                                                                              |     └ bit 7 = 1 -> an m-value section ends the body
>                                                                              |     └ bits 6-4 = 010 -> every feature is a LineString, no types stream
>                                                                              |     └ bits 3-0 = 0000 -> extent 2^(n+6) = 64
> 0000000a  03                                                .                |   feature_count: 3
> 0000000b  04                                                .                |   layout: 0x04 Lines
>                                                                              |     └ bit 7 = 0 -> shared bitfields are bitmaps
>                                                                              |     └ bits 6-4 = 000 -> shared presence bitfields = 0
>                                                                              |     └ bits 3-0 = 0100 -> geometry layout = Lines
> 0000000c                                                                     |   geometry (24 B)
> 0000000c                                                                     |     part_lengths (5 B)
> 0000000c  08                                                .                |       encoding: 0x08 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 3 values from context
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000000d  03                                                .                |       byte_length: 3
> 0000000e  03 03 02                                          ...              |       data [Length(Parts) Int(None)/VarInt, 3 values, 3 B]
>                                                                              |         decoded: [3, 3, 2]
> 00000011                                                                     |     vertices (19 B)
> 00000011  a8                                                .                |       encoding: 0xA8 logical=CwDelta physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 010 -> logical = CwDelta, numbered for vertex stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000012  08                                                .                |       num_values: 8
> 00000013  10                                                .                |       byte_length: 16
> 00000014  16 68 78 28 13 63 4b 18 64 3b 77 28 0f 1c 0e 0e   .hx(.cK.d;w(.... |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 8 values, 16 B]
>                                                                              |         decoded: [11, 52, 71, 72, 61, 22, 23, 34, 73, 4, 13, 24, 5, 38, 12, 45]
> 00000024  08                                                .                |   column_counts: columns = 0, m-values = 2
> 00000025                                                                     |   m_value[0] U32 "dist" (17 B)
> 00000025  06                                                .                |     type: 0x06 AllPresent U32
>                                                                              |       └ bits 7-4 = 0000 -> presence = AllPresent
>                                                                              |       └ bits 3-0 = 0110 -> data type = U32
> 00000026  04 64 69 73 74                                    .dist            |     name: "dist"
> 0000002b                                                                     |     data (11 B)
> 0000002b  98                                                .                |       encoding: 0x98 logical=Delta physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 001 -> logical = Delta, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000002c  08                                                .                |       num_values: 8
> 0000002d  08                                                .                |       byte_length: 8
> 0000002e  00 14 1e 31 1e 32 4f 18                           ...1.2O.         |       data [Data(None) Int(Delta)/VarInt, 8 values, 8 B]
>                                                                              |         decoded: [0, 10, 25, 0, 15, 40, 0, 12]
> 00000036                                                                     |   m_value[1] OptI32 "height" (17 B)
> 00000036  15                                                .                |     type: 0x15 Inline(Bitmap) I32
>                                                                              |       └ bits 7-4 = 0001 -> presence = Inline(Bitmap)
>                                                                              |       └ bits 3-0 = 0101 -> data type = I32
> 00000037  06 68 65 69 67 68 74                              .height          |     name: "height"
> 0000003e  05                                                .                |     present [Present Bool(None)/None, 3 values, 1 B]
>                                                                              |       decoded: 3 present-bits: 101
> 0000003f                                                                     |     data (8 B)
> 0000003f  88                                                .                |       encoding: 0x88 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 00000040  05                                                .                |       num_values: 5
> 00000041  05                                                .                |       byte_length: 5
> 00000042  05 08 12 04 0e                                    .....            |       data [Data(None) Int(None)/VarInt, 5 values, 5 B]
>                                                                              |         decoded: [-3, 4, 9, 2, 7]
> ```
>
> A `Struct` column of two fields, one of them null on some features. The field names are written once, in the tree. The nullable field's presence stream is a bitmap, so its header carries no byte length.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (58 B)
> 00000000  39                                                9                |   size: 57 (varint) - tag + body
> 00000001  02                                                .                |   tag: 0x02 -> Tag02
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  10                                                .                |   header: extent = 64, every feature is a Point
>                                                                              |     └ bit 7 = 0 -> no m-value section
>                                                                              |     └ bits 6-4 = 001 -> every feature is a Point, no types stream
>                                                                              |     └ bits 3-0 = 0000 -> extent 2^(n+6) = 64
> 0000000a  03                                                .                |   feature_count: 3
> 0000000b  00                                                .                |   layout: 0x00 Points
>                                                                              |     └ bit 7 = 0 -> shared bitfields are bitmaps
>                                                                              |     └ bits 6-4 = 000 -> shared presence bitfields = 0
>                                                                              |     └ bits 3-0 = 0000 -> geometry layout = Points
> 0000000c                                                                     |   geometry (8 B)
> 0000000c                                                                     |     vertices (8 B)
> 0000000c  28                                                (                |       encoding: 0x28 logical=CwDelta physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 3 values from context
>                                                                              |         └ bits 6-4 = 010 -> logical = CwDelta, numbered for vertex stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000000d  06                                                .                |       byte_length: 6
> 0000000e  16 68 78 28 13 63                                 .hx(.c           |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 3 values, 6 B]
>                                                                              |         decoded: [11, 52, 71, 72, 61, 22]
> 00000014  01                                                .                |   column_count: 1
> 00000015                                                                     |   column[0] Struct "obj" (37 B)
> 00000015  0c                                                .                |     type: 0x0C AllPresent Struct
>                                                                              |       └ bits 7-4 = 0000 -> presence = AllPresent
>                                                                              |       └ bits 3-0 = 1100 -> data type = Struct
> 00000016  03 6f 62 6a                                       .obj             |     name: "obj"
> 0000001a  02                                                .                |     field_count: 2
> 0000001b                                                                     |     field[0] Str "name" (19 B)
> 0000001b  0b                                                .                |       type: 0x0B AllPresent Str
>                                                                              |         └ bits 7-4 = 0000 -> node presence = AllPresent
>                                                                              |         └ bits 3-0 = 1011 -> data type = Str
> 0000001c  04 6e 61 6d 65                                    .name            |       name: "name"
> 00000021                                                                     |       lengths (5 B)
> 00000021  08                                                .                |         encoding: 0x08 logical=None physical=VarInt
>                                                                              |           └ bit 7 = 0 -> has_explicit_count = false -> 3 values from context
>                                                                              |           └ bits 6-4 = 000 -> logical = None, numbered for string column
>                                                                              |           └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |           └ bits 1-0 = 00 -> string layout = Plain
> 00000022  03                                                .                |         byte_length: 3
> 00000023  02 02 02                                          ...              |         data [Length(VarBinary) Int(None)/VarInt, 3 values, 3 B]
>                                                                              |           decoded: [2, 2, 2]
> 00000026                                                                     |       values (8 B)
> 00000026  04                                                .                |         encoding: 0x04 logical=None physical=WithLen
>                                                                              |           └ bit 7 = 0 -> has_explicit_count = false -> a blob's byte length is its value count
>                                                                              |           └ bits 6-4 = 000 -> logical = None, numbered for byte blob
>                                                                              |           └ bits 3-2 = 01 -> physical = WithLen
>                                                                              |           └ bits 1-0 = 00 -> extension = 0
> 00000027  06                                                .                |         byte_length: 6
> 00000028  61 62 63 64 65 66                                 abcdef           |         data [Data(None) Int(None)/None, 6 values, 6 B]
>                                                                              |           decoded: utf-8 "abcdef"
> 0000002e                                                                     |     field[1] I32 "rank" (12 B)
> 0000002e  15                                                .                |       type: 0x15 Stream I32
>                                                                              |         └ bits 7-4 = 0001 -> node presence = Stream
>                                                                              |         └ bits 3-0 = 0101 -> data type = I32
> 0000002f  04 72 61 6e 6b                                    .rank            |       name: "rank"
> 00000034                                                                     |       present (2 B)
> 00000034  00                                                .                |         encoding: 0x00 logical=None physical=implied
>                                                                              |           └ bit 7 = 0 -> has_explicit_count = false -> 3 values from context
>                                                                              |           └ bits 6-4 = 000 -> logical = None, numbered for bool column
>                                                                              |           └ bits 3-2 = 00 -> physical = implied
>                                                                              |           └ bits 1-0 = 00 -> extension = 0
> 00000035  05                                                .                |         data [Present Bool(None)/None, 3 values, 1 B]
>                                                                              |           decoded: 3 present-bits: 101
> 00000036                                                                     |       data (4 B)
> 00000036  08                                                .                |         encoding: 0x08 logical=None physical=VarInt
>                                                                              |           └ bit 7 = 0 -> has_explicit_count = false -> 2 values from context
>                                                                              |           └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |           └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |           └ bits 1-0 = 00 -> extension = 0
> 00000037  02                                                .                |         byte_length: 2
> 00000038  0e 12                                             ..               |         data [Data(None) Int(None)/VarInt, 2 values, 2 B]
>                                                                              |           decoded: [7, 9]
> ```
>
> A `List` of structs. The lengths stream counts elements per feature, and it and every stream below it write their own count.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (62 B)
> 00000000  3d                                                =                |   size: 61 (varint) - tag + body
> 00000001  02                                                .                |   tag: 0x02 -> Tag02
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  10                                                .                |   header: extent = 64, every feature is a Point
>                                                                              |     └ bit 7 = 0 -> no m-value section
>                                                                              |     └ bits 6-4 = 001 -> every feature is a Point, no types stream
>                                                                              |     └ bits 3-0 = 0000 -> extent 2^(n+6) = 64
> 0000000a  02                                                .                |   feature_count: 2
> 0000000b  00                                                .                |   layout: 0x00 Points
>                                                                              |     └ bit 7 = 0 -> shared bitfields are bitmaps
>                                                                              |     └ bits 6-4 = 000 -> shared presence bitfields = 0
>                                                                              |     └ bits 3-0 = 0000 -> geometry layout = Points
> 0000000c                                                                     |   geometry (6 B)
> 0000000c                                                                     |     vertices (6 B)
> 0000000c  28                                                (                |       encoding: 0x28 logical=CwDelta physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 2 values from context
>                                                                              |         └ bits 6-4 = 010 -> logical = CwDelta, numbered for vertex stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000000d  04                                                .                |       byte_length: 4
> 0000000e  16 68 78 28                                       .hx(             |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 2 values, 4 B]
>                                                                              |         decoded: [11, 52, 71, 72]
> 00000012  01                                                .                |   column_count: 1
> 00000013                                                                     |   column[0] List "items" (43 B)
> 00000013  0d                                                .                |     type: 0x0D AllPresent List
>                                                                              |       └ bits 7-4 = 0000 -> presence = AllPresent
>                                                                              |       └ bits 3-0 = 1101 -> data type = List
> 00000014  05 69 74 65 6d 73                                 .items           |     name: "items"
> 0000001a                                                                     |     lengths (5 B)
> 0000001a  88                                                .                |       encoding: 0x88 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000001b  02                                                .                |       num_values: 2
> 0000001c  02                                                .                |       byte_length: 2
> 0000001d  02 01                                             ..               |       data [Length(Nested) Int(None)/VarInt, 2 values, 2 B]
>                                                                              |         decoded: [2, 1]
> 0000001f                                                                     |     element Struct (31 B)
> 0000001f  0c                                                .                |       type: 0x0C AllPresent Struct
>                                                                              |         └ bits 7-4 = 0000 -> node presence = AllPresent
>                                                                              |         └ bits 3-0 = 1100 -> data type = Struct
> 00000020  02                                                .                |       field_count: 2
> 00000021                                                                     |       field[0] I32 "id" (10 B)
> 00000021  05                                                .                |         type: 0x05 AllPresent I32
>                                                                              |           └ bits 7-4 = 0000 -> node presence = AllPresent
>                                                                              |           └ bits 3-0 = 0101 -> data type = I32
> 00000022  02 69 64                                          .id              |         name: "id"
> 00000025                                                                     |         data (6 B)
> 00000025  88                                                .                |           encoding: 0x88 logical=None physical=VarInt
>                                                                              |             └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |             └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |             └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |             └ bits 1-0 = 00 -> extension = 0
> 00000026  03                                                .                |           num_values: 3
> 00000027  03                                                .                |           byte_length: 3
> 00000028  02 04 06                                          ...              |           data [Data(None) Int(None)/VarInt, 3 values, 3 B]
>                                                                              |             decoded: [1, 2, 3]
> 0000002b                                                                     |       field[1] Str "tag" (19 B)
> 0000002b  0b                                                .                |         type: 0x0B AllPresent Str
>                                                                              |           └ bits 7-4 = 0000 -> node presence = AllPresent
>                                                                              |           └ bits 3-0 = 1011 -> data type = Str
> 0000002c  03 74 61 67                                       .tag             |         name: "tag"
> 00000030                                                                     |         lengths (6 B)
> 00000030  88                                                .                |           encoding: 0x88 logical=None physical=VarInt
>                                                                              |             └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |             └ bits 6-4 = 000 -> logical = None, numbered for string column
>                                                                              |             └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |             └ bits 1-0 = 00 -> string layout = Plain
> 00000031  03                                                .                |           num_values: 3
> 00000032  03                                                .                |           byte_length: 3
> 00000033  02 02 02                                          ...              |           data [Length(VarBinary) Int(None)/VarInt, 3 values, 3 B]
>                                                                              |             decoded: [2, 2, 2]
> 00000036                                                                     |         values (8 B)
> 00000036  04                                                .                |           encoding: 0x04 logical=None physical=WithLen
>                                                                              |             └ bit 7 = 0 -> has_explicit_count = false -> a blob's byte length is its value count
>                                                                              |             └ bits 6-4 = 000 -> logical = None, numbered for byte blob
>                                                                              |             └ bits 3-2 = 01 -> physical = WithLen
>                                                                              |             └ bits 1-0 = 00 -> extension = 0
> 00000037  06                                                .                |           byte_length: 6
> 00000038  68 69 6c 6f 68 69                                 hilohi           |           data [Data(None) Int(None)/None, 6 values, 6 B]
>                                                                              |             decoded: utf-8 "hilohi"
> ```
>
> A `Map` of strings, its keys a dictionary-encoded string stream set.
>
> ```text
> 00000000                                                                     | layer[0] "layer1" (73 B)
> 00000000  48                                                H                |   size: 72 (varint) - tag + body
> 00000001  02                                                .                |   tag: 0x02 -> Tag02
> 00000002  06 6c 61 79 65 72 31                              .layer1          |   name: "layer1"
> 00000009  10                                                .                |   header: extent = 64, every feature is a Point
>                                                                              |     └ bit 7 = 0 -> no m-value section
>                                                                              |     └ bits 6-4 = 001 -> every feature is a Point, no types stream
>                                                                              |     └ bits 3-0 = 0000 -> extent 2^(n+6) = 64
> 0000000a  02                                                .                |   feature_count: 2
> 0000000b  00                                                .                |   layout: 0x00 Points
>                                                                              |     └ bit 7 = 0 -> shared bitfields are bitmaps
>                                                                              |     └ bits 6-4 = 000 -> shared presence bitfields = 0
>                                                                              |     └ bits 3-0 = 0000 -> geometry layout = Points
> 0000000c                                                                     |   geometry (6 B)
> 0000000c                                                                     |     vertices (6 B)
> 0000000c  28                                                (                |       encoding: 0x28 logical=CwDelta physical=VarInt
>                                                                              |         └ bit 7 = 0 -> has_explicit_count = false -> 2 values from context
>                                                                              |         └ bits 6-4 = 010 -> logical = CwDelta, numbered for vertex stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000000d  04                                                .                |       byte_length: 4
> 0000000e  16 68 78 28                                       .hx(             |       data [Data(Vertex) Vertex(ComponentwiseDelta)/VarInt, 2 values, 4 B]
>                                                                              |         decoded: [11, 52, 71, 72]
> 00000012  01                                                .                |   column_count: 1
> 00000013                                                                     |   column[0] Map "tags" (54 B)
> 00000013  0e                                                .                |     type: 0x0E AllPresent Map
>                                                                              |       └ bits 7-4 = 0000 -> presence = AllPresent
>                                                                              |       └ bits 3-0 = 1110 -> data type = Map
> 00000014  04 74 61 67 73                                    .tags            |     name: "tags"
> 00000019                                                                     |     lengths (5 B)
> 00000019  88                                                .                |       encoding: 0x88 logical=None physical=VarInt
>                                                                              |         └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |         └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |         └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |         └ bits 1-0 = 00 -> extension = 0
> 0000001a  02                                                .                |       num_values: 2
> 0000001b  02                                                .                |       byte_length: 2
> 0000001c  02 02                                             ..               |       data [Length(Nested) Int(None)/VarInt, 2 values, 2 B]
>                                                                              |         decoded: [2, 2]
> 0000001e                                                                     |     keys (18 B)
> 0000001e                                                                     |       codes (7 B)
> 0000001e  89                                                .                |         encoding: 0x89 logical=None physical=VarInt
>                                                                              |           └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |           └ bits 6-4 = 000 -> logical = None, numbered for string column
>                                                                              |           └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |           └ bits 1-0 = 01 -> string layout = Dict
> 0000001f  04                                                .                |         num_values: 4
> 00000020  04                                                .                |         byte_length: 4
> 00000021  00 01 00 01                                       ....             |         data [Offset(String) Int(None)/VarInt, 4 values, 4 B]
>                                                                              |           decoded: [0, 1, 0, 1]
> 00000025                                                                     |       dict_lengths (5 B)
> 00000025  88                                                .                |         encoding: 0x88 logical=None physical=VarInt
>                                                                              |           └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |           └ bits 6-4 = 000 -> logical = None, numbered for integer stream
>                                                                              |           └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |           └ bits 1-0 = 00 -> extension = 0
> 00000026  02                                                .                |         num_values: 2
> 00000027  02                                                .                |         byte_length: 2
> 00000028  02 02                                             ..               |         data [Length(Dictionary) Int(None)/VarInt, 2 values, 2 B]
>                                                                              |           decoded: [2, 2]
> 0000002a                                                                     |       dict_values (6 B)
> 0000002a  04                                                .                |         encoding: 0x04 logical=None physical=WithLen
>                                                                              |           └ bit 7 = 0 -> has_explicit_count = false -> a blob's byte length is its value count
>                                                                              |           └ bits 6-4 = 000 -> logical = None, numbered for byte blob
>                                                                              |           └ bits 3-2 = 01 -> physical = WithLen
>                                                                              |           └ bits 1-0 = 00 -> extension = 0
> 0000002b  04                                                .                |         byte_length: 4
> 0000002c  65 6e 64 65                                       ende             |         data [Data(Single) Int(None)/None, 4 values, 4 B]
>                                                                              |           decoded: utf-8 "ende"
> 00000030                                                                     |     value Str (25 B)
> 00000030  0b                                                .                |       type: 0x0B AllPresent Str
>                                                                              |         └ bits 7-4 = 0000 -> node presence = AllPresent
>                                                                              |         └ bits 3-0 = 1011 -> data type = Str
> 00000031                                                                     |       lengths (7 B)
> 00000031  88                                                .                |         encoding: 0x88 logical=None physical=VarInt
>                                                                              |           └ bit 7 = 1 -> has_explicit_count = true -> a num_values varint follows
>                                                                              |           └ bits 6-4 = 000 -> logical = None, numbered for string column
>                                                                              |           └ bits 3-2 = 10 -> physical = VarInt
>                                                                              |           └ bits 1-0 = 00 -> string layout = Plain
> 00000032  04                                                .                |         num_values: 4
> 00000033  04                                                .                |         byte_length: 4
> 00000034  03 04 04 04                                       ....             |         data [Length(VarBinary) Int(None)/VarInt, 4 values, 4 B]
>                                                                              |           decoded: [3, 4, 4, 4]
> 00000038                                                                     |       values (17 B)
> 00000038  04                                                .                |         encoding: 0x04 logical=None physical=WithLen
>                                                                              |           └ bit 7 = 0 -> has_explicit_count = false -> a blob's byte length is its value count
>                                                                              |           └ bits 6-4 = 000 -> logical = None, numbered for byte blob
>                                                                              |           └ bits 3-2 = 01 -> physical = WithLen
>                                                                              |           └ bits 1-0 = 00 -> extension = 0
> 00000039  0f                                                .                |         byte_length: 15
> 0000003a  73 65 61 6d 65 65 72 68 69 6c 6c 62 65 72 67      seameerhillberg  |         data [Data(None) Int(None)/None, 15 values, 15 B]
>                                                                              |           decoded: utf-8 "seameerhillberg"
> ```
