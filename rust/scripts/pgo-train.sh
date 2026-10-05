#!/usr/bin/env bash
set -euo pipefail

mlt=$1
fixtures=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

for corpus in amazon amazon_here bing omt simple; do
    for version in 1 2; do
        "$mlt" convert "$fixtures/$corpus" "$work/v$version/$corpus" --mlt-version "$version"
    done
    "$mlt" convert "$work/v2/$corpus" "$work/mvt/$corpus" --to mvt
done
"$mlt" convert "$fixtures/omt.max1.mbtiles" "$work/omt.mbtiles" --mlt-version 2
"$mlt" ls "$work/v1" > /dev/null
"$mlt" ls "$work/v2" > /dev/null
find "$work/v1" "$work/v2" -name '*.mlt' | while read -r tile; do
    "$mlt" decode "$tile" > /dev/null
done
