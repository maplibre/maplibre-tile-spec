<h1>Encoding Definitions</h1>

---

This page defines the payload of every encoding an MLT stream can name.
Which streams may use which encoding, how the choice is written in the stream header, and where each stream sits are the business of the [v1](specification/v1.md) and [v2](specification/v2.md) specifications.
Those pages define the headers.
This page defines what the bytes after the header mean.

!!! warning "Experimental encodings"
    Encodings marked <span class="experimental"></span> exist only in [MLT v2](specification/v2.md).  
    Streams are called by their [MLT v2](specification/v2.md#string-columns) names throughout.

## Integer Words

Most of MLT is integers.
Lengths, offsets, ids, dictionary codes, geometry topology and the vertex buffer are all streams of integer words, and so are the scaled integers of an [ALP](#alp) float column.

An integer stream is decoded in two steps.
The **physical** encoding turns the payload bytes into a sequence of unsigned words.
The **logical** encoding turns those words into the values.
A stream of `Int64`, `UInt64` or `LongId` values has 64-bit words, and so does an [ALP](#alp) offset stream.
Every other integer stream has 32-bit words.

```
values = logical_decode(physical_decode(payload))
```

A difference is signed even when the values are not, so every delta is [ZigZag](#zigzag)-coded.
A plain or run-length coded value is ZigZag-coded only on a stream of a signed type.
Run lengths, dictionary codes, lengths and offsets are never ZigZag-coded.
Delta arithmetic wraps at the word width.

## Physical Encodings

### ZigZag

Used wherever a signed value has to become an unsigned word.
It is not an encoding a stream names; the logical encodings below say where it applies.

The sign is moved to the least significant bit, so that small negative numbers become small positive numbers:

```
zigzag(n)   = (n << 1) ^ (n >> 31)        // arithmetic shift, 32-bit
unzigzag(u) = (u >> 1) ^ -(u & 1)         // logical shift
```

For 64-bit values the shift is `63`.

| `n` | `zigzag(n)` |
|----:|------------:|
| `0` | `0` |
| `-1` | `1` |
| `1` | `2` |
| `-2` | `3` |
| `2` | `4` |

### None

Each word is stored as it is, little-endian, 4 or 8 bytes per word.

In v2 a stream whose logical encoding is also `None` may leave out `byte_length`, since the value count and the word width give it.
See [Byte Length](specification/v2.md#byte-length).

### VarInt

Each word is stored in 7-bit groups, least significant group first.
Bit 7 of each byte is set when another byte follows.
A 32-bit word takes 1 to 5 bytes and a 64-bit word 1 to 10.

```
300 = 0b1_0010_1100
    -> groups (LSB first): 0101100, 0000010
    -> bytes:              0xAC,    0x02
```

This is the unsigned varint of [Protocol Buffers](https://protobuf.dev/programming-guides/encoding/#varints) and the length prefix every MLT header uses.
Signed values go through [ZigZag](#zigzag) first, where the logical encoding says so.

### Bit Packing <span class="experimental"></span>

Every word is stored in the same number of bits, the bit width of the largest value.

v2 numbers bit packing as a logical encoding, and its physical field is reserved as `0`.
It stands in for the whole physical step: the words come straight out of the packed bits, and no other logical transform is applied.

```
payload := [u8 width]                        1-32
           [u8 packed[ceil(count * width / 8)]]
```

Words are laid LSB-first end to end, so word `i` occupies bits `i * width` to `i * width + width - 1` of the byte run.
Bits past the last word in the final byte are padding.
Encoders write them as `0` and decoders MUST ignore them.
`width` MUST be `1` to `32`.
An empty or all-zero stream has `width = 1`.
A payload whose length is not exactly `1 + ceil(count * width / 8)` MUST be rejected.

```
values: [5, 1, 7, 0]           width = 3
bits:   101 001 111 000        value 0 = bits 0-2, value 1 = bits 3-5, ...
bytes:  0b11_001_101 = 0xCD    bits 0-7:  5, 1, and the low 2 bits of 7
        0b0000_00_01 = 0x01    bits 8-15: the high bit of 7, 0, padding
payload: 03 CD 01
```

Bit packing beats varint when the values are of similar magnitude, since varint spends at least 8 bits per value.
One large value raises the width for every value.
An encoder SHOULD compare the stored size of both.

### FastPFOR

A block codec that stores each block of words in the bit width most of them need, and patches the few words that need more as exceptions in a separate area.
Unlike bit packing it is not sensitive to a handful of outliers.

The payload is the output of the composite codec `Composition(FastPFOR, VariableByte)` of the [FastPFOR library](https://github.com/fast-pack/FastPFOR), stored as whole 32-bit words:

```
payload := [u32 n]                     number of FastPFOR words that follow
           [u32 fastpfor[n]]           whole blocks, in the FastPFOR block format
           [u32 vbyte[...]]            the values that did not fill a block, variable-byte coded
```

The block format is specified by [Lemire and Boytsov, *Decoding billions of integers per second through vectorization*](https://arxiv.org/pdf/1209.2137.pdf), section 6.
A payload whose length is not a multiple of 4 MUST be rejected.
A stream of zero values has an empty payload.

The two tile versions use different variants, and the words of one cannot be read as the other:

| | Block size | Word byte order |
|---|---|---|
| **v1** | 256 values | big-endian |
| **v2** | 128 values | little-endian |

A decoder selects the variant by the layer tag.

FastPFOR only produces 32-bit words.
A 64-bit integer column cannot use it.
An [ALP](#alp) offset stream can, and then has 32-bit words.

## Logical Encodings

The logical encoding is applied to the words the physical step produced.

### None

The words are the values.
On a signed stream each word is [ZigZag](#zigzag)-decoded.

### Delta

Each word is the difference to the previous value.
The first value's predecessor is `0`.
Each difference is [ZigZag](#zigzag)-coded before it becomes a word, whatever the stream's type.

```
delta[0] = values[0] - 0
delta[i] = values[i] - values[i - 1]

decode: values[i] = values[i - 1] + delta[i]
```

Example:

```
values: [100, 105, 102]
deltas: [100, 5, -3]
words:  [200, 10, 5]           zigzag
```

Delta suits monotonic sequences such as ids and offsets, whose differences are small however large the values are.

### RLE

The values are stored as runs, each a `(run_length, value)` pair that expands to `run_length` copies of `value`.
On a signed stream `value` is [ZigZag](#zigzag)-coded.
Run lengths never are.

The two tile versions lay the runs out differently:

| | Payload | Run count |
|---|---|---|
| **v1** | All run lengths, then all values, physically coded as one word sequence | `runs` varint in the stream header |
| **v2** | Interleaved `(run_length, value)` pairs as varints, no physical field | Not stored; read pairs until `byte_length` is exhausted |

For the input `[5, 5, 5, 3, 3, 3, 3]`:

```
runs:   [3, 4]
values: [5, 3]

v1 words: [3, 4, 5, 3]         header: runs = 2, num_rle_values = 7
v2 bytes: 03 05 04 03          header: 7 values from context
```

In both versions the decoded element count is known before the payload is read.
v1 stores it in the header as `num_rle_values`.
v2 takes it from the stream's value count.
Run lengths that do not sum to exactly that count MUST be rejected.
A v2 payload with an odd number of varints MUST be rejected.

### Delta-RLE

[Delta](#delta) followed by [RLE](#rle).
The deltas are ZigZag-coded, and it is those unsigned words that are run-length coded.
Decoding undoes them in reverse: expand the runs, then undo ZigZag and prefix-sum.

```
values: [10, 11, 12, 13, 20, 20, 20]
deltas: [10, 1, 1, 1, 7, 0, 0]
words:  [20, 2, 2, 2, 14, 0, 0]          zigzag
runs:   (1, 20) (3, 2) (1, 14) (2, 0)
```

Delta-RLE suits sequences with a constant step, such as `[1, 2, 3, ...]`, which become one run.

## Boolean Streams

Boolean columns and v1 `Present` streams hold one bit per value.

### Bitmap

Bit `i % 8` of byte `i / 8` is value `i`, LSB-first.
The bitmap is `ceil(count / 8)` bytes.
Bits past `count` in the final byte are padding and MUST be ignored.

```
values: [1, 0, 1, 1, 0, 1]
byte:   0b00_101101 = 0x2D
```

v2 stores every presence bitfield and boolean column as a raw bitmap.
A bitmap can be shared between columns; see [Shared Presence Bitfields](specification/v2.md#shared-presence-bitfields).

### Boolean RLE

v1 compresses the [bitmap](#bitmap) with the byte-level run-length encoding of [ORC](https://orc.apache.org/specification/ORCv1/#byte-run-length-encoding).
The payload is a sequence of runs, each a control byte and what it names:

| Control byte `c` | Meaning |
|---|---|
| `0`-`127` | A repeated run: the next byte, `c + 3` times |
| `128`-`255` | A literal run: the next `256 - c` bytes as they are |

A repeated run is 3 to 130 bytes and a literal run 1 to 128.
The runs expand to exactly `ceil(count / 8)` bitmap bytes.
A payload that expands to more, or ends before that, MUST be rejected.

```
bitmap:  FF FF FF FF FF 2D
payload: 02 FF FF 2D
         02 FF          repeated run: 2 + 3 = 5 copies of FF
         FF 2D          literal run: 256 - 255 = 1 byte, 2D
```

The v1 stream header carries no `runs` or `num_rle_values` for a boolean stream.
Both follow from `num_values`.

## Float Streams

### Plain Floats

IEEE 754 words, little-endian, 4 bytes for a `Float` and 8 for a `Double`.
This is the only float encoding v1 has.

### ALP <span class="experimental"></span>

Adaptive Lossless floating-Point compression stores a float column as integers, which are then [physically encoded](#physical-encodings) like any other integer stream.

Most floats in map data are decimals with few significant digits, such as `12.75` or `0.3`, and are exactly representable as a scaled integer.
ALP finds one decimal scale for the whole column and stores $i = \operatorname{round}(v \cdot 10^e / 10^f)$ per value.

The parameters are three varints in the stream header:

| Parameter | Meaning |
|---|---|
| `e` | Decimal exponent the values were scaled by, `0`-`18` |
| `f` | Factor dividing out the trailing zeros `e` introduced, never exceeding `e` |
| `base` | Frame of reference: the smallest scaled integer in the column, ZigZag-coded |

`e > 18` or `f > e` MUST be rejected.

The payload holds unsigned offsets from `base`, so the smallest is `0` and every value is non-negative.
The offsets are an ordinary unsigned integer stream and carry their own physical encoding.
Its words are 64-bit, except under FastPFOR, which only has 32-bit words.

**Decoding**: $i = \mathit{base} + \mathit{offset}$, in 64-bit integer arithmetic, then $v = i \cdot 10^f / 10^e$.

The sum MUST be formed as an integer before the conversion.
A column spanning $[-2, 2^{53} - 1]$ has an offset of $2^{53} + 1$, which a double cannot hold.

`e` and `f` are stored separately instead of a single `10^(e - f)`.
Scaling by $10^e$ and then dividing by $10^f$ rounds twice, and some values are only exactly representable with $f > 0$.

!!! NOTE
    This arithmetic is normative and deviates from reference ALP, which multiplies by a rounded reciprocal (`v * EXP_ARR[e] * FRAC_ARR[f]`).
    The two disagree on roughly 0.75% of values.
    MLT ALP streams are not bit-interchangeable with a reference implementation.

    An encoder MUST decode every value back and compare bit patterns.
    If any value does not round-trip exactly, the encoder MUST use another encoding.

`e` is at most `18` so that $v \cdot 10^e$ fits in an `i64`.

!!! NOTE
    The reference Rust encoder currently only emits scaled integers with $|i| \le 2^{53} - 1$, where consecutive doubles are at most $1$ apart.
    Beyond that, floating-point scaling can land on a neighbouring integer that still passes its own round-trip check.
    This is a limitation of that implementation, not of the format.

```
values:  [-0.75, 0.25, 1.5, -2.25]
e = 2, f = 0:  i = [-75, 25, 150, -225]
base = -225:   offsets = [150, 250, 375, 0]
```

The header stores `e = 2`, `f = 0` and `base` as the ZigZag varint `c1 03`, and the payload the four offsets as varints.
See the [ALP example](specification/v2.md#examples) on the v2 page for the whole layer.

### Float Dictionary <span class="experimental"></span>

The distinct values are stored once, and a stream of codes holds one index into them per element.

The column has two streams.
The first is the codes: an integer stream of 32-bit words, logical `Dict` in the `Float` family, with its own physical encoding.
The second is the dictionary: [plain floats](#plain-floats), one per distinct value, with an explicit count in its header.

```
values: [1.5, 0.25, 1.5, 1.5, 0.25]
codes:  [0, 1, 0, 0, 1]
dict:   [1.5, 0.25]
```

Entries are distinct by bit pattern.
`-0.0` and `0.0` are two entries, and a `NaN` is never equal to any entry.
A code at or past the dictionary's count MUST be rejected.

## Byte Blobs

String values, dictionary values and FSST symbol tables are byte blobs.
A blob's count is its `byte_length`, and a lengths stream beside it says where each value ends.

### Plain Bytes

The bytes as they are.
Value `i` is the `lengths[i]` bytes that follow the first `lengths[0] + ... + lengths[i - 1]`.
Each value MUST be valid UTF-8.

### Front Coding <span class="experimental"></span>

Neighbouring entries of a sorted dictionary often share a prefix, for example `Main Street`, `Main Street North` and `Maple Avenue`.
Front coding stores the length of the prefix shared with the previous entry, and only the suffix bytes.

The lengths stream that precedes the blob holds `2N` values: `N` shared-prefix lengths, then `N` suffix lengths.
The blob holds the `N` suffixes back to back.

```
entry[i] = entry[i - 1][..prefix_lengths[i]] ++ suffix[i]
```

The first entry's prefix length is always `0`.
A prefix length longer than the previous entry, a lengths stream with an odd count, or suffix bytes left over after the last entry MUST be rejected.

Entries are reconstructed sequentially.
Random access requires a scan from the start of the dictionary.
Prefix lengths are in bytes and may split a multi-byte character.
Only the reconstructed entry has to be valid UTF-8.

```
entries:  ["Main Street", "Main Street North", "Maple Avenue"]
prefixes: [0, 11, 2]
suffixes: [11, 6, 10]
lengths:  [0, 11, 2, 11, 6, 10]
blob:     "Main Street" " North" "ple Avenue"
```

Front coding can be combined with [FSST](#fsst).
The corpus is front-coded first and then FSST-compressed.
An encoder SHOULD compare the stored size of each combination.

### FSST

Fast Static Symbol Table compression replaces frequent byte sequences of up to 8 bytes with one-byte codes.
Unlike a dictionary it compresses strings that merely share substrings, such as localized country names.
It supports random access to one value once its lengths are known, since every code is one byte.

An FSST-compressed blob comes with two streams that carry its symbol table:

| Stream | Holds |
|---|---|
| `SymbolLengths` | The byte length of each symbol, an integer stream |
| `SymbolTable` | The symbols, back to back, a plain byte blob |

There are at most `255` symbols, numbered `0` to `254` in the order the two streams list them.

The `Corpus` blob is a sequence of codes:

| Byte `b` | Meaning |
|---|---|
| `0`-`254` | Symbol `b`, expanded to its bytes |
| `255` | Escape: the next byte is output as it is |

A code naming a symbol the table does not have, or a corpus ending on an escape byte, MUST be rejected.

The corpus compresses all values as one buffer, so a symbol may span two values.
The lengths stream beside the corpus holds the **uncompressed** length of each value.
A decoder expands the whole corpus and then splits it by those lengths.

```
symbols:      ["ab", "cd"]           SymbolLengths = [2, 2], SymbolTable = "abcd"
values:       ["abcd", "abz"]        Lengths = [4, 3]
corpus bytes: 00 01 00 FF 7A         ab cd ab <esc> z
```

The algorithm that trains the table is described by [Boncz, Neumann and Leis, *FSST: Fast Random Access String Compression*](https://www.vldb.org/pvldb/vol13/p2649-boncz.pdf).
Different implementations train different tables for the same input.
A decoder reads the table from the stream, so that only matters when comparing the output of two encoders.

## String Layouts

A string column is a set of the streams above.
The four layouts, in the v2 names:

| Layout | Streams, in order |
|---|---|
| Plain | `Lengths`, `Values` |
| Dict | `Codes`, `DictLengths`, `DictValues` |
| FSST | `Lengths`, `SymbolLengths`, `SymbolTable`, `Corpus` |
| FsstDict | `Codes`, `DictLengths`, `SymbolLengths`, `SymbolTable`, `Corpus` |

`Lengths` and `Codes` hold one value per present value.
The other streams describe the distinct values.
A dictionary is what makes `Codes` an integer stream that any [logical](#logical-encodings) and [physical](#physical-encodings) encoding can compress.

The stream order and the way the layout is announced differ between versions.
v1 counts the streams and puts `Codes` after the dictionary lengths, or last with FSST.
v2 puts `Codes` first and names the layout in the extension bits of that stream.
See [v1 string columns](specification/v1.md#string-columns) and [v2 string columns](specification/v2.md#string-columns).

### Shared Dictionary

Several string columns, such as `name:en`, `name:de` and `name:fr`, index into one dictionary.
The dictionary streams are written once, and each member column then stores only its presence and its `Codes`.

```
DictLengths, DictValues, Present_1, Codes_1, Present_2, Codes_2, ...
```

With FSST the dictionary is four streams instead of two:

```
DictLengths, SymbolLengths, SymbolTable, Corpus, Present_1, Codes_1, Present_2, Codes_2, ...
```

In v2 presence bitfields are not streams, and a member may reference a bitfield shared with any other column.
See [v1](specification/v1.md#shared-dictionary-columns) and [v2](specification/v2.md#shared-dictionary-columns) for the group header each version writes.

## Vertex Streams

The vertex buffer holds $x$ and $y$ interleaved: $[x_0, y_0, x_1, y_1, \ldots]$.
Its words are 32-bit and signed.

### Componentwise Delta

Each coordinate is a delta to the same coordinate of the previous vertex.
`x` and `y` keep separate predecessors, both starting at `0`.
Each delta is [ZigZag](#zigzag)-coded.

```
dx[i] = x[i] - x[i - 1]        x[-1] = 0
dy[i] = y[i] - y[i - 1]        y[-1] = 0
words = [zigzag(dx[0]), zigzag(dy[0]), zigzag(dx[1]), zigzag(dy[1]), ...]
```

```
vertices: (100, 200), (105, 210), (102, 215)
deltas:   (100, 200), (5, 10), (-3, 5)
words:    [200, 400, 10, 20, 5, 10]
```

The v2 `Vertex` family also has a plain `Delta`, which is the integer [Delta](#delta) over the flat word sequence and does not separate the components.

### Vertex Dictionary

The distinct vertices are stored once in a `VertexDict` stream, and a `VertexOffsets` stream holds one index into them per vertex.
`VertexOffsets` is an ordinary unsigned integer stream.
`VertexDict` is a vertex stream and carries any of the encodings in this section.

```
VertexOffsets: [0, 1, 2, 1, 0, 2]
VertexDict:    [(0,0), (10,10), (20,20)]
vertices:      (0,0), (10,10), (20,20), (10,10), (0,0), (20,20)
```

An offset at or past the dictionary's vertex count MUST be rejected.

#### Hilbert Order

Encoders SHOULD sort the dictionary along a Hilbert curve, so that the deltas between neighbouring entries stay short.
The order is not part of the format.
A decoder resolves vertices through `VertexOffsets` and never depends on it.

The reference encoders use the curve of the [`hilbert_2d`](https://crates.io/crates/hilbert_2d) crate's `Hilbert` variant on a $2^{\mathit{bits}} \times 2^{\mathit{bits}}$ grid.
`shift` and `bits` are derived as for [Morton](#morton) below, and each shifted coordinate is masked to 16 bits before it is placed on the grid.
Two vertices with the same curve key are one dictionary entry.

### Morton

A Morton, or Z-order, code interleaves the bits of a coordinate pair into one integer.
Nearby vertices get nearby codes, so a sorted Morton dictionary has small deltas.
Only a `VertexDict` stream uses it.

The parameters are two varints in the stream header:

| Parameter | Meaning |
|---|---|
| `bits` | Bits per axis, at most `16` |
| `shift` | Added to `x` and to `y` before interleaving, so that both are non-negative |

`bits > 16` MUST be rejected.

Encoders derive both from the whole layer's vertices.
`shift` is `-min` when the smallest coordinate `min` on either axis is negative, else `0`.
`bits` is the bit width of `max + shift`, the largest shifted coordinate.

Encoding is two steps.
Shift both coordinates, then interleave them: bit `i` of `sx` goes to bit `2i` of the code, and bit `i` of `sy` to bit `2i + 1`.

```rust
fn encode(x: i32, y: i32, bits: u32, shift: u32) -> u32 {
    let sx = (i64::from(x) + i64::from(shift)) as u32; // MUST be in 0..2^bits
    let sy = (i64::from(y) + i64::from(shift)) as u32;
    let mut code = 0;
    for i in 0..bits {
        code |= ((sx >> i) & 1) << (2 * i);
        code |= ((sy >> i) & 1) << (2 * i + 1);
    }
    code
}
```

Decoding walks the same bits back and undoes the shift.

```rust
fn decode(code: u32, bits: u32, shift: u32) -> (i32, i32) {
    let mut x = 0;
    let mut y = 0;
    for i in 0..bits {
        let mask = 1 << (2 * i);
        x |= (code & mask) >> i;
        y |= ((code >> 1) & mask) >> i;
    }
    (x.wrapping_sub(shift) as i32, y.wrapping_sub(shift) as i32)
}
```

```
(x, y) = (5, 3), shift = 0, bits = 3
sx = 0b101, sy = 0b011
code bits, from bit 0: x0 y0 x1 y1 x2 y2 = 1 1 0 1 1 0
code = 0b011011 = 27
```

The loops are the definition.
The reference Rust codec spreads all 16 bits of an axis at once with masks, which gives the same code for any `bits` up to `16`:

```rust
/// Bit `i` of `v` lands on bit `2i`.
fn spread(mut v: u32) -> u32 {
    v &= 0xFFFF;
    v = (v | (v << 8)) & 0x00FF_00FF;
    v = (v | (v << 4)) & 0x0F0F_0F0F;
    v = (v | (v << 2)) & 0x3333_3333;
    v = (v | (v << 1)) & 0x5555_5555;
    v
}

/// Bit `2i` of `v` lands on bit `i`.
fn compact(mut v: u32) -> u32 {
    v &= 0x5555_5555;
    v = (v | (v >> 1)) & 0x3333_3333;
    v = (v | (v >> 2)) & 0x0F0F_0F0F;
    v = (v | (v >> 4)) & 0x00FF_00FF;
    v = (v | (v >> 8)) & 0x0000_FFFF;
    v
}

let code = spread(sx) | (spread(sy) << 1);
let sx = compact(code);
let sy = compact(code >> 1);
```

The codes are stored as an integer stream of unsigned 32-bit words.
The variants:

| Encoding | Payload | Versions |
|---|---|---|
| `Morton` | The codes themselves | v1 |
| `MortonDelta` | The first code, then each code's difference to the previous one | v1, v2 |
| `MortonRle` | Reserved in v1. The reference Rust decoder rejects it | v1 |

The deltas are plain differences, not ZigZag-coded.
The dictionary is sorted ascending by code, so every difference is non-negative.
v2 only has `MortonDelta` over a sorted dictionary, and names it `Morton` in the `Vertex` family.

```
dict codes: [27, 30, 45]
words:      [27, 3, 15]
```

## Choosing an Encoding

A brute-force search over every combination is too costly.
Use the selection strategy from the [BTRBlocks](https://www.cs.cit.tum.de/fileadmin/w00cfj/dis/papers/btrblocks.pdf) paper:

- Calculate data metrics to exclude unsuitable encodings early (e.g., exclude RLE if the average run length is less than 2).
- Use a sampling-based algorithm: randomly select parts of the data totaling ~1% of the full dataset and apply the candidate encodings from step 1.
  Choose the scheme that produces the smallest output.

Compare stored bytes, not an estimate.
Do not assume a later gzip pass changes which candidate wins.
