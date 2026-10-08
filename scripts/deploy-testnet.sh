#!/usr/bin/env bash
set -euo pipefail

# Agnt Account Testnet Deployment Script
# Requirements: stellar-cli, rust target wasm32-unknown-unknown

if [ -f .env ]; then
  source .env
else
  echo "Error: .env file not found. Please create one from .env.example"
  exit 1
fi

if [ -z "${SOURCE_ACCOUNT:-}" ]; then
  echo "Error: SOURCE_ACCOUNT is not set in .env"
  exit 1
fi

if [ -z "${USDC_SAC_ADDRESS:-}" ]; then
  echo "Error: USDC_SAC_ADDRESS is not set in .env"
  exit 1
fi

echo "Building WASM contract..."
stellar contract build

WASM_FILE="target/wasm32-unknown-unknown/release/agnt_account.wasm"

echo "Deploying contract to Testnet..."
CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM_FILE" \
  --source "$SOURCE_ACCOUNT" \
  --network "${NETWORK:-testnet}")

echo "Contract deployed with ID: $CONTRACT_ID"

echo "Initializing contract with owner key and USDC address..."
OWNER_PK=$(stellar keys public-key "$SOURCE_ACCOUNT")

stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source "$SOURCE_ACCOUNT" \
  --network "${NETWORK:-testnet}" \
  -- \
  __constructor \
  --owner "$OWNER_PK" \
  --usdc "$USDC_SAC_ADDRESS"

echo "Agnt account contract initialization complete!"
echo "Contract ID: $CONTRACT_ID"
