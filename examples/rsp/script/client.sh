#!/bin/bash

# Configuration
SERVER_URL="http://127.0.0.1:3000"
PROOF_DIR="./proofs"

# Usage function
usage() {
  echo "Usage: $0 <command> [options]"
  echo "Commands:"
  echo "  first-layer <index> <thread|fork>  Test first-layer compression"
  echo "  two-to-one <index1> <index2> <thread|fork>  Test two-to-one compression"
  echo "Examples:"
  echo "  $0 first-layer 1 thread"
  echo "  $0 two-to-one 1 2 fork"
  exit 1
}

# Check if curl is installed
if ! command -v curl &>/dev/null; then
  echo "Error: curl is required but not installed."
  exit 1
fi

# Check if proof files exist for two-to-one
check_proof_files() {
  local index1=$1
  local index2=$2
  local file1="${PROOF_DIR}/reduced_0_${index1}.bin"
  local file2="${PROOF_DIR}/reduced_0_${index2}.bin"
  if [ ! -f "$file1" ] || [ ! -f "$file2" ]; then
    echo "Error: Proof files not found: $file1, $file2"
    exit 1
  fi
}

# Test first-layer compression
test_first_layer() {
  local index=$1
  local mode=$2
  if [ "$mode" != "thread" ] && [ "$mode" != "fork" ]; then
    echo "Error: Mode must be 'thread' or 'fork'"
    exit 1
  fi
  echo "Testing first-layer compression with index=$index, mode=$mode"
  time curl -s -X POST "${SERVER_URL}/first-layer/${index}/${mode}" \
    -w "\nHTTP Status: %{http_code}\n" \
    -o response.txt
  cat response.txt
  rm -f response.txt
}

# Test two-to-one compression
test_two_to_one() {
  local index1=$1
  local index2=$2
  local mode=$3
  if [ "$mode" != "thread" ] && [ "$mode" != "fork" ]; then
    echo "Error: Mode must be 'thread' or 'fork'"
    exit 1
  fi
  check_proof_files "$index1" "$index2"
  echo "Testing two-to-one compression with index1=$index1, index2=$index2, mode=$mode"
  time curl -s -X POST "${SERVER_URL}/two-to-one/${mode}" \
    -H "Content-Type: application/json" \
    -d "{\"index1\": $index1, \"index2\": $index2}" \
    -w "\nHTTP Status: %{http_code}\n" \
    -o response.txt
  cat response.txt
  rm -f response.txt
}

# Main logic
if [ $# -lt 2 ]; then
  usage
fi

case "$1" in
first-layer)
  if [ $# -ne 3 ]; then
    echo "Error: first-layer requires index and mode (thread|fork)"
    usage
  fi
  test_first_layer "$2" "$3"
  ;;
two-to-one)
  if [ $# -ne 4 ]; then
    echo "Error: two-to-one requires index1, index2, and mode (thread|fork)"
    usage
  fi
  test_two_to_one "$2" "$3" "$4"
  ;;
*)
  usage
  ;;
esac
