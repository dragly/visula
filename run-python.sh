#!/bin/bash
pushd visula_pyo3 && uv run maturin develop --uv && uv run "../$1"; popd
