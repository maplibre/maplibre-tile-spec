# Contributing to Rust MLT

## Development Workflow
All commands must be run from the repository root using [`just`](https://github.com/casey/just).
Do **not** use bare `cargo` commands; `just` recipes ensure the correct feature flags and CI-matching environments.

| Command | Description |
| :--- | :--- |
| `just rust::check` | Fast compilation check (no binaries). |
| `just rust::test` | Run full test suite (including property-based tests). |
| `just rust::fmt` | Format code according to project style. |
| `just rust::lint` | Run `clippy` lints. |
| `just rust::bless` | Regenerate expected `insta` test snapshots. |

## The Five-Stage Data Pipeline
Data in `mlt-core` moves through a strict linear pipeline:

| Stage | Prefix | Ownership | Purpose |
| :--- | :--- | :--- | :--- |
| **1** | `Raw*` | Borrowed (`&'a [u8]`) | Zero-copy views of input bytes. No allocation. |
| **2** | `Parsed*` | Owned (`'static`) | Fully decoded Rust values (e.g., `Vec<u64>`). |
| **3** | `Tile*` | Owned | **Row-oriented** features (e.g., `v01::tile`). Uses `geo_types`. |
| **4** | `Staged*` | Owned | **Columnar** data being prepared for encoding. |
| **5** | `Encoded*` | Owned | Wire-ready byte buffers. |

These are the Conversion Rules:
* **Forward Only:** Data moves $1 \rightarrow 5$. No backwards conversions (e.g., `Encoded` cannot become `Staged`).
* **Parsing:** `Raw*` $\rightarrow$ `Parsed*` via `TryFrom` or `pub fn parse()`.
* **Staging:** `Parsed*` $\rightarrow$ `Staged*` via `Staged*::Decoded(value)`.
* **Encoding:** `Staged*` $\rightarrow$ `Encoded*` via optimizer traits (`Manual`, `Automatic`, or `Profile`).
* **Serialization:** `Encoded*` $\rightarrow$ bytes via `write_to(&mut writer)`.

## Core Types & Abstractions

* **Columnar** (`Raw`, `Parsed`, `Staged`, `Encoded`): The native on-wire format. Used for storage and optimization.
* **Row-Oriented** (`TileLayer01`, `TileFeature`, `PropValue`): Located in `mlt_core::v01::tile`. Used for business logic and user APIs.
* **Transition:** The optimizer handles the conversion between `StagedLayer01` (Columnar) and `TileLayer01` (Row) at the pipeline boundaries.

## Testing Standards
* **Round-trips:** Tests live in `mlt-core/tests/` (e.g., `property_roundtrip.rs`).
* **Property-based:** Use `proptest` for edge-case discovery in codecs.
* **Snapshots:** Use `insta` for output verification. Update with `just rust::bless`.
