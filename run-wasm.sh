#!/bin/bash
export CARGO_TARGET_DIR=target-wasm
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo +nightly build --target wasm32-unknown-unknown || exit 1
wasm-bindgen --out-dir generated --web target-wasm/wasm32-unknown-unknown/debug/visula.wasm || exit 1
cp index.html generated/ || exit 1
cp wasmserver.py generated/ || exit 1
cd generated || exit 1
python wasmserver.py || exit 1
