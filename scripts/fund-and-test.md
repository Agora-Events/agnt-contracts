# Agnt Account: Manual Testnet Funding and Agent Setup Guide

This guide walks you through funding an Agnt smart account on the Stellar Testnet and registering an AI agent with spending policy rules.

---

## 1. Prerequisites

- Installed `stellar-cli`
- Testnet account identity created (`stellar keys generate owner_key`)
- Testnet XLM funded (`stellar keys fund owner_key`)

---

## 2. Obtain Testnet USDC (Stellar Asset Contract)

1. Find or deploy the USDC Stellar Asset Contract (SAC) on Testnet.
2. Mint testnet USDC to your `owner_key` or source account via Friendbot / Testnet Issuer.
3. Transfer USDC to your deployed Agnt contract address (`CONTRACT_ID`).

```bash
# Example: Send 100 USDC to your Agnt Account contract
stellar contract invoke \
  --id $USDC_SAC_ADDRESS \
  --source owner_key \
  --network testnet \
  -- \
  transfer \
  --from owner_key \
  --to $CONTRACT_ID \
  --amount 1000000000
```
*(Note: Amounts use 7 decimal places / stroops. `1000000000 stroops = 100 USDC`).*

---

## 3. Generate Agent Keypair

Generate an ed25519 keypair for your AI agent:

```bash
stellar keys generate agent_key
AGENT_PK=$(stellar keys public-key agent_key)
```

---

## 4. Register the Agent with Policy Rules

Add the agent to your Agnt account with spending restrictions:

- **Per-transaction Cap**: 10 USDC (`100_000_000 stroops`)
- **Daily Cap**: 50 USDC (`500_000_000 stroops`)
- **Allowed Payee**: Target merchant / API service address
- **Expiration**: Unix timestamp in seconds

```bash
stellar contract invoke \
  --id $CONTRACT_ID \
  --source owner_key \
  --network testnet \
  -- \
  add_agent \
  --agent $AGENT_PK \
  --policy '{
    "per_tx_cap": 100000000,
    "daily_cap": 500000000,
    "payees": ["<TARGET_PAYEE_ADDRESS>"],
    "expires_at": 1800000000,
    "revoked": false
  }'
```

---

## 5. Agent Executes a Payment

The agent constructs and signs a Soroban transaction invoking `transfer` on the USDC contract with `from = CONTRACT_ID`.

The Soroban runtime triggers `__check_auth` on `CONTRACT_ID`. The contract validates the agent's signature, checks the policy limits, updates daily spent tracking, and approves the transfer!

---

## 6. Inspect Daily Spent Balance

You can check an agent's current spending for today at any time:

```bash
stellar contract invoke \
  --id $CONTRACT_ID \
  --source owner_key \
  --network testnet \
  -- \
  get_spent_today \
  --agent $AGENT_PK
```
