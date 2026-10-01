#!/usr/bin/env bash
# Usage: ./bench_param.sh <file> <sed-pattern> <value1> [value2] ...
#
# <sed-pattern> uses \1 as the placeholder for the value.
# The script substitutes each value, rebuilds, runs N iterations,
# and reports avg size ± spread and avg time ± spread.
#
# Examples:
#   ./bench_param.sh mlt-core/src/encoder/geometry/encode.rs \
#       's/MORTON_UNIQUENESS_THRESHOLD: f64 = .*/MORTON_UNIQUENESS_THRESHOLD: f64 = \1;/' \
#       0.3 0.4 0.5 0.6
#
#   ./bench_param.sh mlt-core/src/encoder/property/strings.rs \
#       's/FSST_OVERHEAD_THRESHOLD: usize = .*/FSST_OVERHEAD_THRESHOLD: usize = \1;/' \
#       2_048 4_096 8_192

set -euo pipefail

RUNS=1
INPUT="../data/germany.mbtiles"
FILE="$1"; shift
SED_TEMPLATE="$1"; shift
VALUES=("$@")

if [[ ${#VALUES[@]} -eq 0 ]]; then
  echo "Usage: $0 <file> <sed-pattern> <value1> [value2] ..."
  exit 1
fi

printf "%-20s %12s %12s %12s %12s\n" "Value" "Avg (bytes)" "Spread" "Avg (ms)" "Spread (ms)"
printf '%.0s─' {1..72}; echo

for val in "${VALUES[@]}"; do
  sed_cmd="${SED_TEMPLATE//\\1/$val}"
  sed -i "$sed_cmd" "$FILE"

  cargo build --release -p mlt 2>/dev/null

  sizes=()
  times=()
  for ((i=1; i<=RUNS; i++)); do
    out="/tmp/bench_param_$$.mbtiles"
    rm -f "$out"
    start_ns=$(date +%s%N)
    ./target/release/mlt convert "$INPUT" "$out" 1>/dev/null
    end_ns=$(date +%s%N)
    elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))
    times+=("$elapsed_ms")
    s=$(stat --printf='%s' "$out")
    sizes+=("$s")
    rm -f "$out"
  done

  size_csv=$(IFS=,; echo "${sizes[*]}")
  time_csv=$(IFS=,; echo "${times[*]}")
  read -r savg smn smx tavg tmn tmx < <(python3 -c "
s = [$size_csv]
t = [$time_csv]
print(sum(s)//len(s), min(s), max(s), sum(t)//len(t), min(t), max(t))
")
  sspread=$((smx - smn))
  tspread=$((tmx - tmn))
  printf "%-20s %12s %12s %12s %12s\n" "$val" "$savg" "±$sspread" "${tavg}ms" "±${tspread}ms"
done
