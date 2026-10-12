#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

EXAMPLES=(
  clay
  debug_cylinder
  gltf_viewer
  grid
  line
  mesh
  molecular_dynamics
  neuron
  neuronify
  painter
  plane
  showcase
  svg_icons
  texture
  toon_neurons
  viewer
)

PROFILE_FLAG=""
if [[ "${1:-}" == "--release" ]]; then
  PROFILE_FLAG="--release"
fi

rm -rf dist
for ex in "${EXAMPLES[@]}"; do
  echo "==> Building example: $ex"
  trunk build $PROFILE_FLAG --example "$ex" --dist "dist/$ex" --public-url "/$ex/"
done

cp web/launcher.html dist/index.html
echo "==> Done. Serve dist/ with any static file server."
