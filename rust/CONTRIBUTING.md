# Contributing to the Rust MLT Implementation

## Development Commands

All development commands are run from the repository root using [`just`](https://github.com/casey/just):

```bash
just rust::check   # Fast compile check (no binaries produced)
just rust::test    # Run all tests
just rust::fmt     # Format code
just rust::lint    # Run clippy lints
just rust::bless   # Regenerate expected test snapshots
```

Do **not** use bare `cargo` commands for CI-equivalent checks—the `just` recipes apply
the correct feature combinations and flag sets that match the CI pipeline.

---

## Five-Stage Data Pipeline

All data flowing through `mlt-core` follows a strict five-stage pipeline:

```
RawBytes ──► Raw* ──► Parsed* ──► TileLayer* / TileFeature / PropValue ──► Staged* ──► Encoded* ──► RawBytes
```

Each stage has a distinct role and type-naming convention:

| Stage | Prefix | Description |
|-------|--------|-------------|
| 1 | `Raw*` | Zero-copy borrowed views of input bytes (`&'a [u8]`). Parsing only, no allocation. |
| 2 | `Parsed*` | Fully owned, decoded Rust values (e.g. `Vec<Option<u64>>`). Ready for business logic. |
| 3 | `TileLayer01` / `TileFeature` / `PropValue` | Row-oriented feature representation (module `v01::tile`). Uses `geo_types::Geometry<i32>` and `PropValue` enum. |
| 4 | `Staged*` (`StagedLayer01`, `StagedGeometry`, …) | Columnar owned data ready for encoding. Internally a `EncDec<Encoded*, Parsed*>` alias. |
| 5 | `Encoded*` | Wire-ready owned byte buffers. Can be serialized directly to a file or stream. |

### Conversion rules

- `Raw*` → `Parsed*`: via `TryFrom` (fallible, returns `MltError`).
- `Raw*` constructor: `pub fn parse(input: &'a [u8]) -> MltRefResult<'a, Self>`.
- `Parsed*` → `Staged*`: wrap with `Staged*::Decoded(parsed_value)`.
- `Staged*` → `Encoded*`: call optimizer traits (`ManualOptimisation`, `AutomaticOptimisation`, `ProfileOptimisation`).
- `Encoded*` → bytes: call `write_to(&mut writer)` on the `Encoded*` type.
  To get from `Staged*` to `Encoded*`, use `staged.as_encoded()?`.
- **No backwards conversions** — `Parsed*` cannot become `Raw*`, `Encoded*` cannot become `Staged*`.

---

## Naming Conventions

### Type prefixes

| Prefix | Lifetime         | Owns data? | Purpose                                        |
|--------|------------------|------------|------------------------------------------------|
| `Raw*` | `'a` (borrowed)  | No         | Parsed from raw bytes; holds `&'a [u8]` slices |
| `Parsed*` | `'static` (owned) | Yes        | Decoded Rust values                            |
| `Staged*` | owned            | Yes        | Columnar data being prepared for encoding      |
| `Encoded*` | owned            | Yes        | Wire-ready byte buffers                        |

### Concrete examples

```rust
// Borrowed, zero-copy parse result
pub struct RawId<'a> {
    pub(crate) presence: Option<RawStream<'a>>,
    pub(crate) value: RawIdValue<'a>,
}

// Fully decoded, owned
pub struct ParsedId(pub Vec<Option<u64>>);

// Wire-ready, owned
pub struct EncodedId {
    pub(crate) presence: Option<EncodedStream>,
    pub(crate) value: EncodedIdValue,
}

// Dual-state alias used during the encoding pipeline
pub type StagedId = EncDec<EncodedId, ParsedId>;

// Dual-state alias used during parsing (borrowed)
pub type Id<'a> = EncDec<RawId<'a>, ParsedId>;
```

### Stream types

| Name | Description |
|------|-------------|
| `RawStream<'a>` | Borrowed stream with metadata and a raw byte slice |
| `EncodedStream` | Owned stream with metadata and an allocated byte buffer |

### Property types

Property encoding mirrors the same prefix convention:

- `RawProperty<'a>` / `EncodedProperty` / `ParsedProperty<'a>`
- `RawScalar<'a>` / `EncodedScalar` / `ParsedScalar<T>`
- `RawStrings<'a>` / `EncodedStrings` / `ParsedStrings<'a>`
- `RawSharedDict<'a>` / `EncodedSharedDict` / `ParsedSharedDict`
- `RawName<'a>` / `EncodedName`
- `RawPresence<'a>` / `EncodedPresence`

---

## The `EncDec<E, D>` Enum

`EncDec<Encoded, Decoded>` is a general-purpose two-variant enum:

```rust
pub enum EncDec<E, D> {
    Encoded(E),
    Decoded(D),
}
```

It is used in two distinct roles:

1. **Parsing** — `Id<'a> = EncDec<RawId<'a>, ParsedId>` and similar, to hold data in either
   raw-bytes form or fully decoded form, enabling lazy decoding.

2. **Staging** — `StagedId = EncDec<EncodedId, ParsedId>` and similar, used by the optimizer
   pipeline to hold data that may be decoded (awaiting encoding) or already encoded (ready to
   write).

Do not introduce new uses of `EncDec` without first checking whether a simpler owned struct
would suffice.

---

## Row vs. Columnar Data

- **Columnar** (`Raw*`, `Parsed*`, `Staged*`, `Encoded*`): data is stored column-by-column.
  This is the native on-wire format and the format used by the optimizer.
- **Row-oriented** (`TileLayer01`, `TileFeature`, `PropValue`): data is stored feature-by-feature,
  living in `mlt_core::v01::tile`. This is the convenient working form for business logic, sorting,
  and user-facing APIs. Geometry is `geo_types::Geometry<i32>`, and individual property values use
  the `PropValue` enum. `SharedDict` columns are flattened: each sub-field becomes its own
  `PropValue::Str` entry.

The optimizer converts `StagedLayer01` ↔ `TileLayer01` at its entry and exit boundaries. Users
working at the tile level interact only with `TileLayer01`/`TileFeature`/`PropValue`.

---

## Testing

- Use `just rust::test` to run the full test suite (including all feature combinations).
- Use `just rust::check` for a fast compile check without running tests.
- Round-trip tests live in `mlt-core/tests/` (e.g. `property_roundtrip.rs`).
- Property-based tests use [`proptest`](https://github.com/proptest-rs/proptest).
- Snapshot tests use [`insta`](https://insta.rs/). Run `just rust::bless` to update snapshots.

---

## Adding a New Column Type

When adding a new column type (e.g. `Foo`), follow this checklist:

1. **`model.rs`**: Define `RawFoo<'a>`, `ParsedFoo`, `EncodedFoo`, and the type aliases
   `Foo<'a> = EncDec<RawFoo<'a>, ParsedFoo>` and `StagedFoo = EncDec<EncodedFoo, ParsedFoo>`.
2. **`owned.rs`**: Implement `ToOwned`/`to_owned()` on `RawFoo<'_>` returning `EncodedFoo`.
3. **`codec.rs`** / **`decode.rs`**: Implement `Decode<RawFoo<'a>>` for `ParsedFoo` and
   `impl TryFrom<RawFoo<'a>> for ParsedFoo`.
4. **`encode.rs`**: Implement `FromDecoded` for `EncodedFoo` (converts `ParsedFoo` → `EncodedFoo`).
5. **`serialize.rs`**: Implement `write_to` on `EncodedFoo` and `parse` on `RawFoo<'a>`.
6. **`analyze.rs`**: Implement `Analyze` for `EncodedFoo`.
7. **`optimizer.rs`**: Implement the three optimizer traits for `StagedFoo`.
8. **`mod.rs`**: Re-export all new public types.
