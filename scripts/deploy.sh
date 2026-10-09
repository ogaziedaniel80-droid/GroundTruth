#!/usr/bin/env bash
# scripts/deploy.sh
#
# Build and deploy the land-registry contract to standalone, futurenet, or
# testnet. Reproduces exactly the steps shown in the README "Getting started"
# section, parameterized by NETWORK.
#
# Usage:
#   NETWORK=testnet SOURCE=my-identity ./scripts/deploy.sh
#
# Environment variables:
#   NETWORK   – stellar network target: standalone | futurenet | testnet  (default: testnet)
#   SOURCE    – stellar identity name or secret key used to sign the deploy tx
#
# Prerequisites:
#   - Rust + wasm32v1-none target: rustup target add wasm32v1-none
#   - Stellar CLI: https://developers.stellar.org/docs/tools/cli/install-cli
#   - A funded account for SOURCE on the target NETWORK

set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:-}"
WASM_PATH="target/wasm32v1-none/release/land_registry.wasm"

if [[ -z "$SOURCE" ]]; then
  echo "ERROR: SOURCE environment variable must be set (your stellar identity name or secret key)."
  exit 1
fi

echo "==> Building contract for wasm32v1-none (release)..."
cargo build --target wasm32v1-none --release

echo "==> Deploying to ${NETWORK}..."
CONTRACT_ID=$(stellar contract deploy \
  --wasm "${WASM_PATH}" \
  --source "${SOURCE}" \
  --network "${NETWORK}" \
  2>&1 | tail -1)

echo ""
echo "Contract deployed successfully."
echo "CONTRACT_ID=${CONTRACT_ID}"
echo ""
echo "Next step: run scripts/init_registrars.sh with CONTRACT_ID=${CONTRACT_ID}"
