#!/usr/bin/env bash
# Per-file region, line, branch and condition coverage for filtered to the given paths.
#
# rustc replaced `-Zcoverage-options=mcdc` with `condition`, which emits one branch record per condition but no MC/DC decision records, so
# `llvm-cov --show-mcdc` is empty and `cargo llvm-cov --mcdc` fails on the removed value.
# Condition coverage is the strongest available stand-in, and for a decision of one condition it is exactly MC/DC.
set -euo pipefail

cd "$(dirname "$0")/.."
sources=("${@:-mlt-core/src/codecs}")

toolchain=nightly
sysroot=$(rustc "+$toolchain" --print sysroot)
host=$(rustc "+$toolchain" -vV | sed -n 's/^host: //p')
llvm="$sysroot/lib/rustlib/$host/bin"
[[ -x "$llvm/llvm-cov" ]] || { echo "llvm-tools missing: rustup component add llvm-tools-preview --toolchain $toolchain" >&2; exit 1; }

target=target/mcdc
profraw="$target/profraw"
rm -rf "$profraw"
mkdir -p "$profraw"

export RUSTFLAGS="${RUSTFLAGS:-} -C instrument-coverage -Zcoverage-options=branch,condition"
args=(test --tests -p mlt-core --features unstable-v2 --target-dir "$target")
LLVM_PROFILE_FILE="$PWD/$profraw/%m-%p.profraw" cargo "+$toolchain" "${args[@]}"

mapfile -t bins < <(cargo "+$toolchain" "${args[@]}" --no-run --message-format=json |
    python3 -c 'import json,sys
for line in sys.stdin:
    exe = json.loads(line).get("executable")
    if exe:
        print(exe)')

"$llvm/llvm-profdata" merge -sparse "$profraw"/*.profraw -o "$target/mcdc.profdata"
objects=("${bins[@]:1}")
"$llvm/llvm-cov" report --instr-profile="$target/mcdc.profdata" \
    "${bins[0]}" "${objects[@]/#/--object=}" \
    --show-branch-summary --show-mcdc-summary "${sources[@]}"
