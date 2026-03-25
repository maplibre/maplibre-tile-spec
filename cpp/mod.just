just := quote(just_executable())

_default: (just '--list' 'cpp')

[private]
just *args:
    {{just}} {{args}}

# Initialize CMake build
cmake-init:
    cmake -B build -S . -DCMAKE_BUILD_TYPE=Coverage

# Build with CMake
cmake-build: cmake-init
    cmake --build build --target mlt-cpp-test mlt-cpp-json

# Quick compile check (CMake build)
check: cmake-build

# Run all CI steps: test, coverage, bazel build
ci-test: coverage bazel-build
    {{just}} assert-git-is-clean

# Delete build artifacts
clean:
    rm -rf build

# Reformat code
fmt:
    pre-commit run --all-files clang-format

# Run linting
lint:
    echo "TODO: Add C++ linting command"

_install_npm_deps:
    cd ../test/synthetic/synthetic-test-utils && npm ci
    npm ci

_test-synthetic: _install_npm_deps
    {{just}} ts install
    npx vitest tool/synthetic.test.ts

# Run tests
[working-directory: 'build']
test: _clean_coverage_data cmake-build _test-synthetic
    ctest
    {{just}} cpp::decode-synthetics

_clean_coverage_data:
    find . -name "*.gcda" -delete

# Generate coverage report
[working-directory: 'build']
coverage: _clean_coverage_data (_check-tool 'gcovr') test
    gcovr --root ../.. \
        --filter ../src --filter ../include \
        --txt coverage.txt \
        --merge-mode-functions=separate \
        --cobertura-pretty --cobertura coverage.xml \
        --html-details coverage.html
    @echo "Coverage report at $PWD/coverage.html"

# Build with Bazel
bazel-build:
    bazel build //cpp:mlt_cpp

# Sanity test for mlt-cpp-json tool
test-json:
    build/tool/mlt-cpp-json ../test/expected/tag0x01/bing/4-12-6.mlt | jq . >/dev/null

decode-synthetics: cmake-build (just 'cargo-install' 'fd' 'fd-find')
    #!/usr/bin/env bash
    set -e
    JSON_TOOL=build/tool/mlt-cpp-json
    fd . ../test/synthetic/0x01 -e mlt -x bash -c "echo {} ; $JSON_TOOL {} >/dev/null"
    fd . ../test/synthetic/0x01-rust -e mlt -x bash -c "echo {} ; $JSON_TOOL {} >/dev/null"

_check-tool command:
    @which {{command}} > /dev/null || (echo "Error: {{command}} is not installed. Please install it using: brew install {{command}} (macOS) or pip3 install {{command}} (Linux)" && exit 1)
