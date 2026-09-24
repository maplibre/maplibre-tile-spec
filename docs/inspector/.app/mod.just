just := quote(just_executable())

_default: (just '--list' 'inspector')

[private]
just *args:
    {{just}} {{args}}

# Fail before the bundler does when the workspace mlt-wasm the app links against is missing
[private]
assert-wasm-built:
    @test -f ../../../rust/mlt-wasm/dist/index.js \
      || (echo 'the app links @maplibre/mlt-wasm from the workspace: run "just rust::wasm-build" first' && exit 1)

# Build the mlt CLI the fixture index reads geometry and encoding facets from
[private]
build-mlt:
    cargo build --release --manifest-path ../../../rust/Cargo.toml --package mlt --features unstable-v2

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
    rm -rf node_modules ../app

# Serve the app with the fixtures it indexed
dev: install assert-wasm-built build-mlt
    npm run dev

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
