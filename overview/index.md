# Overview

This page describes the MLT data model used by the [MLT v1](<https://maplibre.org/maplibre-tile-spec/specification/v1/index.md>) and [MLT v2](<https://maplibre.org/maplibre-tile-spec/specification/v2/index.md>) formats.

# Tiles

MLT defines the structure of a single tile. As with MVT, the tile grid, map projection, and anything else outside one tile are out of its scope. Each tile is self-contained: nothing outside it is needed to decode it, though drawing it usually takes a separate [style](<https://maplibre.org/maplibre-style-spec/>).

Typically, a dataset is cut into a pyramid of square tiles addressed as `z/x/y`, and a map loads only the tiles currently on screen. Tiles are commonly stored in [PMTiles](<https://github.com/protomaps/PMTiles>) or MBTiles archives and served over HTTP, for example by the [Martin tile server](<https://maplibre.org/martin/>).

# Frames

An MLT tile is a sequence of frames, with no common header. Each frame starts with its byte size and a format version tag, followed by the body in that format. Once a format is finalized, its byte layout and capabilities do not change. New capabilities go into a new format with a new tag.

A decoder that supports a tag MUST support every feature of that format. An encoder MUST produce valid data for its tag, but MAY choose not to use some encodings. A decoder skips a frame whose tag it does not know, so new formats can be added without breaking existing readers. One tile can therefore mix formats: MLT v1 and v2 layers today, and other kinds of data in future formats, such as 3D or non-visual data.

| Tag | Format | Contents | Status |
| --- | --- | --- | --- |
| `0x01` | [MLT v1](<https://maplibre.org/maplibre-tile-spec/specification/v1/index.md>) | 2D features with scalar properties, compatible with MVT. | Stable. Implemented by every shipped encoder and decoder. |
| `0x02` | [MLT v2](<https://maplibre.org/maplibre-tile-spec/specification/v2/index.md>) | v1 content in a smaller layout, plus per-vertex Z and M values and nested properties. | Under development. The wire format may change without notice. |

# Geometry and extent

Geometry in a tile lives on a flat, local grid, not in longitude and latitude. Each layer declares an `extent`, and vertices are signed integers where `(0, 0)` is the tile's top-left corner and `(extent, extent)` its bottom-right. As in MVT, `x` grows to the right and `y` grows down.

A tile has no notion of where it is in the world. Only its `z/x/y` address, kept outside the tile, places it on the map. Tiles with identical content, such as open water or the inside of a large park, can therefore be stored once and reused at many addresses.

Integer coordinates compress well, since neighboring vertices differ by small numbers. `4096` is the conventional extent and the encoder default. A larger extent gives more precision but costs more bytes. Coordinates MAY be negative or exceed `extent`. This lets geometry crossing a tile boundary keep its shape, so that lines and polygon edges meet across the seam instead of being clipped to it.

# Layers and features

In MLT v1 and v2, each frame is a layer, called a `FeatureTable` in the specification. A layer is a thematic group of data such as `water`, `roads` or `place_labels`, and is the unit a [MapLibre Style](<https://maplibre.org/maplibre-style-spec/>) targets. Layers are equivalent to layers in MVT.

A feature has:

- a geometry, typed by the OGC Simple Feature Access model, excluding `GeometryCollection`
- an optional id
- any number of properties

Features in one layer share a single set of property columns and normally share one geometry type. Mixing geometry types in a layer is allowed, but one type per layer compresses better.

> [!NOTE]
>
> The terms `column`, `field`, `attribute`, and `property` are used interchangeably in the specification.

# Columns, not records

MVT is record-oriented: it writes feature 1 whole, then feature 2 whole, each with its own tags and geometry commands. MLT is column-oriented: it writes every feature's `id`, then every feature's geometry, then every feature's `class`, and so on.

The column layout has the following consequences:

- **Compression.** A column of road classes is a few distinct strings repeated many times, which a dictionary collapses. A column of sequential ids has a constant delta. Neither pattern is visible when the values are spread across records.
- **Decoding speed.** A column decodes with one loop over one buffer, which vectorizes. A record stream decodes with a branchy loop over mixed types.
- **GPU upload.** Vertices are already contiguous, so they can be copied to a GPU buffer without gathering.
- **Partial decoding.** A renderer that needs only the geometry and one property can skip the other columns.

# Streams

A column is split into several streams, each a contiguous run of same-typed values. This follows the [ORC](<https://orc.apache.org/specification/ORCv1/>) file format. Each stream carries its own metadata: value count, byte length and encoding.

A nullable string column, for example, becomes:

- a present stream, one bit per feature, saying which features have a value
- a length stream, the byte length of each string
- a data stream, the UTF-8 bytes

MLT defines these stream kinds:

| Stream | Holds |
| --- | --- |
| **Present** | One bit per feature: does this feature have a value? Omitted for a column that cannot be null. |
| **Data** | The values themselves: booleans, integers, floats, strings, dictionary codes, or vertex coordinates. |
| **Length** | Element counts for variable-sized values: string byte lengths, ring vertex counts, list sizes. |
| **Offset** | Indices into another stream, used by dictionary encodings. |

Each stream is encoded independently. Lengths are usually small and repetitive, so they run-length encode well. String bytes do not, but strings often dictionary-encode well.

# Encodings

Every stream has its own lightweight encoding: delta, run-length, dictionary, bit packing, FSST, and others. Lightweight means cheap enough to decode at render time.

Encodings cascade. Dictionary encoding turns a string column into a stream of integer codes, and that integer stream is then delta-encoded or bit-packed like any other.

Encoders choose encodings by sampling, since trying every combination is too expensive; see [choosing an encoding](<https://maplibre.org/maplibre-tile-spec/implementation-guide/#choosing-an-encoding>). The [encoding algorithms](<https://maplibre.org/maplibre-tile-spec/encodings/index.md>) page describes each scheme. The specifications say which encodings each stream may use.

Tiles are usually also gzip- or brotli-compressed in transit.

# Sorting

Feature order is the encoder's choice. Sorting a layer by a low-cardinality property groups equal values into runs that RLE collapses. Sorting by spatial locality shortens vertex deltas.

MLT does not fix an order. Encoders MAY reorder features. Applications that depend on source order must disable sorting or carry an explicit ordering property.

# MVT compatibility

MLT can represent the same common vector tile content as MVT, but it is not a byte-for-byte or schema-free replacement. The differences come from MVT's per-feature tag/value model versus MLT's per-layer column model:

- **Property types are fixed per layer.** In MVT, the same key can technically reference values of different data types on different features in a layer. In MLT, one property name corresponds to one column, and that column has one declared type for the whole layer. MVT data with mixed types must therefore be normalized before or during conversion, for example by lossless numeric widening, coercing values to strings, dropping mismatched values, or rejecting the tile.
- **Missing properties become typed nulls.** MVT normally represents a missing property by omitting the key/value tag from that feature; the MVT value union has no dedicated null type. MLT represents the union of layer properties as columns, so a feature that lacks a property stores a null in that column and the column is marked nullable. When converting MLT back to MVT, null property values should be omitted from the feature tags.
- **A feature has at most one value per property column.** MVT tag streams can encode the same key more than once for a single feature, even though most MVT APIs expose properties as a map and collapse such duplicates. MLT has one cell per feature per column, so duplicate keys on one feature must be rejected, collapsed deterministically, or renamed before encoding.
- **Feature order is not necessarily a stable round-trip property.** MVT stores features in wire order. MLT encoders may preserve order, but they may also sort features by id or spatial locality to improve compression when that optimization is enabled.
- **Layer names must be non-empty.** MLT always requires a non-empty layer name, and a layer without one MUST be rejected. MVT leaves this unclear: its protobuf schema marks `name` as required, but the written specification never says it cannot be an empty string. Some Mapbox tooling rejects empty names.
- **A layer holds at most `2^31 - 1` features.** MVT sets no limit, but MLT stores the feature count as a signed 32-bit integer.
- **MLT v2 restricts the extent.** MVT allows any positive extent, but MLT v2 accepts only powers of two from `64` to `2097152`. A layer with any other extent must be rescaled or stored as MLT v1.
