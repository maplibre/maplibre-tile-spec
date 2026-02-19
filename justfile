#!/usr/bin/env just --justfile

# How to call the current just executable. Note that just_executable() may have `\` in Windows paths, so we need to quote it.
just := quote(just_executable())

# if running in CI, treat warnings as errors by setting RUSTFLAGS and RUSTDOCFLAGS to '-D warnings' unless they are already set
# Use `CI=true just ci-test` to run the same tests as in GitHub CI.
# Use `just env-info` to see the current values of RUSTFLAGS and RUSTDOCFLAGS
ci_mode := if env('CI', '') != '' {'1'} else {''}
# cargo-binstall needs a workaround due to caching
# ci_mode might be manually set by user, so re-check the env var
binstall_args := if env('CI', '') != '' {'--no-confirm --no-track --disable-telemetry'} else {''}

# By default, show the list of all available commands
@_default:
    {{just}} --list

bench: bench-js bench-java

[working-directory: 'java']
bench-java:
    ./gradlew test -Dbenchmark.iterations=200 -PincludeTags=benchmark
    ./gradlew jmh

bench-js: install-js
    echo "TODO: Add js benchmark command"

[working-directory: 'ts']
build-js: install-js
    npm run build

# Run integration tests, and override what we expect the output to be with the actual output
bless: _clean-int-test _test-run-int
    rm -rf test/expected && mv test/output test/expected

# Delete all build files for multiple languages
clean: clean-java clean-js clean-rust

# Delete build files for Java
clean-java:
    echo "TODO: Add java cleanup command"

# Delete build files for JavaScript
[working-directory: 'ts']
clean-js:
    rm -rf node_modules dist

# Delete build files for Rust
[working-directory: 'rust']
clean-rust:
    cargo clean

# Print Java environment info
[working-directory: 'java']
env-info-java:
    @echo "Running {{if ci_mode == '1' {'in CI mode'} else {'in dev mode'} }} on {{os()}} / {{arch()}}"
    ./gradlew --version

# Run all formatting in every language
fmt: fmt-rust fmt-java fmt-js

# Run formatting for Java
[working-directory: 'java']
fmt-java:
     ./gradlew spotlessApply

# Run formatting for JavaScript
[working-directory: 'ts']
fmt-js:
    npm run format

# Run formatting for Rust
[working-directory: 'rust']
fmt-rust:
    {{just}} fmt

[working-directory: 'ts']
install-js:
    npm ci

# Run linting in every language, failing on lint suggestion or bad formatting. Run `just fmt` to fix formatting issues.
lint: lint-java lint-js lint-rust

# Run linting for Java
[working-directory: 'java']
lint-java:
    ./gradlew spotlessJavaCheck

# Run linting for JavaScript
[working-directory: 'ts']
lint-js: install-js
    npm run lint

# Run linting for Rust
[working-directory: 'rust']
lint-rust:
    {{just}} clippy

# Run all tests in every language, including integration tests
test: test-java test-java-cli test-js test-rust test-int

# Run integration tests, ensuring that the output matches the expected output
test-int: _clean-int-test _test-run-int (_diff-dirs "test/output" "test/expected")

# Run tests for Java
[working-directory: 'java']
test-java:
    ./gradlew test

# Run Java cli tests
[working-directory: 'java']
test-java-cli:
    #!/usr/bin/env bash
    set -euo pipefail
    JAVA="java -Dcom.google.protobuf.use_unsafe_pre22_gencode"
    ENCODE="$JAVA -jar ./mlt-cli/build/libs/encode.jar"
    DECODE="$JAVA -jar ./mlt-cli/build/libs/decode.jar"
    ./gradlew cli
    # Test the encoding CLI
    $ENCODE --mvt ../test/fixtures/omt/10_530_682.mvt --mlt output/varint.mlt --decode
    # Test the using advanced encodings
    $ENCODE --mvt ../test/fixtures/omt/10_530_682.mvt --enable-fastpfor --enable-fsst --mlt output/advanced.mlt
    # decode
    $DECODE --mlt output/advanced.mlt
    # Smoke-test container conversions
    $ENCODE --mbtiles ../test/fixtures/omt.max1.mbtiles --outlines ALL --colmap-delim '[]name/[:_]/' --tessellate --sort-ids --coerce-mismatch --verbose 0 --parallel
    $ENCODE --pmtiles ../test/fixtures/omt-planet-20260112.mvt.max1.pmtiles --outlines ALL --colmap-delim '[]name/[:_]/' --tessellate --sort-ids --coerce-mismatch --verbose 0 --parallel

# Run tests for JavaScript
[working-directory: 'ts']
test-js: install-js
    npm run test

# Run tests for Rust
[working-directory: 'rust']
test-rust:
    {{just}} test

# Delete integration test output files
_clean-int-test:
    rm -rf test/output && mkdir -p test/output

# Compare two directories to ensure they are the same
_diff-dirs OUTPUT_DIR EXPECTED_DIR:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "** Comparing {{OUTPUT_DIR}} with {{EXPECTED_DIR}}..."
    if ! diff --brief --recursive --new-file {{OUTPUT_DIR}} {{EXPECTED_DIR}}; then
        echo "** Expected output does not match actual output"
        echo "** You may want to run 'just bless' to update expected output"
        exit 1
    else
        echo "** Expected output matches actual output"
    fi

# Run integration tests
_test-run-int:
    echo "TODO: Add integration test command, outputting to test/output"
    echo "fake output by copying expected into output so that the rest of the script works"
    # TODO: REMOVE THIS, and replace it with a real integration test run
    cp -r test/expected/* test/output

mkdocs:
	docker build -t squidfunk/mkdocs-material mkdocs
	cd mkdocs && docker run --rm -it -p 8000:8000 -v ${PWD}:/docs squidfunk/mkdocs-material

mkdocs-build:
    docker build -t squidfunk/mkdocs-material mkdocs
    cd mkdocs && docker run --rm -v ${PWD}:/docs squidfunk/mkdocs-material build --strict

# Build Java encoder and generate .mlt files for all .pbf files in test/fixtures
[working-directory: 'java']
generate-expected-mlt:  (cargo-install 'fd' 'fd-find')
    ./gradlew cli
    fd . ../test/fixtures --no-ignore --extension pbf --extension mvt -x {{just}} _generate-one-expected-mlt

# Generate a single .mlt file for a given .mvt or .pbf file, assuming JAR is built
[working-directory: 'java']
_generate-one-expected-mlt file:
    java \
        -Dcom.google.protobuf.use_unsafe_pre22_gencode \
        -jar mlt-cli/build/libs/encode.jar \
        --mvt {{quote(file)}} \
        --mlt {{quote(replace(without_extension(file) + '.mlt', '/fixtures/', '/expected/tag0x01/'))}} \
        --outlines ALL \
        --colmap-delim '[.*]name/[:_]/' \
        --enable-fsst \
        --tessellate \
        --coerce-mismatch \
        --verbose

# Generate synthetic .mlt files and ensure there are no duplicates
generate-synthetic-mlts:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf test/synthetic
    (cd java && ./gradlew :mlt-tools:generateSyntheticMlt)

    # Check for duplicate .mlt files by computing their hashes
    all_hashes=$(find "test/synthetic" -name '*.mlt' -exec sha256sum {} \; | sort)
    duplicates=$(echo "$all_hashes" | awk '{print $1}' | uniq -d)
    if [ -n "$duplicates" ]; then
        echo "::error::Duplicate synthetic MLT files found"
        while IFS= read -r hash; do
            echo ""
            echo "$all_hashes" | grep "^$hash " | awk '{print "  - " $2}'
        done <<< "$duplicates"
        exit 1
    fi

ci-check-synthetic-mlts:
    @echo "Making sure the repo is clean before generating synthetic MLT files."
    {{just}} assert-git-is-clean
    @echo "GIT is clean. Generating synthetic MLT files. If synthetic MLT files change, the git status will no longer be clean, and the CI check will fail."
    {{just}} generate-synthetic-mlts
    {{just}} assert-git-is-clean
    @echo "GIT is still clean after generating synthetic MLT files. Synthetic MLT files are up to date."

# Extract version from a tag by removing language prefix and 'v' prefix
extract-version language tag:
    @echo "{{replace(replace(tag, language + '-', ''), 'v', '')}}"

# Make sure the git repo has no uncommitted changes
assert-git-is-clean:
    @if [ -n "$(git status --porcelain --untracked-files=all)" ]; then \
        >&2 echo "::error::git repo is not clean. Make sure compilation and tests artifacts are in the .gitignore, and no repo files are modified." ;\
        >&2 echo "######### git status ##########" ;\
        git status ;\
        git --no-pager diff ;\
        exit 1 ;\
    fi

# Check if a certain Cargo command is installed, and install it if needed
cargo-install $COMMAND $INSTALL_CMD='' *args='':
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v $COMMAND > /dev/null; then
        if ! command -v cargo-binstall > /dev/null; then
            echo "$COMMAND could not be found. Installing it with    cargo install ${INSTALL_CMD:-$COMMAND} --locked {{args}}"
            cargo install ${INSTALL_CMD:-$COMMAND} --locked {{args}}
        else
            echo "$COMMAND could not be found. Installing it with    cargo binstall ${INSTALL_CMD:-$COMMAND} {{binstall_args}} --locked {{args}}"
            cargo binstall ${INSTALL_CMD:-$COMMAND} {{binstall_args}} --locked {{args}}
        fi
    fi

[working-directory: 'cpp']
cpp-cmake-init:
    cmake -B build -S . -DCMAKE_BUILD_TYPE=Coverage

[working-directory: 'cpp']
cpp-cmake-build: cpp-cmake-init
    cmake --build build --target mlt-cpp-test mlt-cpp-json

[working-directory: 'cpp/build']
cpp-test: cpp-cmake-build
    ctest

[working-directory: 'cpp/build']
cpp-coverage: check-gcovr-is-installed cpp-test
    gcovr --root ../.. \
        --filter ../src --filter ../include \
        --txt coverage.txt \
        --cobertura-pretty --cobertura coverage.xml \
        --html-details coverage.html
    @echo "Coverage report at $PWD/coverage.html"

check-gcovr-is-installed:
    @which gcovr > /dev/null || (echo "Error: gcovr is not installed. Please install it using: brew install gcovr (macOS) or pip3 install gcovr (Linux)" && exit 1)
