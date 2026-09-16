<h1>Encoding Definitions</h1>

---

This page specifies the compression schemes used by MLT streams.
Which streams may use which scheme, and how the choice is encoded, is in the [v1](specification/v1.md) and [v2](specification/v2.md) specifications.

Encodings are split into logical techniques, which transform the values (delta, RLE, dictionary), and physical techniques, which lay the resulting integers out in bytes (varint, bit packing, FastPFOR).
Logical encodings can be cascaded: a dictionary turns strings into codes, and the codes are then delta-coded and bit-packed like any other integers.

Encodings marked <span class="experimental"></span> exist only in [MLT v2](specification/v2.md).

# Plain

No compression is applied to the data.
Depending on the data type, values are stored in the following formats:

- **Integer**: Little-Endian byte order
- **Long**: Little-Endian byte order
- **Float**: IEEE754 floating-point numbers in Little-Endian byte order
- **Double**: IEEE754 floating-point numbers in Little-Endian byte order
- **String**: Length and data streams

# Boolean-RLE

This encoding compresses boolean columns using least-significant bit numbering (bit-endianness).
Refer to the [ORC specification](https://orc.apache.org/specification/ORCv1/#boolean-run-length-encoding) for implementation details.

v1 uses it for `Present` streams.
v2 uses a raw LSB-first bitfield instead, which can be shared between columns.

# Byte-RLE

This encoding compresses byte streams, such as the `GeometryType` stream in Geometry columns.
Refer to the [ORC specification](https://orc.apache.org/specification/ORCv1/#byte-run-length-encoding) for implementation details.

# Integer Encodings

Most data in MLT is stored as integer arrays, making efficient integer compression crucial.

## Logical-Level Techniques

Integers are encoded using delta encoding, run-length encoding (RLE), or a combination of both (delta-RLE).

### Delta Encoding

Computes differences between consecutive elements (e.g., $x_2 - x_1$, $x_3 - x_2$, ...).
Used with [physical-level techniques](#physical-level-techniques) to reduce the number of bits required for storing delta values.

### Run-Length Encoding (RLE)

Refer to [Wikipedia](https://en.wikipedia.org/wiki/Run-length_encoding) for a basic explanation.
For unsigned integers, ZigZag encoding is applied to the values.

The two wire formats lay the runs out differently:

| | Layout | Run count |
|---|---|---|
| **v1** | All run lengths, then all values | `runs` varint in the stream header |
| **v2** | Interleaved `(run_length, value)` pairs | Not stored; scan the payload to `byte_length` |

v1 example, for the input `[5, 5, 5, 3, 3, 3, 3]`:

```
Runs:   [3, 4]        // 3 fives, 4 threes
Values: [5, 3]
Output: [3, 4, 5, 3]  // runs concatenated with values
```

The same input in v2 is `[3, 5, 4, 3]`.
Both formats know the decoded element count.
v1 stores it in the stream header.
v2 takes it from the stream's value count.

### Delta-RLE

Applies delta encoding followed by RLE.
Efficient for ascending sequences like `id` fields.

## Physical-Level Techniques

Null suppression techniques are used to compress integer arrays by reducing the number of bits required to store each integer.

### ZigZag Encoding

Used for encoding signed integers in null suppression techniques.
[ZigZag encoding](https://en.wikipedia.org/wiki/Variable-length_quantity#Zigzag_encoding) uses the least significant bit to represent the sign.

### VarInt Encoding

A byte-aligned null suppression technique that compresses integers using a minimal number of bytes.
For implementation details, refer to [Protobuf](https://protobuf.dev/programming-guides/encoding/#varints).

### Bit Packing <span class="experimental"></span>

Every value is stored with the same bit width, which is the bit width of the largest value.

The payload is `[u8 width][ceil(count * width / 8) bytes]`.
Values are laid LSB-first end to end, so value `i` occupies bits `i * width ..` of the byte run.
`width` is `1`-`32`.
An all-zero or empty stream has `width = 1`.

Bit packing is smaller than varint when the values have similar magnitudes.
A few large values raise the width for every value.
An encoder SHOULD compare the stored size of both before choosing.

### SIMD-FastPFOR

A bit-aligned null suppression technique that compresses integers using a minimal number of bits.
Uses a patched approach to store exceptions (outliers) separately, keeping the overall bit width small.

!!! WARNING
    The two wire formats use different FastPFOR variants and they are **not** interchangeable.

    - **v1**: 256-value blocks, big-endian
    - **v2**: 128-value blocks, little-endian

Refer to:

- <https://arxiv.org/pdf/1209.2137.pdf>
- <https://ayende.com/blog/199524-C/integer-compression-the-fastpfor-code>

**Available implementations**:

- **C++**: <https://github.com/fast-pack/FastPFOR>
- **Java**: <https://github.com/fast-pack/JavaFastPFOR>
- **C#**: <https://github.com/Genbox/CSharpFastPFOR>
- **JS/WebAssembly**: Work in progress (higher implementation complexity)

# Float Encodings

## Plain Floats

IEEE 754 words, little-endian, one per element.
This is the only float encoding v1 has.

## ALP <span class="experimental"></span>

Adaptive Lossless floating-Point compression stores a float column as integers, which are then compressed with the integer encodings above.

Most floats in map data are decimals with few significant digits, such as `12.75` or `0.3`, and are exactly representable as a scaled integer.
ALP finds one decimal scale for the whole column and stores `i = round(v · 10ᵉ / 10ᶠ)` per value.

The parameters are three varints in the stream header:

| Parameter | Meaning |
|---|---|
| `e` | Decimal exponent the values were scaled by, `0`-`18` |
| `f` | Factor dividing out the trailing zeros `e` introduced, never exceeding `e` |
| `base` | Frame of reference: the smallest scaled integer in the column, ZigZag-coded |

The payload holds unsigned offsets from `base`, so the smallest is `0` and every value is non-negative.
The offsets are an ordinary integer stream and carry their own physical encoding.

**Decoding**: `v = (base + offset) · 10ᶠ / 10ᵉ`.

`e` and `f` are stored separately instead of a single `10^(e - f)`.
Scaling by `10ᵉ` and then dividing by `10ᶠ` rounds twice, and some values are only exactly representable with `f > 0`.

!!! NOTE
    This arithmetic is normative and deviates from reference ALP, which multiplies by a rounded reciprocal (`v · EXP_ARR[e] · FRAC_ARR[f]`).
    The two disagree on roughly 0.75% of values.
    MLT ALP streams are not bit-interchangeable with a reference implementation.

    An encoder MUST decode every value back and compare bit patterns.
    If any value does not round-trip exactly, the encoder MUST use another encoding.

`e` is at most `18` so that `v · 10ᵉ` fits in an `i64`.

## Float Dictionary <span class="experimental"></span>

The distinct values are stored once, and a stream of codes holds one index into them per element.
The codes are an integer stream.
The dictionary follows as a second stream of raw values.

# Dictionary Encoding

Dictionary encoding compactly represents repeated values and can be applied to `String` and `Geometry` columns.
Distinct values are stored in a `dictionary` stream, while a separate `data` stream stores indices into the dictionary.

## String Dictionary Encoding

The distinct UTF-8 values are stored once, and a stream of codes holds one index into them per present value.
A dictionary-encoded nullable string column consists of the following streams in order:

| Stream | Holds |
|---|---|
| `Present` | Which features have a value. Omitted when the column is not nullable. |
| `Length` | The byte length of each distinct value |
| `Offset` | One dictionary index per present value |
| `Data` | The distinct values' bytes, back to back |

!!! NOTE
    The stream order differs between versions.
    v2 puts the code stream first.
    v1 puts it between the lengths and the bytes, and last in the FSST dictionary layout.
    See the [v1](specification/v1.md#string-columns) and [v2](specification/v2.md#string-columns) specifications.

The streams are further compressed using the lightweight encodings above:

- **Present**: Boolean-RLE in v1, a raw bitfield in v2
- **Length and Offset**: any [integer encoding](#integer-encodings)
- **Data**: [FSST](#fsst-dictionary-encoding), [front coding](#front-coding), or plain bytes

### FSST Dictionary Encoding

Dictionary encoding requires fully repeating strings to reduce size. However, geospatial attributes often contain strings with common prefixes that are not identical (e.g., localized country names).
FSST replaces frequently occurring substrings while supporting efficient scans and random lookups. It further compresses UTF-8 encoded dictionary values in MLT.
An FSST-compressed string column adds two streams to the layouts above:

| Stream | Holds |
|---|---|
| `SymbolLength` | The byte length of each symbol |
| `SymbolTable` | The symbols themselves |

The `Data` stream then holds the compressed corpus rather than plain bytes.
For implementation details, refer to [this paper](https://www.vldb.org/pvldb/vol13/p2649-boncz.pdf).

**Available implementations**:

- **C++**: <https://github.com/cwida/fsst>
- **Rust**: <https://crates.io/crates/fsst-rs>
- **Java**: Work in progress
- **JS/WebAssembly decoder**: Work in progress (simple to implement)

!!! NOTE
    Different FSST implementations train different symbol tables for the same input.
    A decoder reads the symbol table from the stream, so this only matters when comparing the output of two encoders.

### Front Coding <span class="experimental"></span>

Neighbouring entries of a sorted dictionary often share a prefix, for example `Main Street`, `Main Street North` and `Maple Avenue`.
Front coding stores the length of the prefix shared with the previous entry, and only the suffix bytes.

The lengths stream that precedes the blob holds `2N` values: `N` shared-prefix lengths, then `N` suffix lengths.
Entry `i` is the first `prefix_lengths[i]` bytes of entry `i - 1`, followed by its own suffix.
The first entry's prefix length is always `0`.

Entries are reconstructed sequentially.
Random access requires a scan from the start of the dictionary.
Prefix lengths are in bytes and may split a multi-byte character.
Only the reconstructed entry has to be valid UTF-8.

Front coding can be combined with FSST.
The corpus is front-coded first and then FSST-compressed.
An encoder SHOULD compare the stored size of each combination.

### Shared Dictionary Encoding

Shared dictionary encoding allows multiple columns to share a common dictionary.
Localized name columns such as `name:en`, `name:de` and `name:fr` in an OSM dataset are the main use case.

Columns sharing a dictionary must be grouped together in the file and prefixed with the dictionary.
Each member column then stores only its present stream and its codes:

```
Length, Data, Present₁, Offset₁, Present₂, Offset₂, ...
```

With FSST the dictionary is four streams instead of two:

```
SymbolLength, SymbolTable, Length, Data, Present₁, Offset₁, Present₂, Offset₂, ...
```

In v2 presence bitfields are not streams and can be shared between columns.

## Vertex Dictionary Encoding

Uses an additional `VertexOffsets` stream to store indices for vertex coordinates in the `VertexBuffer` stream.
Vertices in the VertexBuffer are sorted using a Hilbert curve and delta-encoded with null suppression.

### Morton Vertex Dictionary Encoding

`VertexBuffer` coordinates are transformed into a 1D integer using Morton code.
The data is sorted by Morton code and further compressed using integer compression techniques.
The grid the codes are laid on is carried as two parameters, `bits` and `shift`.

v1 can store Morton codes plain, delta-coded or run-length encoded.
v2 stores them only delta-coded over a sorted dictionary.

# Choosing an Encoding

A brute-force search over every combination is too costly.
Use the selection strategy from the [BTRBlocks](https://www.cs.cit.tum.de/fileadmin/w00cfj/dis/papers/btrblocks.pdf) paper:

- Calculate data metrics to exclude unsuitable encodings early (e.g., exclude RLE if the average run length is less than 2).
- Use a sampling-based algorithm: randomly select parts of the data totaling ~1% of the full dataset and apply the candidate encodings from step 1.
  Choose the scheme that produces the smallest output.

Compare stored bytes, not an estimate.
Do not assume a later gzip pass changes which candidate wins.
