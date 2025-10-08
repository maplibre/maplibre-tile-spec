#!/bin/bash

if [[ $# -lt 2 ]]; then
    echo "Error: Missing arguments. Usage: $0 <language> <tag>" >&2
    exit 1
fi

language="$1"
tag="$2"
version=${tag#${language}-}
version=${version#v}
version=$(echo "$version" | tr '[:upper:]' '[:lower:]')
echo "$version"
