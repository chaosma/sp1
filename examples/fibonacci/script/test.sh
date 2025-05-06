#!/bin/bash

export RUST_LOG=info
export action=$1
# --prove / --compress
RUSTFLAGS="-Ctarget-cpu=native" cargo run --release -- --$action \
  > >(sed -r "s/\x1B\[([0-9]{1,2}(;[0-9]{1,2})?)?[m|K]//g" | tee output.txt) \
  2> >(tee error.txt >&2)
