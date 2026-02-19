main_crate := 'mlt-core'
just := quote(just_executable())

# if running in CI, treat warnings as errors by setting RUSTFLAGS and RUSTDOCFLAGS to '-D warnings' unless they are already set
# Use `CI=true just ci-test` to run the same tests as in GitHub CI.
# Use `just env-info` to see the current values of RUSTFLAGS and RUSTDOCFLAGS
ci_mode := if env('CI', '') != '' {'1'} else {''}
export RUSTFLAGS := env('RUSTFLAGS', if ci_mode == '1' {'-D warnings'} else {''})
export RUSTDOCFLAGS := env('RUSTDOCFLAGS', if ci_mode == '1' {'-D warnings'} else {''})
export RUST_BACKTRACE := env('RUST_BACKTRACE', if ci_mode == '1' {'1'} else {'0'})

_default: (_root '--list' 'rust')

_root *args:
    {{just}} {{args}}

# Run integration tests and save its output as the new expected output
bless *args: (_root 'cargo-install' 'cargo-insta')
    cargo insta test --accept --unreferenced=delete --all-features {{args}}

# Build the project
build:
    cargo build --all-targets --all-features

# Quick compile without building a binary
check: (_root 'cargo-install' 'cargo-hack')
    cargo hack check --all-targets --each-feature --workspace

# Generate code coverage report to upload to codecov.io
ci-coverage: env-info _run-gen-snippets && \
            (coverage '--codecov --output-path target/llvm-cov/codecov.info')
    # ATTENTION: the full file path above is used in the CI workflow
    mkdir -p target/llvm-cov

# Run minimal subset of tests to ensure compatibility with MSRV
ci-check: env-info check

# Run linting as expected by CI
ci-lint: env-info test-fmt clippy (_root 'assert-git-is-clean')

# Run all tests as expected by CI
ci-test: env-info _run-gen-snippets test test-doc (_root 'assert-git-is-clean')

# Run minimal subset of tests to ensure compatibility with MSRV
ci-test-msrv: env-info ci-check _run-gen-snippets test

# Clean all build artifacts
clean:
    cargo clean
    rm -f Cargo.lock

# Run cargo clippy to lint the code
clippy *args:
    cargo clippy --all-targets --all-features {{args}}

# Generate code coverage report. Installs `cargo llvm-cov`
coverage *args='--no-clean --open': (_root 'cargo-install' 'cargo-llvm-cov')
    cargo llvm-cov --all-targets --all-features --include-build-script {{args}}

# Build and open code documentation
docs *args='--open':
    DOCS_RS=1 cargo doc --no-deps {{args}} --all-features

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

# Reformat all Cargo.toml files (intslls `cargo sort`)
fmt-toml *args: (_root 'cargo-install' 'cargo-sort')
    cargo sort --workspace --grouped {{args}}

_run-gen-snippets: (_root 'java::gen-snippets')

# Get any package's field from the metadata
get-crate-field field package=main_crate: (_root 'assert-cmd' 'jq')
    cargo metadata --format-version 1 | jq -e -r '.packages | map(select(.name == "{{package}}")) | first | .{{field}} | select(. != null)'

# Get the minimum supported Rust version (MSRV) for the crate
get-msrv package=main_crate: (get-crate-field 'rust_version' package)

# Run linting (clippy + format check)
lint: clippy test-fmt

# Find the minimum supported Rust version and update Cargo.toml
msrv: (_root 'cargo-install' 'cargo-msrv')
    cargo msrv find --write-msrv --ignore-lockfile --component rustfmt -- {{just}} rust::ci-test-msrv

# Run cargo-release
release *args='': (_root 'cargo-install' 'release-plz')
    release-plz {{args}}

# Check semver compatibility with prior published version
semver *args: (_root 'cargo-install' 'cargo-semver-checks')
    cargo semver-checks --all-features {{args}}

# Run all tests
test:
    cargo test --all-targets --all-features
    cargo test --doc --all-features

# Test documentation generation
test-doc: (docs '')

# Test code formatting
test-fmt: (fmt-toml '--check' '--check-format')
    cargo fmt --all -- --check

# Use the experimental workspace publishing with --dry-run
test-publish:
    cargo +nightly -Z package-workspace publish --dry-run

# Find unused dependencies
udeps: (_root 'cargo-install' 'cargo-udeps')
    cargo +nightly udeps --all-targets --all-features

# Update all dependencies, including breaking changes
update:
    cargo +nightly -Z unstable-options update --breaking
    cargo update
