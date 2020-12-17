#!/bin/bash
pushd src
glslc shader.vert -o shader.vert.spv || exit $?
glslc shader.frag -o shader.frag.spv || exit $?
glslc mesh.vert -o mesh.vert.spv || exit $?
glslc mesh.frag -o mesh.frag.spv || exit $?
popd
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo +nightly run || exit $?
