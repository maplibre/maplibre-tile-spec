# Fuzzing for mlt

This directory contains fuzzing infrastructure for the `mlt` parser using [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) and [libFuzzer](https://llvm.org/docs/LibFuzzer.html).

## Overview

Fuzzing is a software testing technique that provides random or semi-random data as input to find bugs, crashes, and security vulnerabilities.
This fuzzer tests the round-trip property of the MapLibre Tile parser:
Any data that can be successfully parsed should be serializable back to bytes that match the original input.

## Subproject Structure

```
fuzz/
├── src/
│   └── lib.rs              # mlt_fuzz library with LayerInput and fuzz_roundtrip logic
├── fuzz_targets/
│   └── layer.rs            # Fuzz target that feeds random data to LayerInput
├── tests/
│   └── reproduce.rs        # Template for reproducing fuzzer-found issues
├── corpus/layer/           # Seed inputs for fuzzing
└── artifacts/              # Crash-inducing inputs discovered by fuzzer
```

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

Popular options:
- Run count: `-- -runs=1000000`
- Timeout per input: `-- -timeout=10`

## Fuzz Targets

### `layer`

**Location:** `fuzz_targets/layer.rs`

Tests the `Layer` parser and serializer by generating arbitrary `LayerInput` values and calling `fuzz_roundtrip()` on them.

If a mismatch is found, the fuzzer panics with a detailed error message showing both the input and output in hexadecimal format.

### `differential`

**Location:** `fuzz_targets/differential.rs`

Compares the Rust decoder against the C++ decoder.
Each input is an arbitrary `StagedLayer`.
The target:

1. encodes it to MLT bytes with the Rust encoder,
2. decodes those bytes with the Rust decoder to a `FeatureCollection` JSON, and
3. decodes the same bytes with the C++ `mlt-cpp-json` tool, run as a subprocess.

The two JSON outputs must match.
A mismatch is a crash.
The C++ tool failing on bytes the Rust decoder accepted is also a crash.
The crash report prints both outputs and the input bytes as hex.

The target runs the prebuilt `mlt-cpp-json` binary, so no C++ code or build changes are needed.
That binary is part of the cpp CMake project.
Build it once, then set `$MLT_CPP_JSON_BIN` to its path:

```bash
# Build the C++ tool (from the cpp/ directory)
cmake -S cpp -B cpp/build -DMLT_WITH_TESTS=OFF
cmake --build cpp/build --target mlt-cpp-json

# Run the fuzzer (from rust/mlt-core/fuzz)
export MLT_CPP_JSON_BIN="$PWD/../../../cpp/build/tool/mlt-cpp-json"
cargo +nightly fuzz run differential
```

Coverage comes from the instrumented Rust encode and decode path.
The C++ decoder is a black-box oracle.
Numbers are compared by value, so `0` and `0.0` count as equal.

## Corpus

The `corpus/layer` directory contains input files that have been discovered during fuzzing. These serve as:
- Seeds for future fuzzing runs
- Regression tests to ensure previously found issues don't reoccur
- Examples of valid inputs

The corpus is currently empty but will be populated as fuzzing discovers interesting inputs.

## Artifacts

When the fuzzer discovers a crash or failure, it saves the triggering input to the `artifacts` directory. Each artifact file contains the exact byte sequence that caused the issue.

## Reproducing Failures

When the fuzzer finds an issue, you can reproduce it using the test infrastructure:

1. The failing input is saved to `artifacts/layer/crash-<hash>`
2. Minimize the input using `cargo fuzz tmin layer artifacts/layer/crash-<hash>`
3. Edit `tests/reproduce.rs` and update the filename:
   ```rust,no_compile
   let bytes = include_bytes!("../artifacts/layer/minimized-from-<hash>");
   ```
4. Run the test:
   ```bash
   cargo test
   ```

This approach allows you to:
- Debug the issue with full Rust tooling (not just nightly)
- Set breakpoints and use a debugger
- Iterate on fixes quickly

Alternatively, you can run the fuzzer directly with the artifact:

```bash
cargo +nightly fuzz run layer artifacts/layer/crash-<hash>
```

## Coverage

To generate coverage information:

```bash
cargo +nightly fuzz coverage layer
```

This creates coverage data in the `coverage` directory, showing which code paths have been exercised during fuzzing.

## Adding New Fuzz Targets

To add a new fuzz target:

1. Create a new file in `fuzz_targets/`:
   ```bash
   cargo fuzz add <target_name>
   ```
2. Implement the fuzz target using `fuzz_target!` macro
3. Add corresponding test infrastructure in `src/lib.rs` and `tests/`

## Further Reading

- [cargo-fuzz documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
- [mlt library documentation](../README.md)
