# FastPFOR fixtures

This directory contains cross-language **FastPFOR** test vectors extracted from the C++ tests.

## Origin

The files correspond to the `compressed{1..4}` / `uncompressed{1..4}` arrays in:

- `cpp/test/test_fastpfor.cpp`

## File formats

For each `vectorN`:

- `vectorN_encoded.bin`
  - Big-endian bytes of the FastPFOR-encoded **32-bit word stream** (`uint32_t[]` in C++).
  - This is the FastPFOR *wire format* as consumed by TS `decodeFastPfor` (big-endian int32 words).
  - Fixtures are stored in canonical form: trailing `0x00000000` padding words are trimmed.
- `vectorN_decoded.bin`
  - Big-endian bytes of the expected **decoded int32 values** (`uint32_t[]` in C++).
  - When interpreted as signed int32, values use two’s complement (e.g. the `-100..99` range in `vector3`).
