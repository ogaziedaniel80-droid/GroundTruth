#!/usr/bin/env bash
# scripts/init_registrars.sh
#
# Initialize a freshly deployed land-registry contract with an admin address,
# registrar set, and multisig threshold. Reproduces exactly the initialization
# command shown in the README "Getting started" section, parameterized for
# standalone, futurenet, and testnet.
#
# Usage:
#   CONTRACT_ID=C... ADMIN=G... REGISTRARS='["G1","G2","G3"]' THRESHOLD=2 \
#     NETWORK=testnet SOURCE=my-identity ./scripts/init_registrars.sh
#
# Environment variables:
#   CONTRACT_ID  – contract address returned by deploy.sh
#   ADMIN        – Stellar address of the contract admin (G...)
#   REGISTRARS   – JSON array of registrar Stellar addresses, e.g. '["G1","G2","G3"]'
#   THRESHOLD    – M in M-of-N multisig (must be <= number of registrars)
#   NETWORK      – standalone | futurenet | testnet  (default: testnet)
#   SOURCE       – stellar identity signing the init transaction

set -euo pipefail

CONTRACT_ID="${CONTRACT_ID:-}"
ADMIN="${ADMIN:-}"
REGISTRARS="${REGISTRARS:-}"
THRESHOLD="${THRESHOLD:-2}"
NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:-}"

for var in CONTRACT_ID ADMIN REGISTRARS SOURCE; do
  if [[ -z "${!var}" ]]; then
    echo "ERROR: ${var} environment variable must be set."
    exit 1
  fi
done

echo "==> Initializing contract ${CONTRACT_ID} on ${NETWORK}..."
echo "    Admin:      ${ADMIN}"
echo "    Registrars: ${REGISTRARS}"
echo "    Threshold:  ${THRESHOLD}"

stellar contract invoke \
  --id "${CONTRACT_ID}" \
  --source "${SOURCE}" \
  --network "${NETWORK}" \
  -- initialize \
  --admin "${ADMIN}" \
  --registrars "${REGISTRARS}" \
  --threshold "${THRESHOLD}"

echo ""
echo "Contract initialized successfully."
echo "The registrar multisig threshold is ${THRESHOLD}-of-$(echo "${REGISTRARS}" | python3 -c 'import sys,json; print(len(json.load(sys.stdin)))' 2>/dev/null || echo "N")."
