#!/bin/bash
export CARGO_TARGET_DIR=target-wasm
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo +nightly build --target wasm32-unknown-unknown || exit 1
wasm-bindgen --out-dir generated --web target-wasm/wasm32-unknown-unknown/debug/molecular_dynamics.wasm || exit 1
wasm-bindgen --out-dir generated --web target-wasm/wasm32-unknown-unknown/debug/viewer.wasm || exit 1
cp \
    index.html \
    spirv_cross_wrapper_glsl.wasm \
    spirv_cross_wrapper_glsl.js \
    wasmserver.py \
    generated/ || exit 1
cd generated || exit 1
python wasmserver.py || exit 1
