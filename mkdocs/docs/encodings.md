<h1>Encoding Definitions</h1>

[TOC]

---

# Plain

No compression is applied to the data.
Depending on the data type, values are stored in the following formats:

- **Boolean**: Least-significant bit ordering within bytes
- **Integer**: Little-Endian byte order
- **Long**: Little-Endian byte order
- **Float**: IEEE754 floating-point numbers in Little-Endian byte order
- **Double**: IEEE754 floating-point numbers in Little-Endian byte order
- **String**: Length and data streams

# Boolean-RLE

This encoding compresses boolean columns using least-significant bit numbering (bit-endianness).
Refer to the [ORC specification](https://orc.apache.org/specification/ORCv1/#boolean-run-length-encoding) for implementation details.

# Byte-RLE

This encoding compresses byte streams, such as the `GeometryType` stream in Geometry columns.
Refer to the [ORC specification](https://orc.apache.org/specification/ORCv1/#byte-run-length-encoding) for implementation details.

# Dictionary Encoding

Dictionary encoding compactly represents repeated values and can be applied to `String` and `Geometry` columns.
Distinct values are stored in a `dictionary` stream, while a separate `data` stream stores indices into the dictionary.
We allow the following encodings of this concept:

## String Dictionary Encoding

The dictionary stream contains distinct UTF-8 encoded string values.
The data stream contains dictionary indices encoded as `UInt32`.
A dictionary-encoded nullable string column consists of the following streams in order:
`Present`, `Length`, `Dictionary`, `Data`

!!! TODO
    Is "can" (⇒ MAY) here the right terminology?
    This implies that streams might be encoded by one of the options below, but also other options.
    If yes, what is the alternative options?
    If no, clarify this misunderstanding.

All streams can be further compressed using lightweight encodings:

- **Present Stream**: Boolean-RLE
- **Length and Data Stream**: See Integer encodings
- **Dictionary**: See FSST Dictionary

### FSST Dictionary Encoding

Dictionary encoding requires fully repeating strings to reduce size. However, geospatial attributes often contain strings with common prefixes that are not identical (e.g., localized country names).
FSST replaces frequently occurring substrings while supporting efficient scans and random lookups. It further compresses UTF-8 encoded dictionary values in MLT.
An FSST dictionary-encoded nullable string column consists of the following streams in order:
`Present`, `SymbolLength`, `SymbolTable`, `String Length`, `Dictionary` (Compressed Corpus)
For implementation details, refer to [this paper](https://www.vldb.org/pvldb/vol13/p2649-boncz.pdf).

**Available implementations**:

- **C++**: <https://github.com/cwida/fsst>
- **Java**: Work in progress
- **JS/WebAssembly decoder**: Work in progress (simple to implement)

### Shared Dictionary Encoding

Shared dictionary encoding allows multiple columns to share a common dictionary.
Nested fields can also use a shared dictionary (e.g., for localized values like `name:*` columns in an OSM dataset).
If used for nested fields, all fields sharing the dictionary must be grouped together in the file and prefixed with the dictionary.

A shared dictionary-encoded nullable string column consists of the following streams in order:
`Length`, `Dictionary`, `Present1`, `Data1`, `Present2`, `Data2`

A shared FSST dictionary-encoded nullable string column consists of the following streams in order:
`SymbolLength`, `SymbolTable`, `String Length`, `Dictionary` (Compressed Corpus), `Present1`, `Data1`, `Present2`, `Data2`

## Vertex Dictionary Encoding

Uses an additional `VertexOffsets` stream to store indices for vertex coordinates in the `VertexBuffer` stream.
Vertices in the VertexBuffer are sorted using a Hilbert curve and delta-encoded with null suppression.

### Morton Vertex Dictionary Encoding

`VertexBuffer` coordinates are transformed into a 1D integer using Morton code.
The data is sorted by Morton code and further compressed using integer compression techniques.

# Integer Encodings

Most data in MLT is stored as integer arrays, making efficient integer compression crucial.

## Logical-Level Techniques

Integers are encoded using delta encoding, run-length encoding (RLE), or a combination of both (delta-RLE).

### Delta Encoding

Computes differences between consecutive elements (e.g., x₂ - x₁, x₃ - x₂, ...).
Used with [physical-level techniques](#physical-level-techniques) to reduce the number of bits required for storing delta values.

### Run-Length Encoding (RLE)

Refer to [Wikipedia](https://en.wikipedia.org/wiki/Run-length_encoding) for a basic explanation.
Runs and values are stored in separate buffers.
For unsigned integers, ZigZag encoding is applied to the values buffer.
Both buffers are further compressed using null suppression.

### Delta-RLE

Applies delta encoding followed by RLE.
Efficient for ascending sequences like `id` fields.

## Physical-Level Techniques
Null suppression techniques are used to compress integer arrays by reducing the number of bits required to store each integer.

### ZigZag Encoding

Used for encoding unsigned integers in null suppression techniques.
[ZigZag encoding](https://en.wikipedia.org/wiki/Variable-length_quantity#Zigzag_encoding) uses the least significant bit to represent the sign.

### VarInt Encoding

A byte-aligned null suppression technique that compresses integers using a minimal number of bytes.
For implementation details, refer to [Protobuf](https://protobuf.dev/programming-guides/encoding/#varints).

### SIMD-FastPFOR

A bit-aligned null suppression technique that compresses integers using a minimal number of bits.
Uses a patched approach to store exceptions (outliers) separately, keeping the overall bit width small.

Refer to:
- <https://arxiv.org/pdf/1209.2137.pdf>
- <https://ayende.com/blog/199524-C/integer-compression-the-fastpfor-code>

**Available implementations**:

- **C++**: <https://github.com/fast-pack/FastPFOR>
- **Java**: <https://github.com/fast-pack/JavaFastPFOR>
- **C#**: <https://github.com/Genbox/CSharpFastPFOR>
- **JS/WebAssembly**: Work in progress (higher implementation complexity)

# Float Encodings

## Adaptive Lossless Floating-Point Compression (ALP)

A lossless floating-point compression technique used in MLT.
For a detailed explanation, refer to [this paper](https://dl.acm.org/doi/pdf/10.1145/3626717).

**Available implementations**:

- **C++**: <https://github.com/duckdb/duckdb/tree/16830ded9b4882b90f35c2d7e2d740547ae7d3fc/src/include/duckdb/storage/compression/alp>
- **Java**: Work in progress
- **JS/WebAssembly**: Work in progress
