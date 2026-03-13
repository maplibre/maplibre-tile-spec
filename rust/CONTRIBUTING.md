# Contributing to Rust MLT

## Development Workflow
All commands must be run from the repository root using [`just`](https://github.com/casey/just).
Do **not** use bare `cargo` commands; `just` recipes ensure the correct feature flags and CI-matching environments.

| Command                              | Description                                                             |
|:-------------------------------------|:------------------------------------------------------------------------|
| `just rust::check`                   | Fast compilation check (no binaries).                                   |
| `just rust::test`                    | Run full test suite (including property-based tests).                   |
| `just rust::fmt`                     | Format code according to project style.                                 |
| `just rust::lint`                    | Run `clippy` lints.                                                     |
| `just rust::generate-synthetic-mlts` | Generate synthetic MLT files for testing. Do not delete files manually. |
| `just rust::bless`                   | Regenerate expected `insta` test snapshots.                             |

## Code Structure
* Most structs and enums live in `model.rs` files, while the specific functionality for those types live in impl blocks of various files like `decoding.rs`, `encoding.rs`, etc. This is not very Rust-idiomatic, but it allows for better organization of the codebase by feature (e.g., all decoding logic in one file, all encoding logic in another file) rather than by type.
* We try to keep encoding and decoding logic separate, as they have different requirements and optimizations. For example, decoding can be more flexible and allow for partial lazy decoding, while encoding needs to operate on owned data structures and may require more complex transformations and optimizations (e.g., reordering features for better compression).

## Decoding Data
When decoding data, `mlt-core` moves through a strict linear pipeline, minimizing unnecessary allocations and copies.  Both raw and parsed data is stored in the container structs (e.g., `TileLayer01`) as variants of the `EncDec<Raw,Parsed>` generic enum to allow for partial lazy decoding for any column like `id`, `geometry`, `property`, `sub-property`. Some internal owned types may be used inside both the `Parsed*` (decoding) and `Staged*` (encoding) data structures.

| Stage | Prefix         | Ownership                | Purpose                                                                       |
|:------|:---------------|:-------------------------|:------------------------------------------------------------------------------|
| **-** | `<Name>`       | Borrowed (`&'a`)         | Container structs to allow partial lazy decoding.                             |
| **1** | `Raw<Name>`    | Borrowed (`&'a`)         | Zero-copy views of input bytes. No allocation.                                |
| **2** | `Parsed<Name>` | Owned or Borrowed (`'a`) | Fully decoded Rust values (e.g., `Vec<u64>`), or could reference input bytes. |
| **3** | `Tile*`        | Owned                    | **Row-oriented** features using `geo_types::Geometry<i32>` for geometries.    |

These are the Conversion Rules:
* **Forward Only:** Data moves `1` -> `3`. No backwards conversions (e.g., `Tile*` cannot become `Parsed*`).
* **Slicing:** Original input bytes are sliced into `Raw*` structures. No copying. Uses `Raw*::parse(&[u8])` -> `MltRefResult<Raw*>` constructors. Not to be confused with `Parsed*` parsing into Parsed stage.
* **Parsing:** `Raw*` `->` `Parsed*` via `TryFrom`.
* **Row-based tiles:** `Parsed*` `->` `Tile*` via `TryFrom`. This is mostly used for GeoJSON generation by CLI and debugging tools, and should probably not be needed for most users like data access via WASM.

## Encoding Data
Encoding is more complex, and requires owned data structures to support optimizations and transformations. The pipeline is as follows:

| Stage | Prefix     | Ownership             | Purpose                                                                    |
|:------|:-----------|:----------------------|:---------------------------------------------------------------------------|
| **1** | `Tile*`    | Owned                 | **Row-oriented** features using `geo_types::Geometry<i32>` for geometries. |
| **2** | `Staged*`  | Owned                 | **Columnar** data being prepared for encoding.                             |
| **3** | `Encoded*` | Owned                 | Wire-ready byte buffers.                                                   |

These are the Conversion Rules:
* **Forward Only:** Data moves `1` -> `5`. No backwards conversions (e.g., `Encoded` cannot become `Staged`).
* All progression steps will use an `Encoder*` types that has some state of **how** to encode, the data of the stage, and return the next stage types, e.g. `TileLayer01` via `Tile01Encoder::encode(&mut self, data: &mut TileLayer01)` -> `Result<StagedLayer01, MltError>` `->` `StagedLayer01`. Note that both `self` and `data` are mutable references, as the encoder may need to mutate internal state and the data being encoded (e.g., for optimizations like reordering features or updating profiling information).
* **Staging:** `Tile*` `->` `Staged*`.
* **Encoding:** `Staged*` `->` `Encoded*`.
* **Serialization:** `Encoded*` `->` bytes via `write_to(&mut writer)`.
* In some use cases, user may want to construct `Staged*` directly (e.g., to generate synthetic MLT files or in benchmarking), and skip the `Tile*` stage.
* There should not be a need to convert from `Parsed*` to `Staged*`, as the former is for decoding and the latter is for encoding. For round trip testing, the workflow should be `Tile*` `->` `Staged*` `->` `Encoded*` `->` `Vec<u8>` buffer `->` `Raw*` `->` `Parsed*` `->` `Tile*`.  It should be possible to compare `Staged*` and `Parsed*` for testing purposes, but not convert from one to another.
