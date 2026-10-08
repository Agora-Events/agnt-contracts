# Agnt Contracts 🤖💳

**Agnt** is an open-source policy-enforced spending account for AI agents on Stellar/Soroban.

A human owner funds the Agnt smart contract account with USDC and provisions restricted Ed25519 keys for AI agents. Spending policies (per-tx caps, daily caps, allowed payees, expiration) are enforced directly **on-chain inside `__check_auth`**.

---

## 🔑 Features & Architectural Overview

- **Soroban Custom Account (`CustomAccountInterface`)**: Replaces standard account keys with programmable contract-based authentication.
- **On-Chain Policy Enforcement**: Limits are verified atomically before any transaction is authorized.
- **Stroop / 7-Decimal Standard**: All USDC token amounts are denominated in stroops (7 decimals, where $1\text{ USDC} = 10,000,000\text{ stroops}$).
- **Daily Spending Bucket Tracking**: Tracks daily spend reset automatically based on ledger timestamp (`timestamp / 86400`).
- **Owner Governance**: Owner can add, update, and revoke agent keys at any time.

> [!NOTE]
> **State Writes in `__check_auth`**: In Soroban, `__check_auth` is a reserved system function called strictly by the host environment during `require_auth` verification. State mutations inside `__check_auth` (such as persistent spend tracker updates) are fully supported and safe.

---

## 📁 Repository Structure

```
.
├── Cargo.toml                      # Workspace manifest
├── README.md                       # Project overview & documentation
├── LICENSE                         # MIT License
├── .env.example                    # Environment variable template
├── .gitignore
├── contracts/
│   └── agnt-account/
│       ├── Cargo.toml              # Contract Crate manifest
│       └── src/
│           ├── lib.rs              # Contract entrypoint & CustomAccountInterface impl
│           ├── policy.rs           # Data structures (Policy, SpendTracker, Signature)
│           ├── storage.rs          # DataKey definitions & storage access helpers
│           ├── errors.rs           # Typed #[contracterror] enum
│           ├── events.rs           # Event emission functions
│           └── test.rs             # Full unit & integration test suite
└── scripts/
    ├── deploy-testnet.sh           # Testnet deployment script
    └── fund-and-test.md            # Manual funding & testing guide
```

---

## 📋 Function Reference Table

| Function | Access | Description |
| :--- | :--- | :--- |
| `__constructor(owner, usdc)` | Init | Initializes contract with owner public key (`BytesN<32>`) and USDC SAC address (`Address`). |
| `add_agent(agent, policy)` | Owner | Registers an AI agent with a spending `Policy`. |
| `update_policy(agent, policy)` | Owner | Updates an existing agent's policy limits or parameters. |
| `revoke_agent(agent)` | Owner | Revokes an agent key immediately (`policy.revoked = true`). |
| `get_agent(agent) -> Policy` | Public | Returns the current policy struct for a given agent. |
| `get_spent_today(agent) -> i128` | Public | Returns the current day's spent amount in stroops. |
| `__check_auth(payload, sig, contexts)` | Host | Reserved custom account entrypoint enforcing signature verification and policy rules. |

---

## 🚨 Error Codes (`AgntError`)

| Code | Variant | Reason |
| :--- | :--- | :--- |
| `1` | `NotOwner` | Signer is not the contract owner. |
| `2` | `UnknownSigner` | Public key is neither owner nor a registered agent. |
| `3` | `AgentRevoked` | Agent key has been revoked by the owner. |
| `4` | `PolicyExpired` | Current ledger timestamp exceeds `policy.expires_at`. |
| `5` | `ContextNotAllowed` | Invocation context is not a USDC `transfer` call from this account. |
| `6` | `PayeeNotAllowed` | Recipient address `to` is not in the agent's payee allowlist. |
| `7` | `OverPerTxCap` | Transfer amount exceeds `policy.per_tx_cap`. |
| `8` | `OverDailyCap` | Transfer amount causes total daily spend to exceed `policy.daily_cap`. |
| `9` | `InvalidAmount` | Transfer amount is $\le 0$. |
| `10` | `BadSignature` | Ed25519 signature verification failed. |

---

## 🛠️ Build & Test Commands

### Prerequisites
- [Rust](https://rustup.rs/) (with `wasm32-unknown-unknown` target)
- [`stellar-cli`](https://developers.stellar.org/docs/smart-contracts/getting-started/setup)

### Commands

```bash
# Run unit & integration tests
cargo test

# Build WASM binary
cargo build --target wasm32-unknown-unknown --release

# Or using stellar-cli
stellar contract build
```

---

## 🚀 Deployment (Testnet Only)

Copy `.env.example` to `.env` and fill in your testnet account details:

```bash
cp .env.example .env
```

Run the deployment script:

```bash
bash scripts/deploy-testnet.sh
```

For manual testing and funding instructions, see [scripts/fund-and-test.md](file:///c:/Users/Danii/Desktop/agnt-contracts/scripts/fund-and-test.md).

---

## 📜 License

Distributed under the [MIT License](file:///c:/Users/Danii/Desktop/agnt-contracts/LICENSE).
