just := quote(just_executable())

_default: (just '--list' 'inspector')

[private]
just *args:
    {{just}} {{args}}

# Fail before the bundler does when the workspace mlt-wasm the app links against is missing
[private]
assert-wasm-built:
    @test -f ../rust/mlt-wasm/dist/index.js \
      || (echo 'the app links @maplibre/mlt-wasm from the workspace: run "just rust::wasm-build" first' && exit 1)

# Build the mlt CLI the fixture index reads geometry and encoding facets from
[private]
build-mlt:
    cargo build --release --manifest-path ../rust/Cargo.toml --package mlt

# Update the committed vitest inline snapshots, which `test` only verifies
bless: install build-mlt
    npm run test:bless

# Build the app into docs/inspector/app
build: install assert-wasm-built build-mlt
    npm run build

# Run all CI steps: lint, test, build
ci-test: lint test build
    {{just}} assert-git-is-clean

# Delete build artifacts
clean:
    rm -rf node_modules ../docs/inspector/app

# Serve the app with the fixtures it indexed
dev: install assert-wasm-built build-mlt
    npm run dev

# Re-encode the real-world tiles from their MVT sources with every size feature on
regenerate-expected: build-mlt
    for dir in amazon amazon_here bing omt simple; do \
      ../rust/target/release/mlt convert --mlt-version 1 --sort none --verify ../test/fixtures/$dir ../test/expected/0x01/$dir; \
      ../rust/target/release/mlt convert --mlt-version 2 --sort none --delta2 --packed-dict-codes --rans-vertices --verify ../test/fixtures/$dir ../test/expected/0x02/$dir; \
    done

# Reformat code
fmt: install
    npm run format

# Install dependencies
install:
    npm ci

# Run linting
lint: install
    npm run lint

# Run tests
test: install build-mlt
    npm run test
