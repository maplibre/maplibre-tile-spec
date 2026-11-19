just := quote(just_executable())

_default: (just '--list' 'js')

[private]
just *args:
    {{just}} {{args}}

# Install devDependencies without writing a lockfile
install:
    npm install --no-package-lock --no-fund --no-audit

# Rebuild minified dist bundles and copy TypeScript declarations
build: install
    npm run build

# Verify TypeScript declarations under strict NodeNext resolution
lint: install
    npm run typecheck

# Run full Node test suite (synthetic + 3D meshes + package exports)
test:
    npm test

# Run all CI steps: build, typecheck, test, and verify dist/ is up to date
ci-test: build lint test
    {{just}} assert-git-is-clean

# Run decode speed and heap benchmark
bench:
    npm run bench

# Remove local node_modules (keep committed dist/ bundles)
clean:
    rm -rf node_modules
