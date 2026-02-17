# Fuzzing for mlt-nom

This directory contains fuzzing infrastructure for the `mlt-nom` parser using [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) and [libFuzzer](https://llvm.org/docs/LibFuzzer.html).

## Overview

Fuzzing is a software testing technique that provides random or semi-random data as input to find bugs, crashes, and security vulnerabilities.
This fuzzer tests the round-trip property of the MapLibre Tile parser:
Any data that can be successfully parsed should be serializable back to bytes that match the original input.

## What is Being Tested

The fuzzer validates the following property:

```
parse(bytes) -> Layer -> write_to(buffer) -> bytes'
assert_eq!(bytes, bytes')
```

This ensures that:
1. The parser correctly interprets the binary format
2. The serializer produces canonical output
3. No data is lost or corrupted during the parse → serialize round-trip

## Prerequisites

Install `cargo-fuzz`:

```bash
cargo install cargo-fuzz
```

> [!NOTE]
> `cargo-fuzz` requires a nightly Rust toolchain.

## Running the Fuzzer

From the `fuzz` directory:

```bash
cargo +nightly fuzz run layer
```

Popular options to this are a run count via `-- -runs=1000000` or a timeout via `-- -timeout=10`.

## Fuzz Targets

### `layer`

**Location:** `fuzz_targets/layer.rs`

Tests the `Layer` parser and serializer. The fuzzer:
1. Generates arbitrary byte sequences
2. Attempts to parse them as a `Layer`
3. If parsing succeeds and consumes all input, serializes the layer back to bytes
4. Verifies the output matches the original input exactly

If a mismatch is found, the fuzzer panics with a detailed error message showing both the input and output in hexadecimal format.

## Corpus

The `corpus/layer` directory contains input files that have been discovered during fuzzing. These serve as:
- Seeds for future fuzzing runs
- Regression tests to ensure previously found issues don't reoccur
- Examples of valid inputs

The corpus is currently empty but will be populated as fuzzing discovers interesting inputs.

## Artifacts

When the fuzzer discovers a crash or failure, it saves the triggering input to the `artifacts` directory. Each artifact file contains the exact byte sequence that caused the issue.

To reproduce an issue:

```bash
cargo +nightly fuzz run layer artifacts/layer/crash-<hash>
```

## Coverage

To generate coverage information:

```bash
cargo +nightly fuzz coverage layer
```

This creates coverage data in the `coverage` directory, showing which code paths have been exercised during fuzzing.

## Debugging Failures

When the fuzzer finds an issue:

1. The failing input is saved to `artifacts/layer/`
1. Minimize the input using
   ```bash
   cargo fuzz tmin layer artifacts/layer/crash-<hash>
   ```
1. Run with the specific input to reproduce:
   ```bash
   cargo +nightly fuzz run layer artifacts/layer/<filename>
   ```

## Adding New Fuzz Targets

To add a new fuzz target:

1. Create a new file in `fuzz_targets/`:
   ```bash
   cargo fuzz add <target_name>
   ```

1. Implement the fuzz target using `fuzz_target!` macro
1. Add the target to `Cargo.toml` if not automatically added

## Further Reading

- [cargo-fuzz documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
