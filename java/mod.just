ci_mode := if env('CI', '') != '' {'1'} else {''}
just := quote(just_executable())

_default: (just '--list' 'java')

[private]
just *args:
    {{just}} {{args}}

# Quick compile check
check:
    ./gradlew check

# Run all CI steps: lint, test, cli tests, coverage, synthetic MLT check
ci-test: env-info lint test test-cli test-publish ci-check-synthetic-mlts
    {{just}} assert-git-is-clean

# Verify synthetic MLT files are up to date
ci-check-synthetic-mlts:
    {{just}} assert-git-is-clean
    {{just}} java::generate-synthetic-mlts
    {{just}} assert-git-is-clean
    @echo "Synthetic MLT check complete."

# Print environment info
env-info:
    @echo "Running {{if ci_mode == '1' {'in CI mode'} else {'in dev mode'} }} on {{os()}} / {{arch()}}"
    ./gradlew --version

# Reformat code
fmt:
    ./gradlew spotlessApply

# Clean build artifacts
clean:
    ./gradlew clean

# Build Java encoder and generate .mlt files for all fixtures
generate-expected-mlt: (just 'cargo-install' 'fd' 'fd-find')
    ./gradlew cli
    fd . ../test/fixtures --no-ignore --extension pbf --extension mvt -x {{just}} java::_generate-one-expected-mlt

# Build Java encoder and run it to generate snippets for all .pbf files
gen-snippets: (just 'cargo-install' 'fd' 'fd-find')
    #!/usr/bin/env bash
    set -euo pipefail
    ./gradlew cli
    fd . ../test/fixtures -e pbf -x java -jar mlt-cli/build/libs/encode.jar --mvt {} \
        --mlt "$(tmp='{.}'; echo ${tmp/\/fixtures\///expected/}).mvt" \
        --rawstreams --tessellate --outlines ALL || echo "### Some snippets failed to generate ###"

# Build Java encoder and run it to generate test files
gen-tests: (just 'cargo-install' 'fd' 'fd-find')
    #!/usr/bin/env bash
    set -euo pipefail
    ./gradlew cli
    fd . ../test/fixtures -e pbf -x bash -c 'java -jar mlt-cli/build/libs/encode.jar --mvt {} \
        --mlt "$(tmp={.}; echo ${tmp/fixtures/expected/tag0x01/}).mlt" \
        --outlines "\*" --tessellate --coerce-mismatch --verbose || echo "### Some snippets failed to generate ###"'

# Generate synthetic .mlt files and ensure there are no duplicates
generate-synthetic-mlts:
    #!/usr/bin/env bash
    set -euo pipefail
    rm -rf ../test/synthetic
    ./gradlew :mlt-tools:generateSyntheticMlt
    all_hashes=$(find "../test/synthetic" -name '*.mlt' -exec sha256sum {} \; | sort)
    duplicates=$(echo "$all_hashes" | awk '{print $1}' | uniq -d)
    if [ -n "$duplicates" ]; then
        echo "::error::Duplicate synthetic MLT files found"
        while IFS= read -r hash; do
            echo ""
            echo "$all_hashes" | grep "^$hash " | awk '{print "  - " $2}'
        done <<< "$duplicates"
        exit 1
    fi

# Run linting (format check)
lint:
    ./gradlew spotlessJavaCheck

# Run tests
test:
    ./gradlew test

# Run Java cli tests
test-cli:
    #!/usr/bin/env bash
    set -euo pipefail
    JAVA="java -Dcom.google.protobuf.use_unsafe_pre22_gencode"
    ENCODE="$JAVA -jar ./mlt-cli/build/libs/encode.jar"
    DECODE="$JAVA -jar ./mlt-cli/build/libs/decode.jar"
    ./gradlew cli
    $ENCODE --mvt ../test/fixtures/omt/10_530_682.mvt --mlt output/varint.mlt --decode
    $ENCODE --mvt ../test/fixtures/omt/10_530_682.mvt --enable-fastpfor --enable-fsst --mlt output/advanced.mlt
    $DECODE --mlt output/advanced.mlt
    $ENCODE --mbtiles ../test/fixtures/omt.max1.mbtiles --outlines ALL --colmap-delim '[]name/[:_]/' --tessellate --sort-ids --coerce-mismatch --verbose 0 --parallel
    $ENCODE --pmtiles ../test/fixtures/omt-planet-20260112.mvt.max1.pmtiles --outlines ALL --colmap-delim '[]name/[:_]/' --tessellate --sort-ids --coerce-mismatch --verbose 0 --parallel

# Run tests with coverage
test-coverage:
    ./gradlew jacocoTestReport

# Test Maven local publishing
test-publish:
    ./gradlew publishMavenPublicationToMavenLocal

# Run benchmark tests
bench:
    ./gradlew test -Dbenchmark.iterations=200 -PincludeTags=benchmark
    ./gradlew jmh

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
