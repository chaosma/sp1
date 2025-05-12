#!/usr/bin/env bash
# benchmark.sh  N  TYPE[first-layer|two-to-one]  [ADDR]

set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <N> <first-layer|two-to-one> [address]" >&2
  exit 1
fi

N=$1
TYPE=$2
ADDR=${3:-127.0.0.1:3000}

url() {
  printf "http://%s%s" "$ADDR" "$1"
}

echo "Benchmarking ${TYPE} with N=${N} against ${ADDR}"

start_ns=$(date +%s%N) # nanoseconds since epoch

for ((i = 0; i < N; i++)); do
  if [[ "$TYPE" == "first-layer" ]]; then
    resp=$(curl -sS -X POST "$(url /first-layer/"$i")")
  elif [[ "$TYPE" == "two-to-one" ]]; then
    idx1=$((i * 2))
    idx2=$((i * 2 + 1))
    json=$(printf '{"index1":%d,"index2":%d}' "$idx1" "$idx2")
    resp=$(curl -sS -H 'Content-Type: application/json' \
      -d "$json" -X POST "$(url /two-to-one)")
  else
    echo "unknown TYPE: $TYPE" >&2
    exit 1
  fi

  if [[ "$resp" != "ok" ]]; then
    echo "request $i failed, response: $resp" >&2
    exit 1
  fi
done

end_ns=$(date +%s%N)
elapsed_ms=$(((end_ns - start_ns) / 1000000))
elapsed_s=$(printf "%.3f" "$(bc -l <<<"$elapsed_ms/1000")")

echo "Completed $N requests – total ${elapsed_s}s (${elapsed_ms} ms)"
