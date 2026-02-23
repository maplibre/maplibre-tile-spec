#!/usr/bin/env just --justfile

mod cpp
mod java
mod rust
mod ts

just := quote(just_executable())
ci_mode := if env('CI', '') != '' {'1'} else {''}

# By default, show the list of all available commands
@_default:
    {{just}} --list --list-submodules

bench:
    {{just}} rust::bench
    {{just}} java::bench
    {{just}} ts::bench

# Run integration tests, and override what we expect the output to be with the actual output
bless: _clean-int-test _test-run-int
    rm -rf test/expected && mv test/output test/expected

# Delete all build files for multiple languages
clean:
    {{just}} rust::clean
    {{just}} java::clean
    {{just}} ts::clean
    {{just}} cpp::clean

# Run all formatting in every language
fmt:
    {{just}} rust::fmt
    {{just}} java::fmt
    {{just}} ts::fmt
    {{just}} cpp::fmt

# Run linting in every language. Run `just fmt` to fix formatting issues.
lint:
    {{just}} rust::lint
    {{just}} java::lint
    {{just}} ts::lint
    {{just}} cpp::lint

# Run all tests in every language, including integration tests
test: test-int
    {{just}} rust::test
    {{just}} java::test
    {{just}} ts::test
    {{just}} cpp::test

# Run integration tests, ensuring that the output matches the expected output
test-int: _clean-int-test _test-run-int (_diff-dirs "test/output" "test/expected")

docs:
	docker run --rm -it -p 8000:8000 -v ${PWD}:/docs zensical/zensical:latest

docs-build:
    docker run --rm -v ${PWD}:/docs zensical/zensical:latest build

# Extract version from a tag by removing language prefix and 'v' prefix
ci-extract-version language tag:
    @echo "{{replace(replace(tag, language + '-', ''), 'v', '')}}"

# Run the mlt CLI tool with the given arguments. Working dir is the `rust` subdir.
[working-directory: 'rust']
mlt *args:
    cargo run --package mlt -- {{args}}

# Ensure a command is available
assert-cmd command:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! type {{command}} > /dev/null; then
        echo "Command '{{command}}' could not be found. Please make sure it has been installed on your computer."
        exit 1
    fi

# Install a Cargo tool if missing (uses cargo-binstall when available)
cargo-install $COMMAND $INSTALL_CMD='' *args='':
    #!/usr/bin/env bash
    set -euo pipefail
    binstall_args="{{ if env('CI', '') != '' {'--no-confirm --no-track --disable-telemetry'} else {''} }}"
    if ! command -v $COMMAND > /dev/null; then
        if ! command -v cargo-binstall > /dev/null; then
            echo "$COMMAND could not be found. Installing it with    cargo install ${INSTALL_CMD:-$COMMAND} --locked {{args}}"
            cargo install ${INSTALL_CMD:-$COMMAND} --locked {{args}}
        else
            echo "$COMMAND could not be found. Installing it with    cargo binstall ${INSTALL_CMD:-$COMMAND} $binstall_args --locked {{args}}"
            cargo binstall ${INSTALL_CMD:-$COMMAND} $binstall_args --locked {{args}}
        fi
    fi

# Make sure the git repo has no uncommitted changes. Fails only if CI envvar is set.
assert-git-is-clean:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -n "$(git status --porcelain --untracked-files=all)" ]; then
        >&2 echo "::error::git repo is not clean. Make sure compilation and tests artifacts are in the .gitignore, and no repo files are modified."
        if [[ "{{ci_mode}}" == "1" ]]; then
            >&2 echo "######### git status ##########"
            git status
            git --no-pager diff
            exit 1
        else
            >&2 echo "git repo is not clean, but not failing because CI mode is not enabled."
        fi
    fi

_clean-int-test:
    rm -rf test/output && mkdir -p test/output

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

_test-run-int:
    echo "TODO: Add integration test command, outputting to test/output"
    echo "fake output by copying expected into output so that the rest of the script works"
    # TODO: REMOVE THIS, and replace it with a real integration test run
    cp -r test/expected/* test/output
