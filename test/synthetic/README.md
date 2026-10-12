# Synthetic MLT Test Fixtures

To verify encoder and decoder implementations across languages, this directory
contains interesting `.mlt` tiles along with their expected logical
representation as JSON.

## Decoding

Any correct decoder should produce identical logical output (json) from the
same `.mlt`, regardless of which encoder wrote it. So TypeScript
decoding a file from `0x01-rust/` should produce the same `.json` as Rust did.

## Where to add a new synthetic

- **`0x0N/`** - Frozen reference tiles. The Java generator that wrote `0x01/` is gone, so Rust verifies against these instead of regenerating them.
- **`0x0N-rust/`** - Rust specific encoding quirks, such as differing fsst implementations
