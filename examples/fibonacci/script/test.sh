#!/bin/bash

export RUST_LOG=info

if [ "$1" == "verify" ]; then
    FLAGS="--load-shards"
else
    FLAGS=""
    for arg in "$@"; do
        FLAGS="$FLAGS --$arg"
    done
fi

RUSTFLAGS="-Ctarget-cpu=native" cargo run -p fibonacci-script --release -- $FLAGS \
  > >(sed -r "s/\x1B\[([0-9]{1,2}(;[0-9]{1,2})?)?[m|K]//g" | tee output.txt) \
  2> >(tee error.txt >&2)
