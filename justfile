#!/usr/bin/env just --justfile

# By default, show the list of all available commands
@_default:
    {{ just_executable() }} --list --unsorted

# Delete all build files for multiple languages
clean: clean-java clean-js clean-rust

# Delete build files for Java
clean-java:
    echo "TODO: Add java cleanup command"

# Delete build files for JavaScript
clean-js:
    echo "TODO: Add js cleanup command"

# Delete build files for Rust
clean-rust:
    cd rust && cargo clean

# Run all tests in every language, including integration tests
test: test-java test-java-cli test-js test-rust test-int

# Run tests for Java
test-java:
    cd java && ./gradlew test

# Run Java cli tests
test-java-cli:
    #!/usr/bin/env bash
    set -euo pipefail
    cd java  # Changing directory requires this recipe to have the #!/... line at the top, i.e. be a proper script
    ./gradlew cli
    # Test the encoding CLI
    java -jar ./build/libs/encode.jar -mvt ../test/fixtures/omt/10_530_682.mvt -metadata -mlt output/varint.mlt
    # ensure expected size
    python3 -c 'import os; expected=2432; ts=os.path.getsize("output/varint.mlt.meta.pbf"); assert ts == expected, f"tile size changed from expected ({expected}), got: {ts}"'
    # Test the meta CLI and ensure it doesn't overwrite the metadata (a sign it correctly matches the encode output)
    java -jar ./build/libs/meta.jar -mvt ../test/fixtures/omt/10_530_682.mvt -meta output/varint.mlt.meta.pbf
    # ensure expected size is maintained (meta writes the same meta file as encode)
    python3 -c 'import os; expected=2432; ts=os.path.getsize("output/varint.mlt.meta.pbf"); assert ts == expected, f"tile size changed from expected ({expected}), got: {ts}"'
    # Test the using advanced encodings
    java -jar ./build/libs/encode.jar -mvt ../test/fixtures/omt/10_530_682.mvt -metadata -advanced -mlt output/advanced.mlt
    # ensure expected sizes
    python3 -c 'import os; expected=67011; ts=os.path.getsize("output/varint.mlt"); assert ts == expected, f"tile size changed from expected ({expected}), got: {ts}"'
    python3 -c 'import os; expected=64776; ts=os.path.getsize("output/advanced.mlt"); assert ts == expected, f"tile size changed from expected ({expected}), got: {ts}"'
    # ensure we can decode the advanced tile
    java -jar ./build/libs/decode.jar -mlt output/advanced.mlt -vectorized

install-js:
    npm ci

# Run tests for JavaScript
test-js: install-js
    npm test

# Run tests for Rust
test-rust:
    cd rust && cargo test

bench-js: install-js
    npm run bench

bench-java:
    cd java && ./gradlew jmh

bench: bench-js bench-java

# Run integration tests, ensuring that the output matches the expected output
test-int: clean-int-test test-run-int (diff-dirs "test/output" "test/expected")

# Run integration tests, and override what we expect the output to be with the actual output
bless: clean-int-test test-run-int
    rm -rf test/expected && mv test/output test/expected

# Run linting in every language, failing on lint suggestion or bad formatting. Run `just fmt` to fix formatting issues.
lint: lint-java lint-js lint-rust

# Run linting for Java
lint-java:
    cd java && ./gradlew spotlessJavaCheck

# Run linting for JavaScript
lint-js:
    echo "TODO: Add js lint command (e.g. eslint)"

# Run linting for Rust
lint-rust:
    cd rust && cargo clippy
    cd rust && cargo fmt --all -- --check

# Run all formatting in every language
fmt: fmt-java fmt-js fmt-rust

# Run formatting for Java
fmt-java:
     cd java && ./gradlew spotlessApply

# Run formatting for JavaScript
fmt-js:
    echo "TODO: Add js fmt command (e.g. prettier)"

# Run formatting for Rust
fmt-rust:
    cd rust && cargo fmt --all

# Delete integration test output files
[private]
clean-int-test:
    rm -rf test/output && mkdir -p test/output

# Run integration tests
[private]
test-run-int:
    echo "TODO: Add integration test command, outputting to test/output"
    echo "fake output by copying expected into output so that the rest of the script works"
    # TODO: REMOVE THIS, and replace it with a real integration test run
    cp -r test/expected/* test/output

# Compare two directories to ensure they are the same
[private]
diff-dirs OUTPUT_DIR EXPECTED_DIR:
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
