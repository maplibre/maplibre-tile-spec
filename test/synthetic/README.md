# Synthetic MLT Test Fixtures

To verify encoder and decoder implementations across languages, this directory
contains interesting `.mlt` tiles along with their expected logical
representation as JSON.

## Decoding

Any correct decoder should produce identical logical output (json) from the
same `.mlt`, regardless of which encoder wrote it. So Java and TypeScript
decoding a file from `0x01-rust/` should produce the same `.json` as Rust did.

## Where to add a new synthetic

- **`0x0N/`** - Java can encode it. Add to `SyntheticMltGenerator.java`.
- **`0x0N-{rust,java}/`** - Java/rust specific encoding quirks, such as differing fsst implementations
