main_crate := 'mlt-core'
just := quote(just_executable())

# if running in CI, treat warnings as errors by setting RUSTFLAGS and RUSTDOCFLAGS to '-D warnings' unless they are already set
# Use `CI=true just ci-test` to run the same tests as in GitHub CI.
# Use `just ci-env-info` to see the current values of RUSTFLAGS and RUSTDOCFLAGS
ci_mode := if env('CI', '') != '' {'1'} else {''}
export RUSTFLAGS := env('RUSTFLAGS', if ci_mode == '1' {'-D warnings'} else {''})
export RUSTDOCFLAGS := env('RUSTDOCFLAGS', if ci_mode == '1' {'-D warnings'} else {''})
export RUST_BACKTRACE := env('RUST_BACKTRACE', if ci_mode == '1' {'1'} else {'0'})

# Features excluded from cargo-hack --each-feature iterations (heavy optional instrumentation)
hack_skip := '--exclude-features __hotpath,__hotpath-alloc,__hotpath-mcp'

_default: (just '--list' 'rust')

[private]
just *args:
    {{just}} {{args}}

# Run integration tests and save its output as the new expected output
bless *args: (just 'cargo-install' 'cargo-insta') (just 'cargo-install' 'cargo-hack')
    cargo hack insta test --accept --unreferenced=delete --each-feature {{hack_skip}} \
        --package mlt-core --exclude-no-default-features --exclude-all-features \
        {{args}}

# Run benchmarks
bench *args: (just 'cargo-install' 'cargo-hack')
    cargo hack bench --all-targets --each-feature {{hack_skip}} \
        --package mlt-core --exclude-no-default-features --exclude-all-features \
        {{args}}

# Run a specific benchmark target
bench-one target='encoding_from_mvt': (just 'cargo-install' 'cargo-hack')
    cargo bench --bench {{target}} -p mlt-core --features __private

alias b := build
# Build the project
build: (just 'cargo-install' 'cargo-hack')
    cargo hack build --all-targets --each-feature {{hack_skip}} \
        --workspace --exclude-no-default-features --exclude-all-features

alias c := check
# Quick compile without building a binary
check: (just 'cargo-install' 'cargo-hack')
    cargo hack check --all-targets --each-feature {{hack_skip}} \
        --workspace --exclude-no-default-features --exclude-all-features
    cd mlt-core/fuzz && cargo check --all-features

# Generate code coverage report to upload to codecov.io
ci-coverage: ci-env-info (just 'java::gen-snippets') (just 'cargo-install' 'cargo-llvm-cov')
    mkdir -p target/llvm-cov

    @echo "::group::Generate synthetic MLTs"
    rm -rf ../test/synthetic/0x01-rust
    cargo llvm-cov run --no-report -p mlt-synthetics
    @echo "::endgroup::"

    @echo "::group::Validate synthetic MLTs"
    {{just}} rust::validate-synthetic
    @echo "::endgroup::"

    # ATTENTION: the output-path here is used in the CI workflow
    # --no-clean preserves profdata accumulated by the synthetic generation run above
    # --doctests requires nightly (test job uses nightly toolchain) and may not be mixed with the main
    {{just}} rust::coverage '--no-clean'
    # target coverage and doctest coverage cannot be mixed
    cargo llvm-cov --doctests --no-clean --workspace --lcov --output-path target/llvm-cov/lcov.info

# Run minimal subset of tests to ensure compatibility with MSRV
ci-check: ci-env-info check

# Print environment info for CI
ci-env-info:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "::group::General environment Information"
    echo "Running for '{{main_crate}}' crate {{if ci_mode == '1' {'in CI mode'} else {'in dev mode'} }} on {{os()}} / {{arch()}}"
    echo "PWD {{justfile_directory()}}"
    {{just}} --version
    rustc --version
    cargo --version
    rustup --version
    echo "RUSTFLAGS='$RUSTFLAGS'"
    echo "RUSTDOCFLAGS='$RUSTDOCFLAGS'"
    echo "RUST_BACKTRACE='$RUST_BACKTRACE'"
    echo "::endgroup::"

    # Linux
    if command -v lscpu &>/dev/null; then
        echo "::group::lscpu"
        lscpu
        echo "::endgroup::"
    fi
    if [ -f /proc/cpuinfo ]; then
        echo "::group::/proc/cpuinfo"
        cat /proc/cpuinfo
        echo "::endgroup::"
    fi

    # MacOS
    if command -v sysctl &>/dev/null && command -v egrep &>/dev/null; then
        echo "::group::sysctl"
        sysctl -a 2>/dev/null | egrep "^hw" || true # ignore permission denied to some sysctl entries
        echo "::endgroup::"
    fi

# Run linting as expected by CI
ci-lint: ci-env-info test-fmt clippy ci-docs-rs
    {{just}} assert-git-is-clean  # call it explicitly to avoid just optimizing it to just one call

# Run all tests as expected by CI
ci-test: ci-env-info (just 'java::gen-snippets') test
    {{just}} assert-git-is-clean  # call it explicitly to avoid just optimizing it to just one call

# Run all tests as expected by CI for wasm
ci-test-wasm: ci-env-info wasm-test
    {{just}} assert-git-is-clean  # call it explicitly to avoid just optimizing it to just one call

# Run minimal subset of tests to ensure compatibility with MSRV
ci-test-msrv: ci-env-info
    #!/usr/bin/env bash
    set -euo pipefail
    # For MSRV, relax all warnings to avoid different warnings per rust version
    unset CI RUSTFLAGS RUSTDOCFLAGS RUST_BACKTRACE
    # Pre-install cargo-hack to avoid CI binstall issues
    {{just}} cargo-install cargo-hack
    {{just}} rust::ci-check
    {{just}} java::gen-snippets
    {{just}} rust::test
    {{just}} assert-git-is-clean

# Build the layer target (installs `cargo fuzz`)
ci-fuzz-build target: (cargo-fuzz 'build' target '--target' 'x86_64-unknown-linux-gnu')

# Fuzz the target for 90 seconds (installs `cargo fuzz`)
ci-fuzz-run target: (cargo-fuzz 'run' target '--target' 'x86_64-unknown-linux-gnu' '--' '-max_total_time=90' '-max_len=300' '-rss_limit_mb=2048')

# Run coverage against a previously generated corpus and export LCOV for Codecov upload.
# Output: target/llvm-cov/fuzz-<target>.lcov
[working-directory: 'mlt-core']
ci-fuzz-cov target: (just 'cargo-install' 'cargo-fuzz')
    #!/usr/bin/env bash
    set -euo pipefail

    LCOV_OUT="../target/llvm-cov/fuzz-{{target}}.lcov"
    mkdir -p "$(dirname "$LCOV_OUT")"
    {{just}} rust::fuzz-llvm-cov {{target}} "$LCOV_OUT" export --format=lcov
    echo "Fuzz coverage LCOV: $LCOV_OUT"

# Build and package release binary for CI
ci-build-release target artifact_name:
    #!/usr/bin/env bash
    set -euo pipefail
    rustup target add {{target}}
    cargo build --release --target {{target}} --package mlt --bin mlt
    cd target/{{target}}/release
    if [ -f mlt.exe ]; then
        7z a ../../../../{{artifact_name}} mlt.exe
    else
        tar -czf ../../../../{{artifact_name}} mlt
    fi

# Output version and tag for GITHUB_OUTPUT (append with >> $GITHUB_OUTPUT)
ci-get-release-info package='mlt':
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION=$({{just}} rust::get-crate-field version {{package}})
    echo "version=$VERSION"
    echo "tag=rust-{{package}}-v$VERSION"

# Clean all build artifacts
clean:
    cargo clean
    rm -f Cargo.lock

# Run cargo clippy to lint the code
clippy *args: (just 'cargo-install' 'cargo-hack')
    cargo clippy --all-targets --all-features --workspace {{args}}
    cd mlt-core/fuzz && cargo clippy --all-features {{args}}

# Generate code coverage report. Installs `cargo llvm-cov`
coverage *args='--no-clean --open': (just 'cargo-install' 'cargo-hack') (just 'cargo-install' 'cargo-llvm-cov')
    cargo hack llvm-cov --all-targets --each-feature {{hack_skip}} \
        --workspace --exclude-no-default-features --exclude-all-features \
        --include-build-script {{args}}

# Run the encoder with hotpath profiling (timing + allocation tracking)
profile *args:
    cargo run --release -p mlt --features='__hotpath,__hotpath-alloc,__hotpath-mcp' -- {{args}}

# Download ~5k benchmark tiles (z0-z6) from Protomaps into outdir.
# Skips if a previous complete download is detected (.complete sentinel).
# ODbL-licensed OSM data; extraction explicitly permitted by Protomaps.
download-benchmark-tiles outdir='/tmp/benchmark-tiles' maxzoom='6': (just 'install-pmtiles')
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -f "{{outdir}}/.complete" ]; then
        echo "{{outdir}} already has tiles, skipping download"
        exit 0
    fi
    TMPFILE=$(mktemp /tmp/benchmark-XXXXXX.pmtiles)
    trap 'rm -f "$TMPFILE"' EXIT
    pmtiles extract \
        https://data.source.coop/protomaps/openstreetmap/v4.pmtiles \
        "$TMPFILE" \
        --maxzoom={{maxzoom}}
    python3 - "{{outdir}}" "{{maxzoom}}" "$TMPFILE" << 'PYEOF'
    import gzip, os, sys
    from pmtiles.reader import Reader, MmapSource, Compression

    out, maxzoom, src = sys.argv[1], int(sys.argv[2]), sys.argv[3]
    os.makedirs(out, exist_ok=True)
    count = 0
    with open(src, "rb") as f:
        reader = Reader(MmapSource(f))
        is_gzip = reader.header()["tile_compression"] == Compression.GZIP
        for z in range(maxzoom + 1):
            for x in range(2 ** z):
                for y in range(2 ** z):
                    data = reader.get(z, x, y)
                    if data:
                        if is_gzip:
                            data = gzip.decompress(data)
                        with open(os.path.join(out, f"{z}_{x}_{y}.mvt"), "wb") as of:
                            of.write(data)
                        count += 1
    print(f"Extracted {count} tiles to {out}")
    PYEOF
    touch "{{outdir}}/.complete"

# Build and open code documentation
docs *args='--open':
    DOCS_RS=1 cargo doc --no-deps {{args}} --features arbitrary --workspace

alias f := fmt
fmt: (_fmt) (_fmt '--manifest-path' 'mlt-core/fuzz/Cargo.toml')

# Reformat code. If nightly is available, use it for better results.
_fmt *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    if (rustup toolchain list | grep nightly && rustup component list --toolchain nightly | grep rustfmt) &> /dev/null; then
        echo 'Reformatting Rust code using nightly Rust fmt to sort imports'
        cargo +nightly fmt --all {{ARGS}} -- --config imports_granularity=Module,group_imports=StdExternalCrate
    else
        echo 'Reformatting Rust with the stable cargo fmt.  Install nightly with `rustup install nightly` for better results'
        cargo fmt --all {{ARGS}}
    fi

# Reformat all Cargo.toml files (installs `cargo sort`)
fmt-toml *args: (just 'cargo-install' 'cargo-sort')
    cargo sort --workspace --grouped {{args}}

# Fuzz the layer target (installs `cargo fuzz`)
fuzz target='layer' *args: (cargo-fuzz 'run' target '--target' 'x86_64-unknown-linux-gnu' '--' args)

# Seed the layer fuzz corpus from all .mlt test files (run once, or after new .mlts are added)
fuzz-seed target='layer': (just 'cargo-install' 'fd' 'fd-find')
    mkdir -p mlt-core/fuzz/corpus/{{target}}
    fd . ../test --extension mlt --threads=1 --exec cp --force {} mlt-core/fuzz/corpus/{{target}}/
    {{just}} rust::fuzz-cmin {{target}}

# Minimize the fuzz corpus for target (slow — run occasionally to prune redundant entries)
fuzz-cmin target='layer': (cargo-fuzz 'cmin' target '--target' 'x86_64-unknown-linux-gnu')

# Collect coverage from the fuzz corpus and open an HTML report (installs `cargo fuzz`, `rustfilt`)
[working-directory: 'mlt-core']
fuzz-cov target='layer': (just 'cargo-install' 'cargo-fuzz') (just 'cargo-install' 'rustfilt')
    #!/usr/bin/env bash
    set -euo pipefail

    HTML_DIR="fuzz/coverage/{{target}}/html"
    mkdir -p "$HTML_DIR"
    {{just}} rust::fuzz-llvm-cov {{target}} "" show \
        --format=html \
        --output-dir="$HTML_DIR" \
        --show-instantiations \
        --show-line-counts-or-regions \
        --Xdemangler=rustfilt
    echo "Coverage report: $HTML_DIR/index.html"
    xdg-open "$HTML_DIR/index.html" 2>/dev/null || open "$HTML_DIR/index.html" 2>/dev/null || true

# Minimize the crash in file (installs `cargo fuzz`)
fuzz-tmin file target='layer': (cargo-fuzz 'tmin' target file)

# Get any package's field from the metadata
get-crate-field field package=main_crate: (just 'assert-cmd' 'jq')
    cargo metadata --format-version 1 | jq -e -r '.packages | map(select(.name == "{{package}}")) | first | .{{field}} | select(. != null)'

# Get the minimum supported Rust version (MSRV) for the crate
get-msrv package=main_crate: (get-crate-field 'rust_version' package)

# Run formatting, cargo check, and clippy for local development
lint: fmt check clippy test

# Find the minimum supported Rust version and update Cargo.toml
msrv: (just 'cargo-install' 'cargo-msrv')
    cargo msrv find --write-msrv --ignore-lockfile --component rustfmt -- {{just}} rust::ci-test-msrv

# Build mlt-py and launch a Python interpreter with mlt pre-imported
[working-directory: 'mlt-py']
py: (just 'cargo-install' 'maturin')
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ ! -d ".venv" ]]; then
        echo "Please make sure to create your virtual environment by running 'python -m venv .venv'"
        exit 1
    fi
    source .venv/bin/activate
    maturin develop
    python3 -i -c "import maplibre_tiles; print('maplibre_tiles ready — try: maplibre_tiles.decode_mlt(data)')"

py-publish:
    docker run --rm -v $(pwd):/io ghcr.io/pyo3/maturin build --release

# Run cargo-release
release *args='': (just 'cargo-install' 'release-plz')
    release-plz {{args}}

# Check semver compatibility with prior published version
semver *args: (just 'cargo-install' 'cargo-semver-checks') (just 'cargo-install' 'cargo-hack')
    cargo hack semver-checks --each-feature {{hack_skip}} \
        --package mlt-core --exclude-no-default-features --exclude-all-features \
        {{args}}

# Regenerate the Python type stub for mlt-py
sync-pyo3-stubs:
    cargo run -p mlt-py --bin stub_gen --features abi3

# Sync the version field in mlt-py/pyproject.toml to match mlt-py/Cargo.toml
sync-pyo3-version:
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION=$({{just}} rust::get-crate-field version mlt-py)
    PYPROJECT="mlt-py/pyproject.toml"
    sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$PYPROJECT"
    echo "Synced $PYPROJECT to version $VERSION"

# Sync the version field in mlt-wasm/package.json to match mlt-wasm/Cargo.toml
sync-wasm-version:
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION=$({{just}} rust::get-crate-field version mlt-wasm)
    PACKAGE_JSON="mlt-wasm/package.json"
    jq --arg VERSION "$VERSION" '.version = $VERSION' "$PACKAGE_JSON" > "$PACKAGE_JSON.tmp" && mv "$PACKAGE_JSON.tmp" "$PACKAGE_JSON"
    echo "Synced $PACKAGE_JSON to version $VERSION"
    if [ -f "mlt-wasm/package-lock.json" ]; then
        PACKAGE_LOCK="mlt-wasm/package-lock.json"
        jq --arg VERSION "$VERSION" '.version = $VERSION | .packages[""].version = $VERSION' "$PACKAGE_LOCK" > "$PACKAGE_LOCK.tmp" && mv "$PACKAGE_LOCK.tmp" "$PACKAGE_LOCK"
        echo "Synced $PACKAGE_LOCK to version $VERSION"
    fi

alias t := test
# Run all tests
test: (just 'cargo-install' 'cargo-hack')
    cargo hack test --all-targets --each-feature {{hack_skip}} \
        --workspace --exclude-no-default-features --exclude-all-features
    {{just}} rust::generate-synthetic-mlts
    {{just}} rust::test-doc

# Test documentation generation
test-doc: (docs '--document-private-items')
    cargo test --doc --features arbitrary --package mlt-core

# Simulate docs.rs for all workspace crates and verify docs can build offline.
ci-docs-rs:
    #!/usr/bin/env bash
    set -euo pipefail
    rustup toolchain install nightly --profile minimal
    export DOCS_RS=1
    export RUSTDOCFLAGS="${RUSTDOCFLAGS:+$RUSTDOCFLAGS } --cfg docsrs"
    cargo fetch --locked
    CARGO_NET_OFFLINE=true cargo +nightly doc --workspace --no-deps --locked

# Test code formatting
test-fmt: (fmt-toml '--check' '--check-format')
    cargo fmt --all -- --check

# Test workspace publishing with --dry-run
test-publish:
    cargo publish --workspace --dry-run

# Find unused dependencies
udeps: (just 'cargo-install' 'cargo-udeps')
    cargo +nightly udeps --all-targets --workspace --all-features

# Update all dependencies, including breaking changes
update:
    cargo +nightly -Z unstable-options update --breaking
    cargo update
    cd mlt-core/fuzz && cargo +nightly -Z unstable-options update --breaking
    cd mlt-core/fuzz && cargo update

# Generate synthetic .mlt files and ensure there are no duplicates
generate-synthetic-mlts *args:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf ../test/synthetic/0x01-rust
    cargo run -p mlt-synthetics -- {{args}}
    {{just}} rust::validate-synthetic
    {{just}} _assert-all-mlt-files-different

# Create .dump files for all .mlt files in test/synthetic for all languages
generate-synthetic-dumps: (just 'cargo-install' 'fd' 'fd-find')
    cargo build --bin mlt
    fd . ../test/synthetic -e mlt -x sh -c 'target/debug/mlt dump {} > {.}.dump || echo "Failed to dump {}"'

# Run mlt-wasm benches
[working-directory: 'mlt-wasm']
wasm-bench: wasm-build
    npm run bench

# Build the mlt-wasm package (compiles Rust to WASM and TypeScript)
[working-directory: 'mlt-wasm']
wasm-build: (just 'cargo-install' 'wasm-pack')
    npm ci --no-fund --no-audit
    npm run build

# Run mlt-wasm tests
[working-directory: 'mlt-wasm']
wasm-test: wasm-build
    npm run test

# Validate every .mlt in test/synthetic against corresponding .json (GeoJSON)
validate-synthetic:
    {{just}} mlt ls --validate-to-json --details basic ../test/synthetic/

[private]
[working-directory: 'mlt-core']
cargo-fuzz *args:  (just 'cargo-install' 'cargo-fuzz')
    cargo +nightly fuzz {{args}}

# Resolve llvm-cov from the nightly sysroot and run it against the fuzz profdata for target.
# Pass output_file="" to write to stdout, or a path to redirect stdout there.
[private]
[working-directory: 'mlt-core']
fuzz-llvm-cov target output_file *args:
    #!/usr/bin/env bash
    set -euo pipefail

    rustup component add llvm-tools-preview --toolchain nightly
    cargo +nightly fuzz coverage {{target}} --target x86_64-unknown-linux-gnu

    SYSROOT=$(rustup run nightly rustc --print sysroot)
    HOST=$(rustup run nightly rustc -vV | sed -n 's/^host: //p')
    LLVM_COV="$SYSROOT/lib/rustlib/$HOST/bin/llvm-cov"
    FUZZ_DIR="fuzz"
    TARGET_BIN=$(find target -type f -name '{{target}}' -path '*/coverage/*/release/{{target}}' 2>/dev/null | head -1)
    echo "target_bin: $TARGET_BIN" >&2
    PROFDATA="$FUZZ_DIR/coverage/{{target}}/coverage.profdata"
    [ -n "{{output_file}}" ] && exec > "{{output_file}}"
    "$LLVM_COV" {{args}} "$TARGET_BIN" \
        --instr-profile="$PROFDATA" \
        --ignore-filename-regex="(/.cargo/registry|/rustup/toolchains)"
