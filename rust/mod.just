main_crate := 'mlt-core'
just := quote(just_executable())

# if running in CI, treat warnings as errors by setting RUSTFLAGS and RUSTDOCFLAGS to '-D warnings' unless they are already set
# Use `CI=true just ci-test` to run the same tests as in GitHub CI.
# Use `just env-info` to see the current values of RUSTFLAGS and RUSTDOCFLAGS
ci_mode := if env('CI', '') != '' {'1'} else {''}
export RUSTFLAGS := env('RUSTFLAGS', if ci_mode == '1' {'-D warnings'} else {''})
export RUSTDOCFLAGS := env('RUSTDOCFLAGS', if ci_mode == '1' {'-D warnings'} else {''})
export RUST_BACKTRACE := env('RUST_BACKTRACE', if ci_mode == '1' {'1'} else {'0'})

_default: (just '--list' 'rust')

[private]
just *args:
    {{just}} {{args}}

# Run integration tests and save its output as the new expected output
bless *args: (just 'cargo-install' 'cargo-insta')
    cargo insta test --accept --unreferenced=delete --all-features {{args}}

# Run benchmarks
bench:
    cargo bench --all-targets --all-features

alias b := build
# Build the project
build:
    cargo build --all-targets --all-features

alias c := check
# Quick compile without building a binary
check: (just 'cargo-install' 'cargo-hack')
    cargo hack check --all-targets --each-feature --workspace
    cd mlt-core/fuzz && cargo check --all-features

# Generate code coverage report to upload to codecov.io
ci-coverage: env-info (just 'java::gen-snippets')
    mkdir -p target/llvm-cov
    # ATTENTION: the output-path here is used in the CI workflow
    {{just}} rust::coverage '--codecov --output-path target/llvm-cov/codecov.info'

# Run minimal subset of tests to ensure compatibility with MSRV
ci-check: env-info check

# Run linting as expected by CI
ci-lint: env-info test-fmt clippy
    {{just}} assert-git-is-clean  # call it explicitly to avoid just optimizing it to just one call

# Run all tests as expected by CI
ci-test: env-info (just 'java::gen-snippets') test
    {{just}} assert-git-is-clean  # call it explicitly to avoid just optimizing it to just one call

# Run minimal subset of tests to ensure compatibility with MSRV
ci-test-msrv: env-info
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
clippy *args:
    cargo clippy --workspace --all-targets --all-features {{args}}
    cd mlt-core/fuzz && cargo clippy --all-features {{args}}

# Generate code coverage report. Installs `cargo llvm-cov`
coverage *args='--no-clean --open': (just 'cargo-install' 'cargo-llvm-cov')
    cargo llvm-cov --all-targets --all-features --include-build-script {{args}}

# Build and open code documentation
docs *args='--open':
    DOCS_RS=1 cargo doc --no-deps {{args}} --all-features --package mlt-core

# Print environment info
env-info:
    @echo "Running for '{{main_crate}}' crate {{if ci_mode == '1' {'in CI mode'} else {'in dev mode'} }} on {{os()}} / {{arch()}}"
    @echo "PWD {{justfile_directory()}}"
    {{just}} --version
    rustc --version
    cargo --version
    rustup --version
    @echo "RUSTFLAGS='$RUSTFLAGS'"
    @echo "RUSTDOCFLAGS='$RUSTDOCFLAGS'"
    @echo "RUST_BACKTRACE='$RUST_BACKTRACE'"

alias f := fmt
# Reformat code. If nightly is available, use it for better results.
fmt:
    #!/usr/bin/env bash
    set -euo pipefail
    if (rustup toolchain list | grep nightly && rustup component list --toolchain nightly | grep rustfmt) &> /dev/null; then
        echo 'Reformatting Rust code using nightly Rust fmt to sort imports'
        cargo +nightly fmt --all -- --config imports_granularity=Module,group_imports=StdExternalCrate
    else
        echo 'Reformatting Rust with the stable cargo fmt.  Install nightly with `rustup install nightly` for better results'
        cargo fmt --all
    fi

# Reformat all Cargo.toml files (installs `cargo sort`)
fmt-toml *args: (just 'cargo-install' 'cargo-sort')
    cargo sort --workspace --grouped {{args}}

# Fuzz the layer target (installs `cargo fuzz`)
fuzz *args: (cargo-fuzz 'run' 'layer' '--' args)

# Minimize the size of the fuzzing corpus (installs `cargo fuzz`)
fuzz-cmin file: (cargo-fuzz 'tmin' 'layer' file)

# Minimize the crash in file (installs `cargo fuzz`)
fuzz-tmin file: (cargo-fuzz 'tmin' 'layer' file)

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

# Build mlt-pyo3 and launch a Python interpreter with mlt_pyo3 pre-imported
[working-directory: 'mlt-pyo3']
py: (just 'cargo-install' 'maturin')
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ ! -d ".venv" ]]; then
        echo "Please make sure to create your virtual environment by running 'python -m venv .venv'"
        exit 1
    fi
    source .venv/bin/activate
    maturin develop
    python3 -i -c "import mlt_pyo3; print('mlt_pyo3 ready — try: mlt_pyo3.decode_mlt(data)')"

py-publish:
    docker run --rm -v $(pwd):/io ghcr.io/pyo3/maturin build --release

# Run cargo-release
release *args='': (just 'cargo-install' 'release-plz')
    release-plz {{args}}

# Check semver compatibility with prior published version
semver *args: (just 'cargo-install' 'cargo-semver-checks')
    cargo semver-checks --all-features {{args}}

# Regenerate the Python type stub for mlt-pyo3
sync-pyo3-stubs:
    cargo run -p mlt-pyo3 --bin stub_gen --features abi3

# Sync the version field in mlt-pyo3/pyproject.toml to match mlt-pyo3/Cargo.toml
sync-pyo3-version:
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION=$({{just}} rust::get-crate-field version mlt-pyo3)
    PYPROJECT="mlt-pyo3/pyproject.toml"
    sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$PYPROJECT"
    echo "Synced $PYPROJECT to version $VERSION"

alias t := test
# Run all tests
test:
    cargo test --all-targets --all-features
    {{just}} rust::generate-synthetic-mlts
    {{just}} rust::test-doc

# Test documentation generation
test-doc: (docs '--document-private-items')
    cargo test --doc --all-features

# Test code formatting
test-fmt: (fmt-toml '--check' '--check-format')
    cargo fmt --all -- --check

# Test workspace publishing with --dry-run
test-publish:
    cargo publish --workspace --dry-run

# Find unused dependencies
udeps: (just 'cargo-install' 'cargo-udeps')
    cargo +nightly udeps --all-targets --all-features

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
    {{just}} rust::_assert-synthetic-mlt-identity
    {{just}} rust::_verify-synthetic-equality

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

    DIR_A="../test/synthetic/0x01"
    DIR_B="../test/synthetic/0x01-rust"

    BIN="cargo run --quiet --"
    find "$DIR_A" -type f -iname "*.mlt" | while read -r file_a; do
      rel_path="$(realpath --relative-to="$DIR_A" "$file_a")"
      file_b="$DIR_B/$rel_path"

      if [[ -f "$file_b" ]]; then
        if ! cmp -s "$file_a" "$file_b"; then
          echo "Differences: $rel_path"
          echo ""
          diff -U20 \
            --label "$file_a" \
            <($BIN decode "$file_a") \
            --label "$file_b" \
            <($BIN decode "$file_b") \
            | delta --side-by-side --file-modified-label "mlt decoded difference"
        fi
      else
        echo "Missing in $DIR_B but present in $DIR_A: $rel_path"
      fi
    done


[private]
[working-directory: 'mlt-core']
cargo-fuzz *args:  (just 'cargo-install' 'cargo-fuzz')
    cargo +nightly fuzz {{args}}
