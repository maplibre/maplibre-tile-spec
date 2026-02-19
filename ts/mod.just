just := quote(just_executable())

_default: (just '--list' 'ts')

[private]
just *args:
    {{just}} {{args}}

# Build the package
build: install
    npm run build

# Quick compile/type check
check: install
    npm run build

# Run all CI steps: lint, test, build
ci-test: lint test build (just 'assert-git-is-clean')

# Delete build artifacts
clean:
    rm -rf node_modules dist

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
test: install
    npm run test

# Run benchmark
bench: install
    echo "TODO: Add js benchmark command"
