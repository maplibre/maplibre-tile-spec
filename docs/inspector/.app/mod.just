just := quote(just_executable())

_default: (just '--list' 'inspector')

[private]
just *args:
    {{just}} {{args}}

# Build the app into docs/inspector/app
build: install
    npm run build

# Run all CI steps: lint, test, build
ci-test: lint test build
    {{just}} assert-git-is-clean

# Delete build artifacts
clean:
    rm -rf node_modules ../app

# Serve the app with the fixtures it indexed
dev: install
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
test: install
    npm run test
