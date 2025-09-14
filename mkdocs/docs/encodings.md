# Encoding Definitions

## Plain

No compression technique is applied on the data.
Depending on the data type, the data are stored in the following format:

- Boolean: Least-significant bit ordering within bytes
- Integer: Little-Endian byte order
- Long: Little-Endian byte order
- Float: IEEE754 floating-point numbers in Little-Endian byte order
- Double: IEEE754 floating-point numbers in Little-Endian byte order
- String: length and data streams

## Boolean-RLE

This encoding is used for compressing boolean columns.
Least-significant bit numbering (bit-endianness) is used as bit order.
Have a look at the [ORC specification](https://orc.apache.org/specification/ORCv1/#boolean-run-length-encoding)
for implementation details.

## Byte-RLE

This encoding is used for compressing byte streams such as the `GeometryType` stream of the Geometry column.
Have a look at the [ORC specification](https://orc.apache.org/specification/ORCv1/#byte-run-length-encoding)
for implementation details.

## Dictionary Encoding

Dictionary encoding is used to compactly represent repeated values and can be applied to `String` and `Geometry` columns.
In addition to the actual distinct values stored in the `dictionary` stream, a separate `data` stream for the indices
into the dictionary is used.

### String Dictionary Encoding

The dictionary stream contains distinct UTF-8 encoded string values.
The data stream contains the indices for the dictionary encoded as `UInt32`.
A dictionary-encoded nullable string column contains of the following streams and order: `Present`, `Length`, `Dictionary`, `Data`.

All streams can be further compressed by recursively applying lightweight encodings:

- Present Stream: Boolean-RLE
- Length and Data Stream: see Integer encodings
- Dictionary: see FSST Dictionary

#### FSST Dictionary Encoding

Dictionary encoding needs fully repeating strings to reduce size.
However, the attributes of geospatial data often contains strings which share a common prefix but are not completely equal
such as the localized country names.
FSST replaces frequently occurring substrings while maintaining support for efficient scans and random look-ups.
It is used in MLT to further compress the UTF-8 encoded dictionary values.
A FSST Dictionary-encoded nullable string column contains of the following streams and order:
`Present`, `SymbolLength`, `SymbolTable`, `String Length`, `Dictionary` (Compressed Corpus)
For implementation details see [the following paper](https://www.vldb.org/pvldb/vol13/p2649-boncz.pdf)

Available implementations:

- C++: [https://github.com/cwida/fsst](https://github.com/cwida/fsst)
- Java: Work in Progress
- Js/WebAssembly decoder: Work in Progress (simple to implement)

#### Shared Dictionary Encoding

A Shared dictionary encoding can be used, to share a common dictionary between the columns.
In addition, nested fields can use a shared dictionary encoding, to share a common dictionary between the fields.
This can be applied for example on localized values of the name:* columns of an OSM dataset that can be identical across fields.
If a shared dictionary encoding is used for nested fields, all fields that use the shared dictionary
must be grouped one behind the other in the file and prefixed with the dictionary.

Shared dictionary-encoded nullable string columns consists of the following streams and order:
`Length`, `Dictionary`, `Present1`, `Data1`, `Present2`, `Data2`

Shared FSST Dictionary-encoded nullable string columns consists of the following streams and order:
`SymbolLength`, `SymbolTable`, `String Length`, `Dictionary` (Compressed Corpus), `Present1`, `Data1`, `Present2`, `Data2`

### Vertex Dictionary Encoding

Uses an additional `VertexOffsets` stream for storing the indices to the coordinates of the vertices
in the `VertexBuffer` stream. The vertices in the VertexBuffer are sorted on a Hilbert-Curve
and delta-encoded in combination with a null suppression technique.

#### Morton Vertex Dictionary Encoding

The coordinates of the `VertexBuffer` are transformed to a 1D integer using the Morton Code.
Then the data are sorted according to the Morton code and further compressed by applying
an Integer compression technique.

## Integer Encodings

In general most data in MLT are stored in the form of arrays of integers.
Using efficient and fast algorithms for compressing arrays of integers is therefore crucial.

### Logical Level Technique

Integers are encoded based on the two lightweight compression techniques delta and rle as well
as a combination of both schemes called delta-rle.

#### Delta

Compute the differences between consecutive elements of a sequence e.g. x2 - x1, x3 - x2, ...
Is always used in combination with a physical level technique to reduce the number of bits needed to store the
delta values.

#### RLE

See [https://en.wikipedia.org/wiki/Run-length_encoding](https://en.wikipedia.org/wiki/Run-length_encoding) for a basic explanation
All runs and values are stored in two separate buffers.
If the values are unsigned integers ZigZag encoding is applied to the values buffer.
Both buffers are then further compressed using a null suppression technique.

#### Combining delta and rle (Delta-RLE)

Delta-compress the values and then apply RLE encoding.
This technique is efficient for ascending sequences like `id`s.

### Physical Level Technique (Null Suppression)

####  ZigZag encoding

For encoding unsigned integers, ZigZag encoding is used for both of the following null suppression techniques.
[ZigZag](https://en.wikipedia.org/wiki/Variable-length_quantity#Zigzag_encoding) encoding uses the least significant bit for encoding the sign.

#### Varint encoding

Byte-aligned null suppression technique try to compress an integer using a minimal number of bytes.
For implementation details see [Protobuf](https://protobuf.dev/programming-guides/encoding/#varints).

#### SIMD-FastPFOR

Bit-aligned null suppression technique that compresses an integer using a minimal number of bits
by using a patched approach to store exceptions (outliers) in a separate section to keep the overall bit width small.

see [https://arxiv.org/pdf/1209.2137.pdf](https://arxiv.org/pdf/1209.2137.pdf) and [https://ayende.com/blog/199524-C/integer-compression-the-fastpfor-code](https://ayende.com/blog/199524-C/integer-compression-the-fastpfor-code)

Available implementations:

- C++: [https://github.com/lemire/FastPFor](https://github.com/lemire/FastPFor)
- Java: [https://github.com/lemire/JavaFastPFOR](https://github.com/lemire/JavaFastPFOR)
- C#: [https://github.com/Genbox/CSharpFastPFOR](https://github.com/Genbox/CSharpFastPFOR)
- Js/WebAssembly: Work in Progress (higher implementation complexity)

## Float encodings

### Adaptive Lossless floating-Point Compression (ALP)

See [https://dl.acm.org/doi/pdf/10.1145/3626717](https://dl.acm.org/doi/pdf/10.1145/3626717) for a detailed explanation.
ALP is a lossless floating-point compression technique that is used to compress the float values in MLT.

Available implementations:

- C++: [https://github.com/duckdb/duckdb/tree/16830ded9b4882b90f35c2d7e2d740547ae7d3fc/src/include/duckdb/storage/compression/alp](https://github.com/duckdb/duckdb/tree/16830ded9b4882b90f35c2d7e2d740547ae7d3fc/src/include/duckdb/storage/compression/alp)
- Java: Work in Progress
- JS/WebAssembly: Work in Progress
