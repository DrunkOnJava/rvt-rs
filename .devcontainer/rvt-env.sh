#!/bin/sh
export PATH="$HOME/.local/bin:$PATH"
export CARGO_BUILD_JOBS=2
export RAYON_NUM_THREADS=2
export RVT_BUILD_PROFILE=ci
export RVT_WASM_PROFILE=dev
