---
description: Payload of every encoding a stream can name
---

<h1>Encoding Definitions</h1>

---

This page defines the payload of every encoding an MLT stream can name.
Which streams may use which encoding, how the choice is written in the stream header, and where each stream sits are the business of the [v1](specification/v1.md) and [v2](specification/v2.md) specifications.
Those pages define the headers.
This page defines what the bytes after the header mean.

!!! warning "Experimental encodings"
    Encodings marked <span class="experimental"></span> exist only in [MLT v2](specification/v2.md).
    Streams are referred to by their [MLT v2](specification/v2.md#string-columns) names throughout.

## Integer Words

Most of MLT is integers.
Lengths, offsets, ids, dictionary codes, geometry topology and the vertex buffer are all streams of integer words, and so are the scaled integers of a [framed, exception-free ALP](#alp) float column.

An integer stream is decoded in two steps.
The **physical** encoding turns the payload bytes into a sequence of unsigned words.
The **logical** encoding turns those words into the values.
A stream of `Int64`, `UInt64` or `LongId` values has 64-bit words, and so does a [framed, exception-free ALP](#alp) offset stream.
Every other integer stream has 32-bit words.

```
values = logical_decode(physical_decode(payload))
```

A difference is signed even when the values are not, so every delta is [zigzag](#zigzag)-encoded.
A plain or run-length-encoded value is ZigZag-encoded only on a stream of a signed type.
Run lengths, dictionary codes, lengths and offsets are never ZigZag-encoded.
Delta arithmetic wraps at the word width.

The steps each logical encoding takes:

--8<-- "diagrams/integer-decode.svg"

## Physical Encodings

### ZigZag {#zigzag}

Used wherever a signed value has to become an unsigned word.
It is not an encoding a stream names; the logical encodings below say where it applies.

The sign is moved to the least significant bit, so that small negative numbers become small positive numbers:

```
zigzag(n)   = (n << 1) ^ (n >> 31)        // arithmetic shift, 32-bit
unzigzag(u) = (u >> 1) ^ -(u & 1)         // logical shift
```

For 64-bit values the shift is `63`.

=== "Mapping"

    --8<-- "diagrams/zigzag.svg"

=== "Values"

    | `n` | `zigzag(n)` |
    |----:|------------:|
    | `0` | `0` |
    | `-1` | `1` |
    | `1` | `2` |
    | `-2` | `3` |
    | `2` | `4` |

### None {#physical-none}

Each word is stored as it is, little-endian, 4 or 8 bytes per word.

In v2 a stream whose logical encoding is also `None` may leave out `byte_length`, since the value count and the word width determine it.
See [byte length](specification/v2.md#byte-length).

### VarInt {#varint}

Each word is stored in 7-bit groups, least significant group first.
Bit 7 of each byte is set when another byte follows.
A 32-bit word takes 1 to 5 bytes and a 64-bit word 1 to 10.

[View example](inspector/app/?fixture=0x02%2Fprops_u32_np.mlt&at=column%5B0%5D){target=_blank .inspector-example} - four `9000`s, two bytes each, the first byte of each with bit 7 set.

=== "Bits"

    --8<-- "diagrams/varint.svg"

=== "Values"

    ```
    300 = 0b0000_0001_0010_1100
        -> groups (LSB first): 0101100, 0000010
        -> bytes:              0xAC,    0x02
    ```

This is the unsigned varint of [Protocol Buffers](https://protobuf.dev/programming-guides/encoding/#varints) and the length prefix every MLT header uses.
Signed values go through [zigzag](#zigzag) first, where the logical encoding says so.

### Bit Packing <span class="experimental"></span> {#bit-packing}

Every word is stored in the same number of bits, the bit width of the largest value.

[View example](inspector/app/?fixture=0x02%2Fprops_str_dict_bp_np.mlt&at=codes){target=_blank .inspector-example} - dictionary codes `0` to `2`, each in two bits.

It stands in for the whole physical step: the words come straight out of the packed bits, and only [zigzag](#zigzag) on a signed stream is applied to them.

```
payload := [u8 width]                        1-32
           [u8 packed[ceil(count * width / 8)]]
```

Words are laid out LSB-first, end to end, so word `i` occupies bits `i * width` to `i * width + width - 1` of the byte run.
Bits past the last word in the final byte are padding.
Encoders write them as `0` and decoders MUST ignore them.
`width` MUST be `1` to `32`.
An empty or all-zero stream has `width = 1`.
A payload whose length is not exactly `1 + ceil(count * width / 8)` MUST be rejected.

=== "Bits"

    --8<-- "diagrams/bit-packing.svg"

=== "Values"

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

### FastPFOR {#fastpfor}

A block codec that stores each block of words at the bit width most of them need, and patches the few words that need more from separate exception arrays.
Unlike [bit packing](#bit-packing), it is not sensitive to a handful of outliers.
It is described by [Lemire and Boytsov, *Decoding billions of integers per second through vectorization*](https://arxiv.org/pdf/1209.2137.pdf).

[View example](inspector/app/?fixture=0x02%2Fids_fpf.mlt&at=column%5B0%5D){target=_blank .inspector-example} - an id stream in the v2 `128le` variant.

v1 and v2 use different variants:

| | v1 | v2 |
|---|---|---|
| Block size | 256 values | 128 values |
| Packing | [Sequential](#fastpfor-packing), 32 values at a time | [Interleaved](#fastpfor-packing), 128 values at a time |
| Word byte order | big-endian | little-endian |
| Value width | 32 bits | 32 and 64 bits |

The payload consists of whole 32-bit words, whatever the width of the values:

```
payload := [u32 block_values]          values in whole blocks, a multiple of 128
           [page; ceil(block_values / 65536)]
           [u8 tail[...]]              the remaining values, variable-byte encoded
```

`block_values` is the value count rounded down to a multiple of `128`.
The values after the last whole block, at most `127`, are the [tail](#fastpfor-tail).
A stream of fewer than `128` values has `block_values = 0`, no page, and only a tail.
A stream of zero values has an empty payload.
A payload whose length is not a multiple of 4 MUST be rejected.

--8<-- "diagrams/fastpfor-layout.svg"

#### Pages

A page holds up to `65536` values, which is `512` blocks.
Every page but the last is full.
Exceptions are collected per page, so that the blocks of a page share one set of exception arrays.

```
page := [u32 meta_offset]              words from this word to meta_size, 1 + the packed words
        [packed block; blocks]         4 * width words each
        [u32 meta_size]                bytes of block metadata
        [u8 meta[meta_size]]           zero-padded to whole words
        [bitmap]                       1 word for u32 values, 2 words for u64, low word first
        [exception array; popcount(bitmap)]
```

The metadata is one entry per block, in block order:

```
block_meta := [u8 width]               0-32 for u32, 0-64 for u64
              [u8 exceptions]          0-127
              if exceptions > 0:
                [u8 max_width]         bit width of the widest value in the block
                [u8 position; exceptions]
```

`width` is the bit width the block is packed at.
A value that needs more is an **exception**: its low `width` bits stay in the block, and its high bits go to an exception array.
`position` is the index of the exception in the block, `0` to `127`, and encoders write the positions in ascending order.

An exception array can exist for each width `k` from `2` to the value width, and bit `k - 1` of the bitmap says whether it does.
Array `k` holds the high bits of every exception in the page with `max_width - width = k`, in block order and then position order.

```
exception_array := [u32 count]
                   [count values at k bits each]
```

The values of an array are packed in groups of 128 [interleaved](#fastpfor-packing), then the rest in groups of 32 [sequentially](#fastpfor-packing).
The last 32-value group is padded with zero values, and the words the padding alone would fill are not written, so the remainder takes `ceil(remainder * k / 32)` words.
Arrays follow in ascending `k`.

A block is decoded by unpacking its `128` values at `width` bits, then patching its exceptions:

```
index = max_width - width
for each position p:
    index == 1:  value[p] |= 1 << width
    otherwise:   value[p] |= next value of array[index] << width
```

Arrays are split by `k` so that each packs at its exact width and no exception stores its own width.
An exception that is a single bit wider than the block has no array: its high part can only be `1`.
Every array has its own read position, which starts at its first value on each page.

A decoder MUST reject
- a `width` or `max_width` above the value width,
- a `max_width` that is not above `width` where there are exceptions,
- a `position` of `128` or more,
- an exception without a value left in its array,
- an array with more than `65536` values,
- and a payload that ends inside a page.

#### Packing {#fastpfor-packing}

=== "Interleaved"

    A **group** is 128 values packed at `width` bits into `4 * width` words, and a v2 block is one group.
    It is laid out for a 128-bit vector register: 4 lanes of `u32`, or 2 lanes of `u64`.

    Value `i` belongs to lane `i mod lanes` and row `i / lanes`.
    Each lane is packed on its own, LSB-first, at `width` bits per value: row `r` occupies bits `r * width` to `r * width + width - 1` of the lane's bits.
    A lane has `128 / lanes` rows of `width` bits, which is exactly `width` words of the lane's own width.
    The group stores word `w` of every lane as one 128-bit vector, for `w = 0` to `width - 1`:

    - for `u32`, lane `l` of vector `w` is the word at `4 * w + l`,
    - for `u64`, lane `l` of vector `w` is the pair of words at `4 * w + 2 * l` and `4 * w + 2 * l + 1`, low half first.

    A `width` of `0` takes no words, and every value is `0`.

    --8<-- "diagrams/fastpfor-interleave.svg"

    Because every lane has the same bit offsets, one shift, one mask and one OR move `lanes` values at once, and no value crosses lanes.
    A decoder without vector instructions loops over the lanes and reads the same bytes.

=== "Sequential"

    32 values at `k` bits each are one continuous LSB-first bit stream of `k` words: value `j` occupies bits `j * k` to `j * k + k - 1`, with bit `i` of the stream being bit `i mod 32` of word `i / 32`.
    v1 packs every block this way, 32 values at a time.
    v2 uses it only for the remainder of an exception array.
    A `u64` value of up to `64` bits crosses word boundaries like any other.

    --8<-- "diagrams/fastpfor-sequential.svg"

#### Tail {#fastpfor-tail}

The tail is variable-byte encoded, with no count and no header.
Each value is stored in 7-bit groups, least significant group first, and bit 7 of its **last** byte is set.
This is the reverse of [varint](#varint), where bit 7 marks a byte that is followed by another.

A `u32` takes 1 to 5 bytes, and the fifth carries the top 4 bits.
A `u64` takes 1 to 10.
The bytes are padded with `0x00` to a multiple of 4.
A padding byte has bit 7 clear, so it never ends a value, and a decoder discards the unfinished value at the end.
A decoder reads until the payload ends, and the number of values it read MUST be the stream's value count minus `block_values`.

#### Choosing the width

The width is the encoder's choice, and any width that decodes to the values is valid.
The reference encoder counts the values of a block by bit length and tries each `width` below the widest value's `max_width`.
With `e` values wider than `width`, it costs

```
128 * width + e * 8 + e * (max_width - width) + 8        bits
```

for the packed block, the position bytes, the exception bits and the `max_width` byte.
It picks the cheapest, and `max_width` itself when no smaller width beats `128 * max_width`.
It never picks a `width` that makes every value an exception.

#### Example

Take 130 `u32` values, `i mod 8` for `i` from `0`, except `300` at `5`, `20` at `77`, `5` at `128` and `200` at `129`.
The first 128 fit in `3` bits except two, so `width = 3` and `max_width = 9`.
The last two are the tail.

--8<-- "diagrams/fastpfor-example.svg"

The same values as `u64` take 22 words, since the bitmap has two.

In v2 a stream of 64-bit words, such as the offsets of a [framed, exception-free ALP](#alp) column, uses the `u64` form.
v1 has no 64-bit FastPFOR, so a v1 stream of 64-bit words cannot use it.

## Logical Encodings

The logical encoding is applied to the words the physical step produced.

### None {#logical-none}

The words are the values.
On a signed stream each word is [zigzag](#zigzag)-decoded.

### Delta {#delta}

Each word is the difference from the previous value.
The first value's predecessor is `0`.
Each difference is [zigzag](#zigzag)-encoded before it becomes a word, whatever the stream's type.

[View example](inspector/app/?fixture=0x02%2Fids_opt_delta.mlt&at=column%5B0%5D){target=_blank .inspector-example} - ids `100`, `101`, `105`, `106`, stored as the steps between them.

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

### Delta2 <span class="experimental"></span> {#delta2}

Each word is the [delta](#delta) of the deltas.
Both predecessors start at `0`, so the first two words are
- `values[0]` and
- `values[1] - 2 * values[0]`.

Each second difference is [zigzag](#zigzag)-encoded, and the arithmetic wraps at the word width.

Encoding is:
```
delta[i]  = values[i] - values[i - 1]      values[-1] = 0
delta2[i] = delta[i] - delta[i - 1]        delta[-1] = 0
```

and decoding is:
```
delta[i] = delta[i - 1] + delta2[i]
values[i] = values[i - 1] + delta[i]
```

!!! example

    ```
    values: [10, 13, 16, 20]
    deltas: [10, 3, 3, 4]
    delta2: [10, -7, 0, 1]
    words:  [20, 13, 0, 2]         zigzag
    ```

Delta2 suits smooth sequences, such as a per-vertex elevation along a densely sampled line, whose steps change little from one to the next.

### RLE {#rle}

The values are stored as runs, each a `(run_length, value)` pair that expands to `run_length` copies of `value`.
On a signed stream `value` is [zigzag](#zigzag)-encoded.
Run lengths never are.

[View example](inspector/app/?fixture=0x02%2Fmvalues_rle.mlt&at=m_value%5B0%5D){target=_blank .inspector-example} - five `5`s and three `7`s, as two runs.

The two tile versions lay the runs out differently:

| | Payload | Run count |
|---|---|---|
| **v1** | All run lengths, then all values, physically encoded as one word sequence | `runs` varint in the stream header |
| **v2** | Interleaved `(run_length, value)` pairs as varints, no physical field | Not stored; read pairs until `byte_length` is exhausted |

For the input `[5, 5, 5, 3, 3, 3, 3]`:

=== "Runs"

    --8<-- "diagrams/rle.svg"

=== "Values"

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

### Delta-RLE {#delta-rle}

[Delta](#delta) followed by [RLE](#rle).
The deltas are ZigZag-encoded, and those unsigned words are run-length encoded.
Decoding undoes both steps in reverse order: expand the runs, then undo ZigZag and prefix-sum.

[View example](inspector/app/?fixture=0x02%2Fids_delta_rle.mlt&at=column%5B0%5D){target=_blank .inspector-example} - four equal ids: one step of `103`, then a run of three `0`s.

=== "Runs"

    --8<-- "diagrams/delta-rle.svg"

=== "Values"

    ```
    values: [10, 11, 12, 13, 20, 20, 20]
    deltas: [10, 1, 1, 1, 7, 0, 0]
    words:  [20, 2, 2, 2, 14, 0, 0]          zigzag
    runs:   (1, 20) (3, 2) (1, 14) (2, 0)
    ```

Delta-RLE suits sequences with a constant step, such as `[1, 2, 3, ...]`, which become one run.

## Boolean Streams

Boolean columns and v1 `Present` streams hold one bit per value.

### Bitmap {#bitmap}

Bit `i % 8` of byte `i / 8` is value `i`, LSB-first.
The bitmap is `ceil(count / 8)` bytes.
Bits past `count` in the final byte are padding and MUST be ignored.

[View example](inspector/app/?fixture=0x02%2Fprop_i32_val_null.mlt&at=present){target=_blank .inspector-example} - a value, then a null: bits `1` and `0`.

=== "Bits"

    --8<-- "diagrams/bitmap.svg"

=== "Values"

    ```
    values: [1, 0, 1, 1, 0, 1]
    byte:   0b0010_1101 = 0x2D
    ```

### Runs <span class="experimental"></span> {#bool-runs}

Alternating run lengths as varints.
The first counts the leading `0` bits and can be `0`.
The lengths MUST sum to `count`.
Runs is best for a bitfield that is set over contiguous blocks of values.

```
values:  [0, 0, 1, 1, 1, 0]
payload: 02 03 01              2 zeros, 3 ones, 1 zero
```

### Sparse <span class="experimental"></span> {#bool-sparse}

A summary bitmap with one bit per byte of the [bitmap](#bitmap), set where that byte is not zero, followed by the non-zero bytes in order.
The summary is `ceil(ceil(count / 8) / 8)` bytes, and its bits past the last bitmap byte MUST be `0`.
A stored byte MUST NOT be `0`.
Sparse is best for a bitfield whose set bits are rare and do not clump.

```
bitmap:  00 2D 00              24 values
summary: 02                    only byte 1 is non-zero
payload: 02 2D
```

### Boolean RLE {#boolean-rle}

v1 compresses the [bitmap](#bitmap) with the byte-level run-length encoding of [ORC](https://orc.apache.org/specification/ORCv1/#byte-run-length-encoding).
The payload is a sequence of runs, each a control byte followed by the bytes it describes.

[View example](inspector/app/?fixture=0x01%2Fids_opt.mlt&at=present){target=_blank .inspector-example} - five ids, one of them missing, as byte runs.

| Control byte `c` | Meaning |
|---|---|
| `0`-`127` | A repeated run: the next byte, `c + 3` times |
| `128`-`255` | A literal run: the next `256 - c` bytes as they are |

A repeated run is 3 to 130 bytes and a literal run 1 to 128.
The runs expand to exactly `ceil(count / 8)` bitmap bytes.
A payload that expands to more, or ends before that, MUST be rejected.

=== "Runs"

    --8<-- "diagrams/boolean-rle.svg"

=== "Values"

    ```
    bitmap:  FF FF FF FF FF 2D
    payload: 02 FF FF 2D
             02 FF          repeated run: 2 + 3 = 5 copies of FF
             FF 2D          literal run: 256 - 255 = 1 byte, 2D
    ```

The v1 stream header carries no `runs` or `num_rle_values` for a boolean stream.
Both follow from `num_values`.

## Float Streams

### Plain Floats {#plain-floats}

IEEE 754 words, little-endian, 4 bytes for a `Float` and 8 for a `Double`.
This is the only float encoding v1 has.

[View example](inspector/app/?fixture=0x02%2Fprop_f64_max_np.mlt&at=column%5B0%5D){target=_blank .inspector-example} - raw IEEE 754 words.

### Framed, Exception-Free ALP <span class="experimental"></span> {#alp}

Framed, Exception-Free ALP (Adaptive Lossless floating-Point compression) stores a float column as integers, which are then [physically encoded](#physical-encodings) like any other integer stream.

[View example](inspector/app/?fixture=0x02%2Fprop_f32_alp_np.mlt&at=column%5B0%5D){target=_blank .inspector-example} - `-0.75` to `0.75`, stored as `-75` to `75` with `e = 2`.

Most floats in map data are decimals with few significant digits, such as `12.75` or `0.3`, and are exactly representable as a scaled integer.
Framed, Exception-Free ALP finds one decimal scale for the whole column and stores $i = \operatorname{round}(v \cdot 10^e / 10^f)$ per value.

$e$ is the decimal exponent the values were scaled by, $0 \le e \le 18$.
$f$ is the factor dividing out the trailing zeros $e$ introduced, $0 \le f \le e$.

The parameters are two values in the stream header:

| Parameter | Meaning |
|---|---|
| `scale` | One byte packing $e$ and $f$ as $\frac{e(e+1)}{2} + f$, from $0$ to $189$ |
| `base` | Frame of reference: the smallest scaled integer in the column, ZigZag varint |

`scale` numbers only the valid pairs, row by row.
A 4-bit nibble for each of $e$ and $f$ would be simpler but cannot hold $e = 18$.
Two 5-bit fields would not fit in a byte, while the $190$ valid pairs up to $(e, f) = (18, 18)$ do.
A `scale` above $189$ MUST be rejected.

To decode, $e$ is the largest integer with $\frac{e(e+1)}{2} \le \mathit{scale}$, and $f$ is what remains:

$$
e = \left\lfloor \frac{\sqrt{8 \cdot \mathit{scale} + 1} - 1}{2} \right\rfloor
\qquad
f = \mathit{scale} - \frac{e(e+1)}{2}
$$

The square root is exact enough in double precision for every valid `scale`.

--8<-- "diagrams/alp-scale.svg"

The payload holds unsigned offsets from `base`, so the smallest is `0` and every value is non-negative.
The offsets are an ordinary unsigned integer stream and carry their own physical encoding.
The stream's words are 64-bit.

**Decoding**: $i = \mathit{base} + \mathit{offset}$, in 64-bit integer arithmetic, then $v = i \cdot 10^f \cdot 10^{-e}$, with $10^{-e}$ the nearest double and not an exact division.

The sum MUST be formed as an integer before the conversion.
A column spanning $[-2, 2^{53} - 1]$ has an offset of $2^{53} + 1$, which a double cannot hold.

$e$ and $f$ are stored separately instead of as a single $10^{e - f}$.
Scaling up by $10^e$ and then down by $10^f$ rounds twice, and some values are only exactly representable with $f > 0$.

!!! important "Rounding"
    An encoder MUST decode every value back and compare bit patterns.
    If any value does not round-trip exactly, the encoder MUST use another encoding.

$e$ is at most $18$ so that $v \cdot 10^e$ fits in an `i64`.

!!! NOTE
    The reference Rust encoder currently emits only scaled integers with $|i| \le 2^{53} - 1$, where consecutive doubles are at most $1$ apart.
    Beyond that, floating-point scaling can land on a neighboring integer that still passes its own round-trip check.
    This is a limitation of that implementation, not of the format.

=== "Offsets"

    --8<-- "diagrams/alp.svg"

=== "Values"

    ```
    values:  [-0.75, 0.25, 1.5, -2.25]
    e = 2, f = 0:  i = [-75, 25, 150, -225]
    base = -225:   offsets = [150, 250, 375, 0]
    ```

The header stores `e = 2`, `f = 0` as the `scale` byte `03` and `base` as the ZigZag varint `c1 03`, and the payload stores the four offsets as varints.
See the [framed, exception-free ALP example](specification/v2.md#examples) on the v2 page for the whole layer.

### Float Dictionary <span class="experimental"></span> {#float-dictionary}

The distinct values are stored once, and a stream of codes holds one index into them per element.

[View example](inspector/app/?fixture=0x02%2Fprop_f32_dict_nan_np.mlt&at=column%5B0%5D){target=_blank .inspector-example} - repeated floats, `NaN` among them.

The column has two streams.
The first is the codes: an integer stream of 32-bit words, logical `Dict` in the `Float` family, with its own physical encoding.
The second is the dictionary: [plain floats](#plain-floats), one per distinct value, with an explicit count in its header.

=== "Codes"

    --8<-- "diagrams/float-dictionary.svg"

=== "Values"

    ```
    values: [1.5, 0.25, 1.5, 1.5, 0.25]
    codes:  [0, 1, 0, 0, 1]
    dict:   [1.5, 0.25]
    ```

Entries are distinct by bit pattern.
`-0.0` and `0.0` are two entries, and a `NaN` is never equal to any entry.
A code at or past the dictionary's count MUST be rejected.

## Strings {#strings}

String values, dictionary values and FSST symbol tables are byte blobs.
A blob's count is its `byte_length`, and a lengths stream beside it says where each value ends.

A string column is stored with one of four encodings, which combine a blob with an optional dictionary:

| Encoding | Stores |
|---|---|
| Plain | Every value's bytes as [plain bytes](#plain-bytes) |
| Dictionary | The distinct values once, and a code per value |
| FSST | Every value's bytes, compressed with [FSST](#fsst) |
| FSST dictionary | The distinct values once, compressed with [FSST](#fsst), and a code per value |

The streams of each encoding and how a column announces it are the business of [v1](specification/v1.md#string-columns) and [v2](specification/v2.md#string-columns).

### Plain Bytes {#plain-bytes}

The bytes as they are.
Value `i` is the `lengths[i]` bytes that follow the first `lengths[0] + ... + lengths[i - 1]`.
Each value MUST be valid UTF-8.

[View example](inspector/app/?fixture=0x02%2Fprops_str_plain_np.mlt&at=column%5B0%5D){target=_blank .inspector-example} - several strings, each its length's worth of bytes in turn.

### Front Coding <span class="experimental"></span> {#front-coding}

Neighboring entries of a sorted dictionary often share a prefix, for example `Main Street`, `Main Street North` and `Maple Avenue`.
Front coding stores the length of the prefix shared with the previous entry, and only the suffix bytes.

[View example](inspector/app/?fixture=0x02%2Fprops_str_front_dict_np.mlt&at=column%5B0%5D){target=_blank .inspector-example} - a dictionary whose entries share prefixes.

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

=== "Prefixes"

    --8<-- "diagrams/front-coding.svg"

=== "Values"

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

### FSST {#fsst}

Fast Static Symbol Table compression replaces frequent byte sequences of up to 8 bytes with one-byte codes.
Unlike a dictionary, it compresses strings that merely share substrings, such as localized country names.
It decodes quickly, since every code is one byte.

[View example](inspector/app/?fixture=0x02%2Fprops_str_fsst_dict_np.mlt&at=column%5B0%5D){target=_blank .inspector-example} - a symbol table and the codes into it.

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

=== "Expansion"

    --8<-- "diagrams/fsst.svg"

=== "Values"

    ```
    symbols:      ["ab", "cd"]           SymbolLengths = [2, 2], SymbolTable = "abcd"
    values:       ["abcd", "abz"]        Lengths = [4, 3]
    corpus bytes: 00 01 00 FF 7A         ab cd ab <esc> z
    ```

The algorithm that trains the table is described by [Boncz, Neumann and Leis, *FSST: Fast Random Access String Compression*](https://www.vldb.org/pvldb/vol13/p2649-boncz.pdf).
Different implementations train different tables for the same input.
A decoder reads the table from the stream, so this only matters when comparing the output of two encoders.

### Dictionary {#string-dictionary}

The distinct values are stored once, and a code names one of them per element.
Repeated values such as a city or a country cost one code instead of their bytes.

[View example](inspector/app/?fixture=0x02%2Fprops_str_dict_bp_np.mlt&at=codes){target=_blank .inspector-example} - three distinct strings and one code per feature.

The codes are an unsigned integer stream, so any [logical](#logical-encodings) and [physical](#physical-encodings) encoding can compress them.
The dictionary is a byte blob, [plain](#plain-bytes), [front-coded](#front-coding) or [FSST](#fsst)-compressed.
Its lengths are those of the distinct values.

```
values: ["Oslo", "Bergen", "Oslo", "Oslo"]
codes:  [0, 1, 0, 0]
dict:   ["Oslo", "Bergen"]
```

A code at or past the dictionary's count MUST be rejected.
A dictionary beats plain strings once values repeat, and loses when they are mostly distinct, so an encoder SHOULD compare the stored size of both.

### Shared Dictionary {#shared-dictionary}

Several string columns, such as `name:en`, `name:de` and `name:fr`, index into one dictionary.
The dictionary is written once, and each member column then stores only its presence and its codes.

[View example](inspector/app/?fixture=0x02%2Fprops_shared_dict_bp.mlt&at=column%5B0%5D){target=_blank .inspector-example} - several columns reading one corpus.

The dictionary can be plain, front-coded or FSST-compressed, as for a single column.
See [v1](specification/v1.md#shared-dictionary-columns) and [v2](specification/v2.md#shared-dictionary-columns) for the group header each version writes.

## Vertex Streams

The vertex buffer holds $x$ and $y$ interleaved: $[x_0, y_0, x_1, y_1, \ldots]$.
Its words are 32-bit and signed.

### Componentwise Delta {#componentwise-delta}

Each coordinate is a delta to the same coordinate of the previous vertex.
`x` and `y` keep separate predecessors, both starting at `0`.
Each delta is [zigzag](#zigzag)-encoded.

[View example](inspector/app/?fixture=0x02%2Fline.mlt&at=vertices){target=_blank .inspector-example} - three vertices, each stored as its step from the one before.

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

The v2 `Vertex` family also has a plain `Delta`, which is the integer [delta](#delta) over the flat word sequence and does not separate the components.

### Componentwise Delta2 <span class="experimental"></span> {#componentwise-delta2}

[Delta2](#delta2) of each coordinate, with `x` and `y` keeping separate predecessors.

```
vertices: (0, 0), (70, 1), (140, 3), (210, 6)
deltas:   (0, 0), (70, 1), (70, 2), (70, 3)
delta2:   (0, 0), (70, 1), (0, 1), (0, 1)
words:    [0, 0, 140, 2, 0, 2, 0, 2]
```



### Vertex Dictionary {#vertex-dictionary}

The distinct vertices are stored once in a `VertexDict` stream, and a `VertexOffsets` stream holds one index into them per vertex.
`VertexOffsets` is an ordinary unsigned integer stream.
`VertexDict` is a vertex stream and carries any of the encodings in this section.

[View example](inspector/app/?fixture=0x02%2Fpoint_morton_dictionary.mlt&at=vertex_dict){target=_blank .inspector-example} - vertices stored once and indexed.

=== "Offsets"

    --8<-- "diagrams/vertex-dictionary-offsets.svg"

=== "Values"

    ```
    VertexOffsets: [0, 1, 2, 1, 0, 2]
    VertexDict:    [(0,0), (10,10), (20,20)]
    vertices:      (0,0), (10,10), (20,20), (10,10), (0,0), (20,20)
    ```

An offset at or past the dictionary's vertex count MUST be rejected.

#### Hilbert Order

Encoders SHOULD sort the dictionary along a Hilbert curve, so that the deltas between neighboring entries stay short.
The order is not part of the format.
A decoder resolves vertices through `VertexOffsets` and never depends on the order.

The reference encoders use the curve of the [`hilbert_2d`](https://crates.io/crates/hilbert_2d) crate's `Hilbert` variant on a $2^{\mathit{bits}} \times 2^{\mathit{bits}}$ grid.
`shift` and `bits` are derived as for [Morton](#morton) below, and each shifted coordinate is masked to 16 bits before it is placed on the grid.
Two vertices with the same curve key are one dictionary entry.

### Morton {#morton}

A Morton, or Z-order, code interleaves the bits of a coordinate pair into one integer.
Nearby vertices get nearby codes, so a sorted Morton dictionary has small deltas.
Only a `VertexDict` stream uses it.

[View example](inspector/app/?fixture=0x02%2Fpoint_morton_dictionary.mlt&at=vertex_dict){target=_blank .inspector-example} - vertices as Morton codes, delta-encoded.

The parameters are two varints in the stream header:

| Parameter | Meaning |
|---|---|
| `bits` | Bits per axis, at most `16` |
| `shift` | Added to `x` and to `y` before interleaving, so that both are non-negative |

`bits > 16` MUST be rejected.

Encoders derive both from the whole layer's vertices.
`shift` is `-min` when the smallest coordinate `min` on either axis is negative, else `0`.
`bits` is the bit width of `max + shift`, the largest shifted coordinate.

Encoding takes two steps.
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

=== "Bits"

    --8<-- "diagrams/morton.svg"

=== "Values"

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
The stream holds the first code, then each code's difference from the previous one.
v1 names this `MortonDelta`; v2 names it `Morton` in the `Vertex` family.
There is no plain or RLE Morton encoding.

The deltas are plain differences, not ZigZag-encoded.
The dictionary is sorted ascending by code, so every difference is non-negative.

```
dict codes: [27, 30, 45]
words:      [27, 3, 15]
```

### rANS <span class="experimental"></span> {#rans}

rANS entropy-codes the [componentwise deltas](#componentwise-delta) of a plain layout's vertex stream.
A `VertexDict` stream MUST NOT use it.

Each delta is split in two:

- a **symbol**, which says roughly how large the delta is and its sign, and is coded with [rANS](https://arxiv.org/abs/1311.2540) so that frequent symbols take fewer bits,
- the delta's low **raw bits**, which are close to random and stored as they are.

The symbol frequencies are counted per **context**, so each delta is coded with a table built from similar deltas.
The tables travel in the payload.

To decode, a decoder:

1. reads the depth and the tables,
2. marks which vertices start a [run](#rans-runs), from the topology streams,
3. per vertex, decodes the `dx` symbol, then the `dy` symbol in a context that depends on `dx`, then reads both deltas' raw bits,
4. prefix-sums the deltas into coordinates, as for componentwise delta.

#### How rANS decodes a symbol

A table of precision `p` divides `2^p` slots among its symbols.
Symbol `s` owns `freq(s)` consecutive slots, starting at `cum(s)`, the sum of the frequencies of the symbols below it.

The decoder holds a state `x`, a 32-bit number.
The low `p` bits of `x` are a slot, and the symbol owning that slot is the one decoded.
The state then shrinks by about `log2(2^p / freq(s))` bits, so a symbol owning half the slots costs one bit, and one owning a quarter costs two.
When the state drops below `2^16`, the next 16-bit word of the stream is shifted in from below.
The state stays in `[2^16, 2^32)` between symbols.

```
slot = x & (2^p - 1)
s    = the symbol whose slots hold slot
x    = freq(s) * (x >> p) + slot - cum(s)
if x < 2^16: x = (x << 16) | next refill word of the lane
```

A table of precision `3`, decoding the state `0x10005`:

--8<-- "diagrams/rans-slots.svg"

The state becomes `2 * 8192 + 5 - 4 = 16385`, two bits smaller, and is below `2^16`, so the next word is shifted in.

A table of precision `0` has one slot and one symbol.
Decoding from it leaves `x` unchanged and costs no bits.

#### Why rANS compresses {#rans-math}

A symbol of probability $P$ carries $\log_2 \frac{1}{P}$ bits of information.
A table approximates $P(s)$ as $\frac{\mathit{freq}(s)}{2^p}$, so the ideal cost of $s$ is $\log_2 \frac{2^p}{\mathit{freq}(s)}$ bits.

The state $x$ holds the symbols encoded so far in about $\log_2 x$ bits.
Encoding $s$ maps $x$ to

$$
x' = 2^p \left\lfloor \frac{x}{\mathit{freq}(s)} \right\rfloor + \mathit{cum}(s) + (x \bmod \mathit{freq}(s))
$$

so $x' \approx x \cdot \frac{2^p}{\mathit{freq}(s)}$, and $\log_2 x$ grows by the ideal cost of $s$.

With every frequency `1` this is $x' = 2^p x + s$, which appends $s$ as a digit of $x$ in base $2^p$, and every symbol costs $p$ bits.
rANS instead gives $s$ the $\mathit{freq}(s)$ digit values from $\mathit{cum}(s)$ on.
The digit is $\mathit{cum}(s) + (x \bmod \mathit{freq}(s))$, so it carries $\log_2 \mathit{freq}(s)$ bits of $x$.

The decoder finds $s$ as the symbol whose slots hold the digit $x' \bmod 2^p$.
The quotient $\lfloor x' / 2^p \rfloor$ is $\lfloor x / \mathit{freq}(s) \rfloor$, and the digit's offset into those slots is $x \bmod \mathit{freq}(s)$, so the decoder rebuilds $x$ exactly.
Every $x'$ decodes to exactly one pair of $x$ and $s$.

The last symbol encoded is the lowest digit and is decoded first, so the [encoder](#rans-encoding) codes each lane in reverse.

Encoding symbol `3` of the table above into $x = 16385$ gives the state the decoder started from:

$$
x' = 8 \left\lfloor \frac{16385}{2} \right\rfloor + 4 + (16385 \bmod 2) = 8 \cdot 8192 + 5 = 65541 = \mathtt{0x10005}
$$

$x$ grows by $\log_2 \frac{65541}{16385} \approx 2 = \log_2 \frac{8}{2}$ bits.

If symbol $s$ occurs $n(s)$ times among the $N$ symbols of a context, that context costs on average

$$
\sum_s \frac{n(s)}{N} \log_2 \frac{2^p}{\mathit{freq}(s)}
$$

bits per symbol.
It is lowest when $\frac{\mathit{freq}(s)}{2^p} = \frac{n(s)}{N}$ for every $s$, and is then the entropy $\sum_s \frac{n(s)}{N} \log_2 \frac{N}{n(s)}$.
A low precision rounds the frequencies further from the counts and costs more per symbol, and a high one costs more table bits.

#### Renormalization {#rans-renormalization}

The state stays in $[2^{16}, 2^{32})$.
The encoder shifts 16-bit words out of the state into its lane, and the decoder shifts them back in.

Before encoding $s$, the encoder shifts words out until $x < \mathit{freq}(s) \cdot 2^{32 - p}$, so that $x' < 2^{32}$.
A shift divides $x$ by $2^{16}$, and the interval $[\mathit{freq}(s) \cdot 2^{16 - p}, \mathit{freq}(s) \cdot 2^{32 - p})$ spans exactly that factor.
So $x$ lands in it after a unique number of shifts, and $x' \ge 2^{16}$.

Decoding from $x \ge 2^{16}$ leaves $x \ge \mathit{freq}(s) \cdot 2^{16 - p} \ge 2^{16 - p}$.
Since $p \le 11$, one refill word brings $x$ back to at least $2^{16}$.

[*Asymmetric numeral systems*](https://arxiv.org/abs/1311.2540) by Duda proves these properties, and [*rANS*](https://reearth.engineering/posts/r-ans-en/) by Re:Earth derives them with small decimal examples.

#### Runs {#rans-runs}

The first vertex of a ring, part or geometry jumps from the last vertex of the one before it, so its delta is much larger than the others.
Such a vertex is encoded in contexts of its own.

A run is the vertex range
- of each ring, else
- of each part, else
- of each geometry, taken from the deepest offset level the geometry section has.

A layer with none of the three levels has one run per vertex.
A vertex starts a run when it is the first of one.
The runs MUST cover exactly the stream's `num_values` vertices.
Because the runs come from the topology, the stream is decoded after the topology streams.

#### Symbols

Each delta is the wrapping 32-bit difference from the same coordinate of the previous vertex, as in componentwise delta, without ZigZag.

The symbol keeps the bit length `len` of the magnitude and the `d` bits after its leading one.
`d` is the stream's mantissa depth, `0` to `2`.
The leading one is implied by `len`.
The remaining low bits are the raw bits.
This is the same split a float makes into exponent, mantissa and dropped precision.

At depth `1`:

=== "Bits"

    --8<-- "diagrams/rans-symbol.svg"

=== "Values"

    | Delta | Binary | `len` | Kept bit | Raw bits | Symbol |
    |---:|---:|---:|---:|---:|---:|
    | `0` | | | | | `0` |
    | `1` | `1` | `1` | | | `3` |
    | `-1` | `1` | `1` | | | `4` |
    | `2` | `10` | `2` | `0` | | `7` |
    | `3` | `11` | `2` | `1` | | `9` |
    | `-3` | `11` | `2` | `1` | | `10` |
    | `5` | `101` | `3` | `0` | `1` | `11` |
    | `100` | `1100100` | `7` | `1` | `00100` | `29` |

Positive deltas get odd symbols and negative ones the even symbol above.
A magnitude with fewer than `d` bits after its leading one makes the unused mantissa positions `0`.

```
delta = 0:  symbol 0, no raw bits
otherwise:  m = |delta|                       as an unsigned 32-bit value
            len = bit length of m             1-32
            k = min(len - 1, d)               mantissa bits the symbol keeps
            raw_bits = len - 1 - k
            mantissa = (m >> raw_bits) & (2^k - 1)
            code = (len << d) | (mantissa << (d - k))
            symbol = 2 * code - 1 if delta > 0, 2 * code if delta < 0
            raw = the low raw_bits bits of m
```

Decoding reverses it:

```
symbol = 0:  delta 0
otherwise:   code = ceil(symbol / 2)
             len = code >> d
             k = min(len - 1, d)
             raw_bits = len - 1 - k
             mantissa = (code & (2^d - 1)) >> (d - k)
             m = ((2^k | mantissa) << raw_bits) | the next raw_bits raw bits
             delta = m if symbol is odd, -m if it is even     wrapping, so m = 2^31 is i32::MIN
```

Symbols run up to `64` at depth `0`, `128` at depth `1` and `256` at depth `2`, the symbol of `i32::MIN`.
The codes above it at depths `1` and `2` name no delta.
A larger depth moves bits from the raw bits into the symbol, where a predictable bit costs less than one, at the price of larger tables.

#### Contexts

There are 14 contexts, each with its own table:

| Context | Codes |
|---|---|
| `0` | `dx` inside a run |
| `1` | `dx` at a run start |
| `2 + 2c + s` | `dy`, where `c` is `dx`'s `len` capped at `5`, `0` for a zero `dx`, and `s` is `1` at a run start |

`dx` and `dy` of a vertex tend to be of similar size, so the `dy` table is picked by how large `dx` was.

A polygon with a hole:

--8<-- "diagrams/rans-runs.svg"

#### Payload

```
payload := [u8 depth]           0-2
           [tables]             bit-packed LSB-first, padded to a byte
           [varint lane_len; 4]
           [lane; 4]
           [raw bits]           LSB-first, padded to a byte
```

The tables are one per context, in context order:

```
table := [1 bit present]
         if present:
           [4 bits precision]   0-11
           [9 bits max_symbol]  0-256
           if precision > 0:
             [gamma(freq + 1)] for every symbol 0..=max_symbol
```

`gamma(v)` is Elias gamma, LSB-first: `z` zero bits, a one bit, then the low `z` bits of `v`, where `z` is the bit length of `v` minus one.
A zero frequency is `gamma(1)`, a single one bit.
`z` above `12` MUST be rejected.

A table of precision `0` has no frequencies on the wire.
Its only symbol is `max_symbol`, with frequency `1`.

The frequencies MUST sum to `2^precision`, the frequency of `max_symbol` MUST be non-zero, and a symbol with a non-zero frequency MUST be one that some delta maps to at the stream's depth.

#### Lanes

Four rANS states decode in parallel, each with a lane of its own.
Vertex `i` codes `dx` on lane `2 * (i mod 2)` and `dy` on lane `2 * (i mod 2) + 1`, so even vertices use lanes `0` and `1`, and odd vertices lanes `2` and `3`.
A lane is its state's initial value as a little-endian `u32`, then the little-endian `u16` refill words in the order they are read.
Its length MUST be even and at least `4`.

The raw bits are one stream shared by all lanes, read in vertex order: `dx`'s raw bits, then `dy`'s.

The symbols and raw bits of the example below:

--8<-- "diagrams/rans-lanes.svg"

```
x_prev = y_prev = 0
for i in 0..vertex_count:
    s  = 1 if vertex i starts a run, else 0
    a  = 2 * (i mod 2)
    sx = decode a symbol on lane a,     from context s
    sy = decode a symbol on lane a + 1, from context 2 + 2 * min(ceil(sx / 2) >> d, 5) + s
    x_prev = x_prev + delta(sx)           reads dx's raw bits, wrapping
    y_prev = y_prev + delta(sy)           reads dy's raw bits, wrapping
    output x_prev, y_prev
```

A decoder MUST reject a symbol from a context without a table, a lane that does not end on state `2^16` with every word read, and raw bits whose byte length is not exactly the number of bytes the raw bit reads need.

#### Encoding {#rans-encoding}

rANS is last in, first out, so an encoder codes each lane's symbols in reverse, starting every state at `2^16`:

```
while x >= freq(s) * 2^(32 - p): emit x & 0xFFFF; x = x >> 16
x = ((x / freq(s)) << p) + x mod freq(s) + cum(s)
```

A lane is the final state, then the emitted words in reverse order.
The final state is where decoding starts, and decoding every symbol returns it to `2^16`.

An encoder SHOULD pick the depth and precisions that make the stream shortest, and SHOULD compare its stored size with componentwise delta.

!!! example

    The vertices `(100, 200)`, `(105, 210)`, `(102, 215)` as one run, at depth `1`:

    | Vertex | Lanes | `dx` | Symbol | Context | Raw bits | `dy` | Symbol | Context | Raw bits |
    |---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
    | `0` | `0`, `1` | `100` | `29` | `1` | `00100` | `200` | `33` | `13` | `001000` |
    | `1` | `2`, `3` | `5` | `11` | `0` | `1` | `10` | `15` | `8` | `10` |
    | `2` | `0`, `1` | `-3` | `10` | `0` | | `5` | `11` | `6` | `1` |

    Context `0` holds symbols `10` and `11`, once each, and gets precision `1` with one slot per symbol.
    Contexts `1`, `6`, `8` and `13` hold a single symbol each and get precision `0`.
    The other nine contexts are absent.

    Only context `0` changes a state.
    Lane `0` starts at `0x20000`, decodes `29` for free, then slot `0x20000 & 1 = 0`, symbol `10`, and ends on `1 * (0x20000 >> 1) + 0 - 0 = 0x10000`.
    Lane `2` starts at `0x20001`, slot `1`, symbol `11`, and ends on `1 * 0x10000 + 1 - 1 = 0x10000`.
    Lanes `1` and `3` start and end at `0x10000`.
    No state drops below `2^16`, so no lane has refill words.

    ```
    depth:    01
    tables:   63 C1 FF 52 E8 00 61 81 F0 00 42 08     95 bits, padded
    lanes:    04 04 04 04                            four lanes of 4 bytes
              00 00 02 00                            lane 0: 0x00020000
              00 00 01 00                            lane 1: 0x00010000
              01 00 02 00                            lane 2: 0x00020001
              00 00 01 00                            lane 3: 0x00010000
    raw bits: 04 69                                  15 bits: 4, 8, 1, 2, 1, padded
    ```
