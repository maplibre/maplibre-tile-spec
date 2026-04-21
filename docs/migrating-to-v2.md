# MLT v2 Wire Format Migration Guide

This document provides a detailed, side-by-side comparison of every structural element
between MLT v1 and v2.  It is the authoritative reference for implementing v2 encoders
and decoders.

---

## Guiding Principles

| Principle | v1 | v2 |
|---|---|---|
| Feature count | Not stored; implied by geometry stream `num_values` | `feature_count` varint in every layer header |
| Column layout | Separate metadata section then data section | Column-by-column: type + config + data together |
| Stream type identification | Explicit `stream_type` byte on every stream | Eliminated; role implied by column type and stream position |
| Count per stream | Always stored as varint | Omitted when equal to `feature_count` or `popcount(presence_bitfield)`; required only when neither applies |
| Byte length per stream | Always stored as varint | Encoded in `physical` field: `None-noLen` omits it (derivable as count × width); `None-withLen`, `VarInt`, `FastPFor128` always carry it |
| Presence streams | Bool-RLE stream with 4-field header | Raw packed bitfield, no header; sharable across columns via index reference |
| RLE auxiliary fields | `runs` + `num_rle_values` as varints | Eliminated; both derivable at decode time |
| String encoding variant | `Str`/`OptStr` type + runtime `stream_count` | Encoded in column type code; 8 flat types |
| Dict null encoding | Optional presence stream controls which features are present | Null-at-index-0 in the indices stream; no presence bitfield |
| FastPFor byte order | Big-endian u32 | Little-endian u32 |
| FastPFor block size | 256 | 128 |

> **Note on lazy / skip-ahead parsing.** A reader that only needs a subset of
> columns must be able to advance past unwanted streams without decoding them.
> `physical=None-noLen` streams are always skippable: `byte_length = num_values
> × element_width`, where both are known at header-parse time.  `physical=VarInt`
> and `physical=FastPFor128` streams always carry an explicit `byte_length` (it
> is part of their physical encoding value), so they are also skippable.
> `has_explicit_count` is **not** needed for skipping: `popcount` of a presence
> bitfield is a fast bit-counting operation.  `has_explicit_count = 1` is only
> required when the count is genuinely not derivable from `feature_count` or
> `popcount(presence_bitfield)` — for example, geometry vertex counts and string
> character/offset counts.

---

## Layer Envelope

### v1

`feature_count` is not stored.  It is implied by the length of the geometry types stream.
Metadata and data are completely separated; the metadata section must be fully parsed
before any column data can be located.

```
[varint body_len+1]
[u8    tag = 1]
[varint name_len] [name bytes]
[varint extent]
[varint column_count]
── metadata section ──────────────────────────────
[u8    col_type₀]  [optional: varint name_len + bytes]
[u8    col_type₁]  [optional: varint name_len + bytes]
...
[u8    col_typeN]  [optional: varint name_len + bytes]
── data section ──────────────────────────────────
[streams for column 0]
[streams for column 1]
...
[streams for column N]
```

### v2

```
[varint body_len+1]
[u8    tag = 1]
[varint name_len] [name bytes]
[varint extent]
[varint feature_count]              ← NEW
[varint column_count]
── columns (merged meta + data) ──────────────────
[column 0: type + name? + presence? + data]
[column 1: type + name? + presence? + data]
...
[column N: type + name? + presence? + data]
```

**What changes:**
- `feature_count` is inserted after `extent`.
- The metadata section is eliminated.  Each column is fully self-describing; its type
  byte, optional name, optional presence section, and stream data appear contiguously.

**Derived invariants unlocked by `feature_count`:**

> **Popcount** means "count the number of bits set to `1` in the presence bitfield."
> If a presence bitfield has `feature_count` bits and K of them are `1`, then K features
> have a value and the optional data stream that follows contains exactly K encoded values.
> This document uses `popcount(bitfield)` as shorthand for that count.

| Stream | Count in v1 | Count in v2 |
|---|---|---|
| Geometry types | `num_values` varint | = `feature_count` |
| Non-optional scalar data | `num_values` varint | = `feature_count` |
| Non-optional ID data | `num_values` varint | = `feature_count` |
| Presence bitfield size | stored in header | = `ceil(feature_count / 8)` bytes |
| Optional data stream | `num_values` varint | = `popcount(presence_bitfield)` — number of `1` bits in the bitfield |

---

## Column Layout (Meta + Data Merge)

Every column — whether top-level or a SharedDict child — follows one of three templates
depending on its column type:

**Non-optional** (e.g. `I32`):
```
[u8    column_type]
[varint name_len] [name bytes]        ← only when column_type.has_name() == true
[data section]
```

**With own presence** (e.g. `OptI32`): own bitfield is declared inline, no prefix varint:
```
[u8    column_type]
[varint name_len] [name bytes]
[ceil(feature_count/8) bytes: raw packed bitfield, LSB-first per byte]
  ← this bitfield is registered as the next presence group (0-based, sequential)
[data section]
```

**With shared presence** (e.g. `OptRefI32`): references a previously declared bitfield:
```
[u8    column_type]
[varint name_len] [name bytes]
[varint presence_group]               ← 0-based index of a prior column's bitfield
[data section]
```

`has_name()` is false for `Id`, `OptId`, `LongId`, `OptLongId`, and `Geometry` (same
rule as v1).

### v1 split vs. v2 merged — concrete example

**v1** (two-pass layout, 3 columns: Geometry, Id, OptI32):

```
[varint 3]              ← column_count
[0x04]                  ← Geometry type
[0x00]                  ← Id type
[0x11]                  ← OptI32 type
[varint name_len] "val" ← Id has no name; OptI32 name
[geometry streams...]
[id streams...]
[presence stream for OptI32]
[i32 streams...]
```

**v2** (one-pass layout):

```
[varint feature_count]
[varint 3]              ← column_count
[0x04]                  ← Geometry column_type
[geometry_flags]
[geometry streams...]
[0x00]                  ← Id column_type
[enc_byte] [data...]
[0x11]                  ← OptI32 column_type  (own presence — no prefix varint)
[varint name_len] "val"
[bitfield bytes...]     ← ceil(feature_count/8) bytes; registered as presence group 0
[enc_byte] [data...]
```

---

## Encoding Byte

Every integer stream in v2 is prefixed by exactly one **encoding byte**, whose layout
replaces the two v1 bytes (stream_type + encoding) for streams where the type is implied.

### v1 stream header (4–14 bytes per stream)

```
[u8] stream_type
     high nibble: category (0=Present, 1=Data, 2=Offset, 3=Length)
     low  nibble: subtype  (DictionaryType / OffsetType / LengthType)
[u8] encoding
     bits 7-5: logical1   (0=None, 1=Delta, 2=CwDelta, 3=Rle, 4=Morton, 5=PseudoDecimal)
     bits 4-2: logical2   (secondary; 0=None, 1=Delta, 3=Rle — used only with Morton)
     bits 1-0: physical   (0=None, 1=FastPFor256, 2=VarInt)
[varint] num_values        ← always present
[varint] byte_length       ← always present
[varint] runs              ← RLE streams only
[varint] num_rle_values    ← RLE streams only
[varint] bits              ← Morton streams only
[varint] shift             ← Morton streams only
```

### v2 encoding byte (1–6 bytes per stream)

The byte has no separate flag bits for byte_length or extension presence — both are
implied by the `logical` and `physical` fields, eliminating invalid bit combinations.

```
[u8] encoding_byte
     bit  7: has_explicit_count  (0 = use context count, 1 = varint follows)
     bits 6-4: logical
               0 = None
               1 = Delta
               2 = CwDelta  (ComponentwiseDelta)
               3 = Rle      → physical implied = VarInt; byte_length always follows;
                               bits 3-2 reserved (must be 00)
               4 = DeltaRle → physical implied = VarInt; byte_length always follows;
                               bits 3-2 reserved (must be 00)
               5 = Morton   → extension byte always follows;
                               bits 3-2 are physical (all four values valid)
               6 = PseudoDecimal
               7 = reserved
     bits 3-2: physical  (meaning when logical ∈ {None, Delta, CwDelta, Morton, PseudoDecimal})
               0 = None-noLen   raw fixed-width, byte_length omitted
                                  (derivable: byte_length = num_values × element_width)
               1 = None-withLen raw fixed-width, varint byte_length follows
               2 = VarInt       zigzag/varint encoded,   varint byte_length follows
               3 = FastPFor128  block-SIMD compressed,   varint byte_length follows
     bits 1-0: reserved (must be 0)

[optional: varint num_values]   ← present when has_explicit_count = 1
[optional: varint byte_length]  ← present when physical ∈ {None-withLen, VarInt, FastPFor128},
                                    or when logical ∈ {Rle, DeltaRle}
[optional: extension_byte]      ← present when logical = Morton (always)
     bits 7-6: Morton sub-variant  (00=None, 01=Rle, 10=Delta, 11=reserved)
     bits 5-0: reserved (must be 0)
[optional: varint bits]         ← present when logical = Morton
[optional: varint shift]        ← present when logical = Morton
```

### Byte-length rules

byte_length presence is encoded directly in the `physical` field — there is no
separate flag bit:

| `physical` value | byte_length | Rationale |
|---|---|---|
| `None-noLen` (0) | omitted | `byte_length = num_values × element_width`; both known at header time |
| `None-withLen` (1) | varint follows | Use when explicit length is preferred (e.g. compression later) |
| `VarInt` (2) | varint follows | Cannot skip a VarInt stream without knowing its byte span |
| `FastPFor128` (3) | varint follows | Not self-delimiting |
| `Rle` / `DeltaRle` logical | varint follows | VarInt physical implied; same skippability requirement |

### Count context rules

`has_explicit_count = 0` is valid when the count equals `feature_count` or equals
`popcount(presence_bitfield)`.  `popcount` is a fast bit-counting operation, so
recomputing it on the fly is cheaper than storing an extra varint.  `has_explicit_count
= 1` is required only when neither source applies.

| Stream position | Implicit count (when `has_explicit_count = 0`) |
|---|---|
| Geometry types (first geometry stream) | `feature_count` |
| Non-optional scalar / ID data | `feature_count` |
| Dict index stream (`StrDict`, `StrFsstDict`, child refs) | `feature_count` (all features have an index) |
| Data stream after own presence bitfield | `popcount(presence_bitfield)` |
| Data stream with `presence_ref > 0` | `popcount(referenced_presence_bitfield)` |
| Geometry vertex / aux streams | must use `has_explicit_count = 1` |
| String character / offset / dict-size streams | must use `has_explicit_count = 1` |

### Changes from v1

| Field | v1 | v2 |
|---|---|---|
| `stream_type` byte | Always present (1 byte) | Eliminated (role implied by position) |
| `encoding` byte | Always present (1 byte) | Replaced by `encoding_byte` |
| `has_byte_length` flag | Part of encoding byte | Eliminated; byte_length presence encoded in `physical` field |
| `has_extension` flag | Part of encoding byte | Eliminated; extension byte implied by `logical=Morton` |
| `num_values` | Always present (1–5 bytes) | Conditional: omitted when derivable |
| `byte_length` | Always present (1–5 bytes) | Omitted for `physical=None-noLen`; always present otherwise |
| `runs` (RLE) | Present for RLE (1–5 bytes) | Eliminated |
| `num_rle_values` (RLE) | Present for RLE (1–5 bytes) | Eliminated |
| `bits`/`shift` (Morton) | Present for Morton (2–10 bytes) | In extension byte + varints; extension always present when `logical=Morton` |

---

## Presence Streams

### v1 presence stream

Optional columns prepend a bool-RLE stream with a full stream header:

```
[u8    stream_type = 0x00]           ← Present category, subtype 0
[u8    encoding]
       bits 7-5: logical1 = Rle (3)
       bits 4-2: logical2 = None (0)
       bits 1-0: physical = None (0)  ← or VarInt
[varint num_values]                  ← = feature_count
[varint byte_length]
[varint runs]                        ← RLE pair count
[varint num_rle_values]              ← = feature_count
[RLE-packed bits...]
```

The stream uses bool-RLE encoding: pairs of `(run_length, value)` where value is 0
(absent) or 1 (present).  Header overhead: 4–14 bytes before the data.

### v2 presence section

Presence is encoded in the **column type** itself, not in a prefix varint, giving three
distinct variants for any nullable type:

**`Opt*` (own presence)** — bitfield immediately follows the optional name; no leading varint:
```
[ceil(feature_count/8) bytes: raw packed bits, LSB-first within each byte]
← registered as the next presence group (0-based sequential index)
← num_values for the data stream = popcount(this bitfield)
```

**`OptRef*`** — references a prior column's already-declared bitfield:
```
[varint presence_group]   ← 0-based index of the group to reuse
← no bitfield bytes follow
← num_values for the data stream = popcount(referenced bitfield)
```

**What changes:**

| Aspect | v1 | v2 |
|---|---|---|
| Encoding | Bool-RLE pairs | Raw packed bitfield (1 bit per feature) |
| Header overhead | 4–14 bytes | **0 bytes** for `Opt*` (bitfield directly follows column type); 1 byte (varint) for `OptRef*` |
| Size known in advance | No (stored in `byte_length`) | Yes: always `ceil(feature_count / 8)` bytes |
| Sharing identical bitfields | Not possible | `OptRef*` type with group index; bitfield stored once |
| Null count for data stream | Stored in `num_rle_values` | `popcount(bitfield)`, computed at decode time |
| "Own bitfield" marker | n/a | Encoded in column type — no `presence_ref = 0` byte needed |

**Presence group numbering** is layer-wide and sequential.  Every `Opt*` column
(top-level or SharedDict child) increments the counter when its bitfield is parsed.

**Dict variants** (`OptStrDict`, `OptStrFsstDict`, `OptSharedDictChildRef`) do **not**
have a presence section; they encode null via index 0 in the indices stream and therefore
have no corresponding `OptRef*` variant either.

---

## Scalar and ID Columns

Covers: `Bool`, `I8`, `U8`, `I32`, `U32`, `I64`, `U64`, `F32`, `F64`, `Id`, `LongId`
and their `Opt*` counterparts.

### v1 layout

```
── metadata section (written before all column data) ──
[u8 column_type]
[optional: varint name_len + bytes]

── data section ──
[optional presence stream]   ← full stream header + bool-RLE data
[stream_type byte]           ← always 0x10 (Data, DictionaryType::None)
[encoding byte]
[varint num_values]          ← = feature_count (non-optional) or number of 1-bits in presence (optional)
[varint byte_length]
[encoded data bytes]
```

### v2 layout

Non-optional:
```
[u8 column_type]                    ← e.g. I32
[optional: varint name_len + bytes]
[encoding_byte]
[optional: varint num_values]       ← only when has_explicit_count = 1
[optional: varint byte_length]      ← present when physical ∈ {None-withLen, VarInt, FastPFor128}
[encoded data bytes]
```

Optional — own presence (`Opt*`):
```
[u8 column_type]                    ← e.g. OptI32
[optional: varint name_len + bytes]
[ceil(feature_count/8) bytes: bitfield]   ← no prefix varint
[encoding_byte] [optional count/size] [data bytes]
```

Optional — shared presence (`OptRef*`):
```
[u8 column_type]                    ← e.g. I32SharedPresence
[optional: varint name_len + bytes]
[varint presence_group]             ← 0-based group index
[encoding_byte] [optional count/size] [data bytes]
```

**What changes per column:**

| | v1 bytes | v2 bytes |
|---|---|---|
| Column type in metadata section | 1 (in meta pass) | 1 (inline) |
| Name in metadata section | 0–33 (in meta pass) | 0–33 (inline) |
| Presence stream header | 4–14 | 0 (`Opt*` — bitfield directly follows) / 1 (`OptRef*` — group ref varint) |
| Presence data | RLE packed | Raw packed bits |
| `stream_type` byte | 1 | 0 (eliminated) |
| `num_values` | 1–5 | 0 (derivable from `feature_count` or presence popcount) |
| `byte_length` | 1–5 | 0 for `None-noLen`; 1–5 for `None-withLen`, VarInt, FastPFor128 |

**Element widths** used for `physical=None-noLen` byte-length derivation:

| Column types | Element width |
|---|---|
| `Bool`, `I8`, `U8` | 1 byte |
| `I32`, `U32`, `F32` | 4 bytes |
| `I64`, `U64`, `F64`, `Id`, `LongId` | 8 bytes |

---

## Geometry Column

### v1 layout

```
── metadata section ──
[u8 column_type = 0x04]    ← Geometry; no name written

── data section ──
[varint stream_count]       ← number of geometry streams that follow
[stream 0: types]
  [u8  stream_type = 0x10]  ← Data(None)
  [u8  encoding]
  [varint num_values]       ← = feature_count
  [varint byte_length]
  [geometry type data]
[stream 1..N: auxiliary streams, each with full header]
  [u8  stream_type]         ← identifies role: Data(Vertex/Morton),
                               Offset(Vertex/Index), Length(Geometries/Parts/Rings/Triangles)
  [u8  encoding]
  [varint num_values]
  [varint byte_length]
  [stream data]
```

Stream presence is implicit: `stream_count` streams follow, each identified by its
`stream_type` byte.

### v2 layout

The `geometry_flags` byte is eliminated entirely.  The column type byte is one of
the named `Geo*` types below, which unambiguously specifies which streams are
present and in what order.  No name is written (same rule as v1).

```
[u8 column_type]    ← one of the Geo* column types; no name bytes follow
[streams in fixed order determined by column_type; see table below]
```

Every stream: `[encoding_byte] [optional: varint num_values] [optional: varint byte_length] [data]`

| Column type | Streams (in order, all `has_explicit_count = 1` except Types) |
|---|---|
| `GeoPoints` | Types¹, Vertices |
| `GeoPointsDict` | Types¹, VertexData (dict), VertexOffsets |
| `GeoMultiPoints` | Types¹, GeoLengths, Vertices |
| `GeoMultiPointsDict` | Types¹, GeoLengths, VertexData (dict), VertexOffsets |
| `GeoLines` | Types¹, PartLengths, Vertices |
| `GeoLinesDict` | Types¹, PartLengths, VertexData (dict), VertexOffsets |
| `GeoMultiLines` | Types¹, GeoLengths, PartLengths, Vertices |
| `GeoMultiLinesDict` | Types¹, GeoLengths, PartLengths, VertexData (dict), VertexOffsets |
| `GeoPolygons` | Types¹, PartLengths, RingLengths, Vertices |
| `GeoPolygonsDict` | Types¹, PartLengths, RingLengths, VertexData (dict), VertexOffsets |
| `GeoMultiPolygons` | Types¹, GeoLengths, PartLengths, RingLengths, Vertices |
| `GeoMultiPolygonsDict` | Types¹, GeoLengths, PartLengths, RingLengths, VertexData (dict), VertexOffsets |
| `GeoTessPolygons` | Types¹, TriLengths, IndexBuffer, Vertices |
| `GeoTessPolygonsWithOutlines` | Types¹, GeoLengths, PartLengths, RingLengths, TriLengths, IndexBuffer, Vertices |

¹ Types stream: `has_explicit_count = 0` (count = `feature_count`)

**Dict columns** store a deduplicated vertex dictionary in VertexData and per-vertex
indices in VertexOffsets.  The vertex encoding (plain delta, CwDelta, Morton) is
specified by the `logical` field in the VertexData encoding byte.

**Mixed-type columns** use the type whose stream set is a superset of all geometry
types present:
- Point + LineString → `GeoLines` (Points consume no PartLengths entry)
- LineString + Polygon → `GeoPolygons` (LineString vertex counts stored in RingLengths)
- Polygon + MultiPolygon → `GeoMultiPolygons`

**What changes:**

| Aspect | v1 | v2 |
|---|---|---|
| Stream presence declaration | `varint stream_count` + `stream_type` per stream | Encoded in column type |
| `stream_count` varint | 1–5 bytes | Eliminated |
| `geometry_flags` byte | n/a (v1 has no flags) | Eliminated (merged into column type) |
| `stream_type` per stream | 1 byte × N streams | Eliminated |
| `num_values` for types stream | varint (= feature_count) | Omitted |
| Vertex encoding style | Read from `stream_type` subtype | Read from `logical` field in encoding byte |

---

## String Columns

### Overview of changes

| | v1 | v2 |
|---|---|---|
| Column types | `Str (28)`, `OptStr (29)` | 8 explicit types (28–35) |
| Encoding variant declaration | `varint stream_count` (2–5 determines variant) | Encoded in column type code |
| `stream_count` varint | 1–5 bytes | Eliminated |
| `stream_type` per stream | 1 byte × N streams | Eliminated (position-implied) |
| Null encoding for dict variants | Presence bitfield (separate stream) | Null-at-index-0 in indices stream |
| Presence sharing | Not possible | Dict variants: none; plain variants: shared presence group |

---

### 8.1 StrPlain / OptStrPlain

A plain string column stores per-feature string lengths and the concatenated string data.

**v1 layout (`Str`, non-optional):**

```
── metadata ──  [u8 = 28]  [varint name_len + bytes]
── data ──
[varint stream_count = 2]
[stream 0: lengths]
  [u8  stream_type = 0x31]   ← Length(VarBinary)
  [u8  encoding]
  [varint num_values]         ← = feature_count
  [varint byte_length]
  [length data]
[stream 1: string data]
  [u8  stream_type = 0x10]   ← Data(None)
  [u8  encoding = raw/None]
  [varint num_values]         ← = total byte count of all strings
  [varint byte_length]        ← = same
  [raw UTF-8 bytes]
```

**v1 layout (`OptStr`, optional):**

```
── metadata ──  [u8 = 29]  [varint name_len + bytes]
── data ──
[varint stream_count = 3]
[presence stream]             ← full bool-RLE header + data
[stream 1: lengths]           ← num_values = number of 1-bits in presence bitfield
[stream 2: string data]
```

**v2 layout (`StrPlain`, non-optional):**

```
[u8 = 28]  [varint name_len + bytes]
[encoding_byte for lengths]        ← has_explicit_count=0 (= feature_count)
[lengths data]
[encoding_byte for string data]
  has_explicit_count=1             ← total byte count ≠ feature_count
  logical=None, physical=None-noLen  ← byte_length = num_values × 1 (trivial)
[varint num_values]                ← total UTF-8 byte count
[raw UTF-8 bytes]
```

**v2 layout (`OptStrPlain`, optional — own presence):**

```
[u8 = 29]  [varint name_len + bytes]
[ceil(feature_count/8) bytes: bitfield]   ← no prefix varint; registered as next group
[encoding_byte for lengths]        ← has_explicit_count=0; count = popcount(bitfield)
[lengths data]
[encoding_byte for string data]
  has_explicit_count=1
  logical=None, physical=None-noLen
[varint num_values]
[raw UTF-8 bytes]
```

**v2 layout (`StrPlainSharedPresence`, optional — shared presence):**

```
[u8 = StrPlainSharedPresence]  [varint name_len + bytes]
[varint presence_group]        ← 0-based group index
[encoding_byte for lengths]    ← has_explicit_count=0; count = popcount(referenced group)
[lengths data]
[encoding_byte for string data] has_explicit_count=1, logical=None, physical=None-noLen
[varint num_values]
[raw UTF-8 bytes]
```

---

### 8.2 StrDict / OptStrDict

A dictionary string column stores unique string values in a dictionary and per-feature
indices into that dictionary.

**Null encoding change — the most significant difference for dict columns:**

| | v1 | v2 |
|---|---|---|
| Optional dict column | Presence bitfield controls which features have values; index stream only covers present features | No presence bitfield; **index 0 = null**; index stream covers all `feature_count` features |
| Non-optional dict column | Presence absent; index stream covers all features | Same; index 0 is a valid dict entry (decoder assumes non-null) |
| Index count | = number of `1` bits in presence, or `feature_count` | Always `feature_count` |

**v1 layout (`OptStr` + dictionary variant, stream_count = 4):**

```
── metadata ──  [u8 = 29]  [varint name_len + bytes]
── data ──
[varint stream_count = 4]
[presence stream]              ← bool-RLE; num_values = feature_count
[offsets stream]               ← Offset(String); per-present-feature dict indices
  [u8  stream_type = 0x23]
  [u8  encoding]
  [varint num_values]          ← = number of 1-bits in presence bitfield
  [varint byte_length]
  [offset data]
[dict lengths stream]          ← Length(Dictionary)
  [u8  stream_type = 0x36]
  [u8  encoding]
  [varint num_values]          ← = dict size
  [varint byte_length]
  [dict length data]
[dict data stream]             ← Data(Single/Shared)
  [u8  stream_type = 0x11/0x12]
  [u8  encoding = raw/None]
  [varint num_values]          ← = total dict UTF-8 bytes
  [varint byte_length]
  [raw UTF-8 dict bytes]
```

**v2 layout (`OptStrDict`, optional):**

```
[u8 = 31]  [varint name_len + bytes]
← NO presence section for dict variants
[encoding_byte for dict_lengths]
  has_explicit_count=1             ← dict size ≠ feature_count
[varint num_values]                ← dict entry count
[dict_lengths data]
[encoding_byte for dict_data]
  has_explicit_count=1
  logical=None, physical=None-noLen  ← byte_length = num_values × 1
[varint num_values]                ← total dict UTF-8 bytes
[raw UTF-8 dict bytes]
[encoding_byte for indices]
  has_explicit_count=0             ← = feature_count (index 0 = null, all features present)
[indices data]                     ← index 0 = null; indices 1..N map to dict entries 0..N-1
```

**v2 layout (`StrDict`, non-optional):**

Identical wire format to `OptStrDict`.  The column type code (30 vs 31) tells the
decoder whether index 0 should be treated as null.

---

### 8.3 StrFsst / OptStrFsst

FSST-plain compresses string data using a per-column symbol table.  The column stores a
symbol table, per-value lengths, and the compressed corpus.

**FSST symbol table stream** (same for all FSST variants):

```
[encoding_byte]
  has_explicit_count=1
  logical=None, physical=None-noLen  ← raw bytes; byte_length = num_values × 1
[varint num_values]                  ← number of raw symbol table bytes
[symbol table bytes]
```

Symbol lengths (how long each symbol is) precede the symbol table bytes.

**v1 layout (`OptStr` + FSST-plain variant, stream_count = 5):**

```
[varint stream_count = 5]
[presence stream]                     ← bool-RLE header
[symbol_lengths stream]               ← Length(Symbol)
[symbol_table stream]                 ← Data(Fsst)
[per-value lengths stream]            ← Length(Dictionary)
[compressed corpus stream]            ← Data(Single/Shared)
```

Each stream has a full 4-field header plus optional RLE metadata.

**v2 layout (`OptStrFsst`, optional — own presence):**

```
[u8 = 33]  [varint name_len + bytes]
[ceil(feature_count/8) bytes: bitfield]   ← no prefix varint; registered as next group
[encoding_byte] [symbol_lengths data] ← has_explicit_count=1
[encoding_byte] [symbol_table data]   ← has_explicit_count=1, logical=None, physical=None-noLen
[encoding_byte] [per-value lengths]   ← has_explicit_count=0; count = popcount(bitfield)
[encoding_byte] [compressed corpus]   ← has_explicit_count=1, logical=None, physical=None-noLen
```

**v2 layout (`StrFsst`, non-optional):**

Same but no presence section; per-value lengths count = `feature_count`.

---

### 8.4 StrFsstDict / OptStrFsstDict

FSST-dictionary stores the dictionary corpus FSST-compressed.  Per-feature values are
indices into the decoded dictionary.

**v1 layout (`OptStr` + FSST-dictionary variant, stream_count = 6):**

```
[varint stream_count = 6]
[presence stream]                     ← bool-RLE
[symbol_lengths stream]
[symbol_table stream]
[dict_lengths stream]
[compressed dict corpus stream]
[per-feature offsets stream]          ← Offset(String); indices per present feature
```

**v2 layout (`OptStrFsstDict`, optional):**

```
[u8 = 35]  [varint name_len + bytes]
← NO presence section
[encoding_byte] [symbol_lengths data] ← has_explicit_count=1
[encoding_byte] [symbol_table data]   ← has_explicit_count=1, logical=None, physical=None-noLen
[encoding_byte] [dict_lengths data]   ← has_explicit_count=1 (dict size ≠ feature_count)
[encoding_byte] [dict_corpus data]    ← has_explicit_count=1, logical=None, physical=None-noLen
[encoding_byte] [indices data]        ← has_explicit_count=0 (= feature_count); index 0 = null
```

**v2 layout (`StrFsstDict`, non-optional):**

Same wire format; decoder does not treat index 0 as null.

---

## SharedDict Columns

A SharedDict column stores a shared string corpus (plain or FSST-compressed) that
multiple child columns index into.

### Overview of changes

| Aspect | v1 | v2 |
|---|---|---|
| Column types | `SharedDict (30)` — single type with runtime variant | `SharedDictPlain (36)`, `SharedDictFsst (37)` — flat types |
| Corpus encoding variant | Runtime: detected from stream types within stream_count group | Static: encoded in column type code |
| Children | `varint stream_count` + optional presence + offset stream per child | Full column definitions with type byte (`SharedDictChildRef`/`OptSharedDictChildRef`) |
| Optional child null encoding | Presence bitfield (separate stream per optional child) | Null-at-index-0 in the indices stream |
| Shared presence for children | Not possible | Possible for plain/FSST variants; **not** for dict children (null-at-0 convention) |

### v1 layout

```
── metadata ──  [u8 = 30]  [varint name_len + bytes]
                [varint child_col_count]
                [u8 child_type₀] [varint name_len + bytes]
                ...
── data ──
[varint stream_count]            ← dict streams + (1 or 2 per child)
[dict stream(s)]                 ← lengths + data, or FSST 4-stream group
[per child:]
  [varint child_stream_count]    ← 1 or 2 depending on optionality
  [optional presence stream]     ← bool-RLE header + data
  [offset/index stream]          ← Offset(Key) or Offset(String)
    [u8  stream_type = 0x23]
    [u8  encoding]
    [varint num_values]
    [varint byte_length]
    [index data]
```

### v2 layout (SharedDictPlain)

```
[u8 = 36]  [varint name_len + bytes]
[encoding_byte] [dict_lengths data]  ← has_explicit_count=1
[encoding_byte] [dict_data bytes]    ← has_explicit_count=1, logical=None, physical=None-noLen

[varint child_column_count]
[child 0:]
  [u8 = 38 (SharedDictChildRef) or 39 (OptSharedDictChildRef)]
  [varint name_len + bytes]
  ← NO presence section for child ref types (null-at-0 for optional)
  [encoding_byte] [indices data]
    has_explicit_count=0             ← = feature_count for both ref types; index 0 = null when type = 39
[child 1:]
  ...
```

### v2 layout (SharedDictFsst)

```
[u8 = 37]  [varint name_len + bytes]
[encoding_byte] [symbol_lengths data] ← has_explicit_count=1
[encoding_byte] [symbol_table data]   ← has_explicit_count=1, logical=None, physical=None-noLen
[encoding_byte] [dict_lengths data]   ← has_explicit_count=1
[encoding_byte] [dict_corpus data]    ← has_explicit_count=1, logical=None, physical=None-noLen

[varint child_column_count]
[children identical to SharedDictPlain children...]
```

### Child type codes

| Column type | Code | Null handling |
|---|---|---|
| `SharedDictChildRef` | 38 | No nulls; index 0 is a valid dict entry |
| `OptSharedDictChildRef` | 39 | index 0 = null; all other indices map to dict |

---

## RLE Encoding

### v1 RLE stream header extras

All RLE and DeltaRle streams (except bool-RLE presence streams) carry two extra varints
after `byte_length`:

```
[varint runs]             ← number of (run_length, value) pairs
[varint num_rle_values]   ← sum of all run_lengths = total decoded element count
```

Both fields are redundant: they can be derived from the stream data itself.

### v2 changes

Both fields are **eliminated**:

- `num_rle_values` = `num_values`, already known from context (`feature_count`,
  presence popcount, or explicit count in encoding byte).
- `runs` is recoverable by scanning: the data is interleaved `[run_length_varint,
  value_varint]` pairs, so `runs = total_pairs` (found by reading to byte_length).

Bool-RLE is eliminated entirely; presence data uses raw packed bitfields (§5).

### Encoding byte for RLE streams

| Logical | Encoding byte bits 6-4 | Physical implied | byte_length |
|---|---|---|---|
| `Rle` | `011` | VarInt | always follows |
| `DeltaRle` | `100` | VarInt | always follows |

bits 3-2 (physical field) must be `00` for Rle/DeltaRle; they are reserved for future
RLE sub-variants.  No extension byte follows.

---

## Morton Encoding

### v1 Morton header extras

Morton streams carry `bits` and `shift` after `byte_length`:

```
[varint bits]      ← number of bits used per coordinate component
[varint shift]     ← coordinate shift value
```

Morton sub-variant (plain/delta/rle) is encoded via `logical2` in the encoding byte.

### v2 changes

`bits` and `shift` are moved to an **extension byte** that is unconditionally present
whenever `logical = Morton` (no flag needed):

```
[encoding_byte]
  logical = 5 (Morton)        ← bits 6-4
  physical = ...              ← bits 3-2 (all four values valid for Morton)
[extension_byte]              ← always present; no flag
  bits 7-6: Morton sub-variant  (00=None, 01=Rle, 10=Delta, 11=reserved)
  bits 5-0: reserved (must be 0)
[varint bits]
[varint shift]
```

The `logical2` field in v1's encoding byte is replaced by the Morton sub-variant in the
extension byte.  The extension byte is implied by `logical=Morton` rather than by an
explicit `has_extension` flag.

---

## FastPFor Codec

### v1 FastPFor wire format

```
[u32 BE]  N = number of FastPFor-compressed u32 words
[N × u32 BE]  FastPFor primary codec output
[remaining u32 BE words]  VariableByte remainder codec output
```

All u32 words are stored **big-endian**.  Block size is **256**.

### v2 FastPFor wire format

```
[u32 LE]  N = number of FastPFor-compressed u32 words
[N × u32 LE]  FastPFor primary codec output
[remaining u32 LE words]  VariableByte remainder codec output
```

Changes:
- All u32 words are stored **little-endian**.
- Block size is **128**.
- `physical=FastPFor128` (value 3 in bits 3-2) always carries `byte_length`; it marks
  where the FastPFor stream ends and the next stream begins.

---

## Column Type Codes

### v1 column type codes

| Code | Type | Code | Type |
|------|------|------|------|
| 0 | Id | 1 | OptId |
| 2 | LongId | 3 | OptLongId |
| 4 | Geometry | — | — |
| 10 | Bool | 11 | OptBool |
| 12 | I8 | 13 | OptI8 |
| 14 | U8 | 15 | OptU8 |
| 16 | I32 | 17 | OptI32 |
| 18 | U32 | 19 | OptU32 |
| 20 | I64 | 21 | OptI64 |
| 22 | U64 | 23 | OptU64 |
| 24 | F32 | 25 | OptF32 |
| 26 | F64 | 27 | OptF64 |
| 28 | Str | 29 | OptStr |
| 30 | SharedDict | — | — |

### v2 column type codes

Specific numeric assignments are deferred to the final spec; only the variant names
are defined here.

Every nullable scalar/string type has **three** variants:

| Variant suffix | Presence mechanism |
|---|---|
| *(none)* — non-optional | No presence |
| `Opt*` — own-presence | Bitfield directly follows name; no prefix varint |
| `OptRef*` — shared-presence | `varint presence_group` follows name |

**ID and scalar types:**

| Non-optional | Own-presence | Shared-presence |
|---|---|---|
| `Id` | `OptId` | `OptRefId` |
| `LongId` | `OptLongId` | `OptRefLongId` |
| `Bool` | `OptBool` | `OptRefBool` |
| `I8` | `OptI8` | `OptRefI8` |
| `U8` | `OptU8` | `OptRefU8` |
| `I32` | `OptI32` | `OptRefI32` |
| `U32` | `OptU32` | `OptRefU32` |
| `I64` | `OptI64` | `OptRefI64` |
| `U64` | `OptU64` | `OptRefU64` |
| `F32` | `OptF32` | `OptRefF32` |
| `F64` | `OptF64` | `OptRefF64` |

**String and shared-dict types:**

| Non-optional | Own-presence | Shared-presence |
|---|---|---|
| `StrPlain` | `OptStrPlain` | `OptRefStrPlain` |
| `StrDict` | — (null-at-0; no presence bitfield) |
| `StrFsst` | `OptStrFsst` | `OptRefStrFsst` |
| `StrFsstDict` | — (null-at-0; no presence bitfield) |
| `SharedDictPlain` | — | — |
| `SharedDictFsst` | — | — |
| `SharedDictChildRef` | — (null-at-0; no presence bitfield) |

**Geometry types** (no optional variants; geometry columns are always present):

| Column type | Description |
|---|---|
| `GeoPoints` | Point geometries |
| `GeoPointsDict` | Point geometries with vertex dictionary |
| `GeoMultiPoints` | MultiPoint geometries |
| `GeoMultiPointsDict` | MultiPoint geometries with vertex dictionary |
| `GeoLines` | LineString / mixed point+line geometries |
| `GeoLinesDict` | LineString geometries with vertex dictionary |
| `GeoMultiLines` | MultiLineString geometries |
| `GeoMultiLinesDict` | MultiLineString geometries with vertex dictionary |
| `GeoPolygons` | Polygon / mixed geometries |
| `GeoPolygonsDict` | Polygon geometries with vertex dictionary |
| `GeoMultiPolygons` | MultiPolygon geometries |
| `GeoMultiPolygonsDict` | MultiPolygon geometries with vertex dictionary |
| `GeoTessPolygons` | Tessellated polygons (index buffer, no ring topology) |
| `GeoTessPolygonsWithOutlines` | Tessellated polygons with outline ring topology |

**Invariants:**
- Dict-family string types (`OptStrDict`, `OptStrFsstDict`, `OptSharedDictChildRef`) have no
  shared-presence variant because they encode null via index 0, not via a presence bitfield.
- Shared-presence (`OptRef*`) code = own-presence code with bit 7 set.
- Geometry column types have no optional variants; a missing geometry column is simply absent
  from the layer.

---

## Stream Type Bytes (Eliminated)

In v1 every stream begins with a `stream_type` byte that encodes the stream's role:

```
bits 7-4: category  (0=Present, 1=Data, 2=Offset, 3=Length)
bits 3-0: subtype   (DictionaryType / OffsetType / LengthType)
```

In v2 this byte is **eliminated from the wire format entirely**.  Stream role is always
determinable without it:

| Column type | How stream role is known |
|---|---|
| Scalar / ID | Single data stream; role is trivial |
| `Geo*` | Column type determines which streams are present; position in fixed sequence declares role |
| `StrPlain` / `OptStrPlain` | Positions: 1=lengths, 2=string-data |
| `StrDict` / `OptStrDict` | Positions: 1=dict-lengths, 2=dict-data, 3=indices |
| `StrFsst` / `OptStrFsst` | Positions: 1=sym-lengths, 2=sym-table, 3=lengths, 4=corpus |
| `StrFsstDict` / `OptStrFsstDict` | Positions: 1=sym-lengths, 2=sym-table, 3=dict-lengths, 4=dict-corpus, 5=indices |
| `SharedDictPlain` / `SharedDictFsst` | Corpus streams in fixed order; children are self-describing column definitions |

The `StreamType` concept (`Present`, `Data`, `Offset`, `Length`) and its subtypes
(`DictionaryType`, `OffsetType`, `LengthType`) remain useful as descriptive labels for
documentation and in-memory representations, but they are no longer serialized to the
wire in v2.

---

## Overhead Comparison Table

Per-stream header costs, typical small-integer varint values.

| Stream Kind                            | v1 overhead (bytes) | v2 overhead (bytes) | Saving |
|----------------------------------------|---|---|---|
| Presence bitfield header (`Opt*`)      | 4–14 | 0 (bitfield directly follows column type) | 4–14 |
| Shared presence (`OptRef*`)            | full bitfield copy | 1 (presence_group varint) | ≈ ceil(N/8) |
| Non-optional scalar, VarInt            | 4–6 | 1 (enc) + varint(size) | 2–4 |
| Non-optional scalar, None-noLen        | 4–6 | 1 | 3–5 |
| Non-optional scalar, FastPFor128       | 5–7 | 1 (enc) + varint(size) | 2–4 |
| Optional scalar, VarInt                | 4–6 | 1 (enc) + varint(size) | 2–4 |
| RLE stream, VarInt                     | 6–10 | 1 | 5–9 |
| Geometry: stream_count varint          | 1–3 | 0 (encoded in column type) | 1–3 |
| Geometry: types stream                 | 4–6 | 1 | 3–5 |
| Geometry: aux stream, FastPFor         | 6–10 | 1 + varint(count) + varint(size) | 2–4 |
| Morton vertex stream                   | 8–14 | 1 + ext + 2 varints | 4–8 |
| String variant declaration             | 1 (Str/OptStr type) + 1–3 (stream_count) | 0 | 2–4 |
| Dict optional: presence + index stream | 4–10 + bool-RLE | 0 (null-at-0) | 4–10 + bitfield |

**Estimated total saving for a typical tile (100 streams, 10 optional columns):**
600–1 000 bytes of stream-header metadata, plus elimination of bool-RLE presence data
(replaced by compact bitfields).
