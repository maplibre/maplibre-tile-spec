# FastPFOR Test Fixtures

Binary fixtures for cross-language FastPFOR validation tests.

## Format

Each `.bin` file contains a sequence of 32-bit unsigned integers in **big-endian** byte order.

## Files

| File | Description |
|------|-------------|
| `vector{N}_uncompressed.bin` | Original uncompressed data (Int32Array view) |
| `vector{N}_compressed.bin` | FastPFOR-encoded data from C++ reference implementation |

## Regeneration

To regenerate these fixtures from the C++ source:

```bash
node scripts/generate-fastpfor-fixtures.mjs
```

## Source

Generated from `cpp/test/test_fastpfor.cpp`.
