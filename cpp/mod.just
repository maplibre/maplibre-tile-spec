just := quote(just_executable())

_default: (_root '--list' 'cpp')

_root *args:
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
ci-test: coverage bazel-build (_root 'assert-git-is-clean')

# Delete build artifacts
clean:
    rm -rf build

# Reformat code
fmt:
    echo "TODO: Add C++ formatting command"

# Run linting
lint:
    echo "TODO: Add C++ linting command"

# Run tests
[working-directory: 'build']
test: cmake-build
    ctest

# Generate coverage report
[working-directory: 'build']
coverage: (_check-tool 'gcovr') test
    gcovr --root ../.. \
        --filter ../src --filter ../include \
        --txt coverage.txt \
        --cobertura-pretty --cobertura coverage.xml \
        --html-details coverage.html
    @echo "Coverage report at $PWD/coverage.html"

# Build with Bazel
bazel-build:
    bazel build //cpp:mlt_cpp

# Sanity test for mlt-cpp-json tool
test-json:
    build/tool/mlt-cpp-json ../test/expected/tag0x01/bing/4-12-6.mlt | jq .

_check-tool command:
    @which {{command}} > /dev/null || (echo "Error: {{command}} is not installed. Please install it using: brew install {{command}} (macOS) or pip3 install {{command}} (Linux)" && exit 1)
