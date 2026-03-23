main_crate := 'mlt-core'
just := quote(just_executable())

# if running in CI, treat warnings as errors by setting RUSTFLAGS and RUSTDOCFLAGS to '-D warnings' unless they are already set
# Use `CI=true just ci-test` to run the same tests as in GitHub CI.
# Use `just ci-env-info` to see the current values of RUSTFLAGS and RUSTDOCFLAGS
ci_mode := if env('CI', '') != '' {'1'} else {''}
export RUSTFLAGS := env('RUSTFLAGS', if ci_mode == '1' {'-D warnings'} else {''})
export RUSTDOCFLAGS := env('RUSTDOCFLAGS', if ci_mode == '1' {'-D warnings'} else {''})
export RUST_BACKTRACE := env('RUST_BACKTRACE', if ci_mode == '1' {'1'} else {'0'})

_default: (just '--list' 'rust')

[private]
just *args:
    {{just}} {{args}}

# Run integration tests and save its output as the new expected output
bless *args: (just 'cargo-install' 'cargo-insta') (just 'cargo-install' 'cargo-hack')
    cargo hack insta test --accept --unreferenced=delete --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-cpp,default,__private \
        --exclude-all-features \
        --features fastpfor-rust \
        {{args}}

# Run benchmarks
bench *args: (just 'cargo-install' 'cargo-hack')
    cargo hack bench --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-cpp,default \
        --exclude-all-features \
        --features fastpfor-rust \
        {{args}}

alias b := build
# Build the project
build: (just 'cargo-install' 'cargo-hack')
    cargo hack build --all-targets --each-feature \
        --workspace --exclude mlt-core \
        --exclude-no-default-features --exclude-all-features
    cargo hack build --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-rust,default,__private \
        --exclude-all-features \
        --features fastpfor-cpp
    cargo hack build --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-cpp,default,__private \
        --exclude-all-features \
        --features fastpfor-rust

alias c := check
# Quick compile without building a binary
check: (just 'cargo-install' 'cargo-hack')
    cargo hack check --all-targets --each-feature \
        --workspace --exclude mlt-core \
        --exclude-no-default-features --exclude-all-features
    cargo hack check --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-rust,default,__private \
        --exclude-all-features \
        --features fastpfor-cpp
    cargo hack check --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-cpp,default,__private \
        --exclude-all-features \
        --features fastpfor-rust
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
    {{just}} _assert-all-mlt-files-different test/synthetic/0x01-rust
    {{just}} rust::_verify-synthetic-equality
    {{just}} rust::_assert-synthetic-mlt-identity
    @echo "::endgroup::"

    # ATTENTION: the output-path here is used in the CI workflow
    # --no-clean preserves profdata accumulated by the synthetic generation run above
    # --doctests requires nightly (test job uses nightly toolchain) and may not be mixed with the main
    {{just}} rust::coverage '--no-clean'
    # target coverage and docktest coverage cannot be mixed
    cargo llvm-cov --doctests --no-clean --workspace --codecov --output-path target/llvm-cov/codecov.info

# Run minimal subset of tests to ensure compatibility with MSRV
ci-check: ci-env-info check

# Print environment info for CI
ci-env-info:
    @echo "::group::General environment Information"
    @echo "Running for '{{main_crate}}' crate {{if ci_mode == '1' {'in CI mode'} else {'in dev mode'} }} on {{os()}} / {{arch()}}"
    @echo "PWD {{justfile_directory()}}"
    {{just}} --version
    rustc --version
    cargo --version
    rustup --version
    @echo "RUSTFLAGS='$RUSTFLAGS'"
    @echo "RUSTDOCFLAGS='$RUSTDOCFLAGS'"
    @echo "RUST_BACKTRACE='$RUST_BACKTRACE'"
    @echo "::endgroup::"

    @echo "::group::lscpu"
    lscpu
    @echo "::endgroup::"

    @echo "::group::/proc/cpuinfo"
    cat /proc/cpuinfo
    @echo "::endgroup::"

# Run linting as expected by CI
ci-lint: ci-env-info test-fmt clippy
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
ci-fuzz-run target: (cargo-fuzz 'run' target '--target' 'x86_64-unknown-linux-gnu' '--' '-max_total_time=90' '--max_len=300')

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
    cargo hack clippy --all-targets --each-feature \
        --workspace --exclude mlt-core \
        --exclude-no-default-features --exclude-all-features \
        {{args}}
    cargo hack clippy --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-rust,default,__private \
        --exclude-all-features \
        --features fastpfor-cpp \
        {{args}}
    cargo hack clippy --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-cpp,default,__private \
        --exclude-all-features \
        --features fastpfor-rust \
        {{args}}
    cd mlt-core/fuzz && cargo clippy --all-features {{args}}

# Generate code coverage report. Installs `cargo llvm-cov`
coverage *args='--no-clean --open': (just 'cargo-install' 'cargo-hack') (just 'cargo-install' 'cargo-llvm-cov')
    cargo hack llvm-cov --all-targets --each-feature \
        --workspace --exclude mlt-core \
        --exclude-no-default-features --exclude-all-features \
        --include-build-script {{args}}
    cargo hack llvm-cov --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-rust,default,__private \
        --exclude-all-features \
        --features fastpfor-cpp \
        --include-build-script {{args}}
    cargo hack llvm-cov --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-cpp,default,__private \
        --exclude-all-features \
        --features fastpfor-rust \
        --include-build-script {{args}}

# Build and open code documentation
docs *args='--open':
    DOCS_RS=1 cargo doc --no-deps {{args}} --features fastpfor-cpp,arbitrary --package mlt-core

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

# Run linting (clippy + format check)
lint: clippy test-fmt

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
    cargo hack semver-checks --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-rust,default,__private \
        --exclude-all-features \
        --features fastpfor-cpp \
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
    cargo hack test --all-targets --each-feature \
        --workspace --exclude mlt-core \
        --exclude-no-default-features --exclude-all-features
    cargo hack test --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-rust,default,__private \
        --exclude-all-features \
        --features fastpfor-cpp
    cargo hack test --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-cpp,default,__private \
        --exclude-all-features \
        --features fastpfor-rust
    {{just}} rust::generate-synthetic-mlts
    {{just}} rust::test-doc

# Test documentation generation
test-doc: (docs '--document-private-items')
    cargo test --doc --features fastpfor-cpp,arbitrary --package mlt-core

# Test documentation generation as it would be built on docs.rs (docsrs cfg, fastpfor-rust only)
ci-test-doc:
    DOCS_RS=1 RUSTDOCFLAGS="${RUSTDOCFLAGS} --cfg docsrs" cargo doc --no-deps --no-default-features --features fastpfor-rust,arbitrary --package mlt-core

# Test code formatting
test-fmt: (fmt-toml '--check' '--check-format')
    cargo fmt --all -- --check

# Test workspace publishing with --dry-run
test-publish:
    cargo publish --workspace --dry-run

# Find unused dependencies
udeps: (just 'cargo-install' 'cargo-udeps') (just 'cargo-install' 'cargo-hack')
    cargo hack +nightly udeps --all-targets --each-feature \
        --workspace --exclude mlt-core \
        --exclude-no-default-features --exclude-all-features
    cargo hack +nightly udeps --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-rust,default,__private \
        --exclude-all-features \
        --features fastpfor-cpp
    cargo hack +nightly udeps --all-targets --feature-powerset \
        --package mlt-core \
        --exclude-features fastpfor-cpp,default,__private \
        --exclude-all-features \
        --features fastpfor-rust

# Update all dependencies, including breaking changes
update:
    cargo +nightly -Z unstable-options update --breaking
    cargo update

# Generate synthetic .mlt files and ensure there are no duplicates
generate-synthetic-mlts:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf ../test/synthetic/0x01-rust
    cargo run -p mlt-synthetics
    {{just}} rust::validate-synthetic
    {{just}} _assert-all-mlt-files-different test/synthetic/0x01-rust
    {{just}} rust::_verify-synthetic-equality
    {{just}} rust::_assert-synthetic-mlt-identity

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

# Assert each .mlt in compare_dir exists in ref_dir and is binary-identical. Skip '_rust' files.
_assert-synthetic-mlt-identity ref_dir='../test/synthetic/0x01' compare_dir='../test/synthetic/0x01-rust':
    #!/usr/bin/env bash
    set -euo pipefail
    # Ensure compare_dir exists.
    if [[ ! -d "{{compare_dir}}" ]]; then
        echo "::error::Compare directory {{compare_dir}} does not exist"
        exit 1
    fi

    # Expand .mlt files; fail if none are found to avoid masking missing/failed generation.
    shopt -s nullglob
    mlt_files=( "{{compare_dir}}"/*.mlt )
    shopt -u nullglob
    if (( ${#mlt_files[@]} == 0 )); then
        echo "::error::No .mlt files found in {{compare_dir}}"
        exit 1
    fi

    for mlt in "${mlt_files[@]}"; do
        base=$(basename "$mlt")
        if [[ "$base" == *"-rust.mlt" ]]; then
            continue
        fi
        ref="{{ref_dir}}/$base"
        if [[ ! -f "$ref" ]]; then
            echo "::error::File $mlt has no counterpart in {{ref_dir}}"
            exit 1
        elif ! cmp -s "$ref" "$mlt"; then
            echo "::error::Binary difference: $base"
            exit 1
        fi
    done
    echo "All {{compare_dir}}/*.mlt files have identical counterparts in {{ref_dir}} (except *-rust.mlt)."

# checks that the contents of two synthetic directories are equal
_verify-synthetic-equality: (just 'cargo-install' 'delta' 'git-delta')
    #!/usr/bin/env bash
    set -euo pipefail

    # `realpath --relative-to-...` doesn't work on MacOS
    # https://unix.stackexchange.com/a/655825/312244
    posix_relpath() {
        set -- "${1%/}/" "${2%/}/" ''
        while [ "$1" ] && [ "$2" = "${2#"$1"}" ]
        do
            set -- "${1%/?*/}/" "$2" "../$3"
        done
        rel="${3}${2#"$1"}"
        [ "${rel#/}" ] && rel="${rel%/}" || rel="${rel:-.}"
        printf '%s\n' "$rel"
    }

    DIR_A="../test/synthetic/0x01"
    DIR_B="../test/synthetic/0x01-rust"

    BIN="cargo run --bin mlt --quiet --"
    find "$DIR_A" -type f -iname "*.mlt" | while read -r file_a; do
      rel_path="$(posix_relpath "$DIR_A" "$file_a")"
      file_b="$DIR_B/$rel_path"
      file_b_rust="${file_b%.mlt}-rust.mlt"

      if [[ -f "$file_b" ]]; then
        if ! cmp -s "$file_a" "$file_b"; then
          echo "Differences: $rel_path"
          echo ""
          diff -U20 \
            --label "$file_a" \
            <($BIN dump "$file_a") \
            --label "$file_b" \
            <($BIN dump "$file_b") \
            | delta --side-by-side --file-modified-label "mlt dump difference"
        fi
      elif [[ -f "$file_b_rust" ]]; then
        diff -U20 \
          --label "$file_a" \
          <($BIN decode "$file_a") \
          --label "$file_b_rust" \
          <($BIN decode "$file_b_rust") \
          | delta --side-by-side --file-modified-label "mlt decoded difference"
      else
        echo "Missing in $DIR_B but present in $DIR_A: $rel_path"
      fi
    done

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
