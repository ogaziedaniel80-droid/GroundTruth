# GroundTruth

**A tamper-evident anchoring layer for land title records, built on Stellar/Soroban.**

GroundTruth does not replace a country's land registry. It sits alongside it, giving every title document and every transfer of ownership a timestamped, cryptographically verifiable, publicly auditable fingerprint on the Stellar network — so fraud, duplicate titles, and backdated paperwork become detectable instead of invisible.

---

## Table of contents

- [Problem](#problem)
- [Approach](#approach)
- [Why Soroban / Stellar](#why-soroban--stellar)
- [System architecture](#system-architecture)
- [Repository structure](#repository-structure)
- [Data model](#data-model)
- [Smart contract interface](#smart-contract-interface)
- [Trust & security model](#trust--security-model)
- [Getting started](#getting-started)
- [Development workflow](#development-workflow)
- [Deployment](#deployment)
- [Roadmap](#roadmap)
- [FAQ](#faq)
- [Contributing](#contributing)
- [License](#license)

---

## Problem

In many emerging markets, land registries are paper-based, centralized in a single office, or maintained in databases with no audit trail. This produces:

- **Duplicate titles** — the same parcel sold to multiple buyers because there is no single source of truth.
- **Silent record tampering** — a registrar (or someone with access) can alter an ownership record after the fact with no trace.
- **Disputed provenance** — when a dispute goes to court, there is often no reliable way to prove which document existed first, or who transferred what to whom and when.
- **Slow, opaque verification** — buyers, lenders, and courts have no fast way to independently verify a title's history.

Replacing an entire national land registry is a multi-decade institutional project. GroundTruth instead solves the narrower, tractable problem: **make it impossible to quietly rewrite history**, without asking any government to give up control of the registry itself.

## Approach

1. A title document (deed, survey, certificate) is hashed (SHA-256) off-chain. The document itself is never uploaded on-chain.
2. The hash, GPS metadata of the parcel, and a reference to off-chain storage (IPFS CID or encrypted object storage URL) are anchored on a Soroban smart contract as a `TitleRecord`.
3. Every subsequent event — a transfer, a correction, a dispute flag — is recorded as an on-chain transaction, not an in-place edit. The full history is permanent and publicly queryable.
4. Ownership transfers require co-signature from a **registrar multisig** (M-of-N), so no single official can unilaterally rewrite ownership, and no forged transfer can be pushed through without institutional consensus.
5. Anyone — a buyer, a bank, a court, a citizen — can independently recompute the hash of a document they hold and check it against the chain to verify it hasn't been altered since it was anchored.

The government registry remains the legal source of truth. GroundTruth is the tamper-evident witness layer standing next to it.

## Why Soroban / Stellar

- **Cheap, predictable fees** — anchoring millions of parcel records needs transaction costs low enough to be viable at national scale; Stellar's fee model fits this far better than most general-purpose L1s.
- **Fast finality** — 3–5 second settlement means transfer co-signing and verification feel instant to end users.
- **Native multisig primitives** — Soroban's `require_auth` / `require_auth_for_args` and Stellar's account-level multisig make the registrar M-of-N pattern a first-class citizen instead of bespoke contract logic.
- **Composability with identity/KYC anchors** — Stellar's existing ecosystem of anchors, and Soroban's ability to call out to other contracts, make it straightforward to attach a verified-identity credential to an owner address later without re-architecting the registry.
- **Deterministic, auditable execution** — Soroban's resource-metered WASM environment gives predictable, reproducible transaction costs and behavior, which matters when courts or regulators may need to independently verify execution.

## System architecture

```
                         ┌─────────────────────────────┐
                         │        Citizens / Banks      │
                         │     (web app, mobile app)    │
                         └───────────────┬───────────────┘
                                         │
                         ┌───────────────▼───────────────┐
                         │           Frontend             │
                         │   React + Freighter wallet      │
                         │  title lookup · transfer flow   │
                         │  registrar co-sign dashboard    │
                         └───────────────┬───────────────┘
                                         │ REST/GraphQL
                         ┌───────────────▼───────────────┐
                         │            Backend              │
                         │  - document hashing (SHA-256)   │
                         │  - IPFS / object storage pinning│
                         │  - Soroban RPC client            │
                         │  - event indexer → Postgres      │
                         │  - KYC/identity anchor adapter   │
                         └───────────────┬───────────────┘
                                         │ Soroban RPC
                         ┌───────────────▼───────────────┐
                         │      Soroban smart contract     │
                         │        (land-registry)          │
                         │  - TitleRecord storage           │
                         │  - registrar multisig (M-of-N)   │
                         │  - transfer proposal/co-sign flow│
                         │  - full on-chain event log       │
                         └───────────────┬───────────────┘
                                         │
                         ┌───────────────▼───────────────┐
                         │         Stellar network          │
                         └─────────────────────────────────┘
```

Documents and PII never touch the chain — only hashes, coordinates, status, and pseudonymous Stellar addresses do.

## Repository structure

```
land-registry/
├── Cargo.toml                      # Rust workspace manifest
├── README.md
├── contracts/
│   └── land-registry/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs              # contract entry point
│       │   ├── types.rs            # TitleRecord, TransferProposal, DataKey
│       │   ├── registrar.rs        # registrar set + multisig threshold logic
│       │   └── errors.rs           # contract error codes
│       └── tests/
│           └── test_land_registry.rs
├── backend/                        # (planned) Node/TS API + indexer
│   ├── src/
│   │   ├── api/                    # REST endpoints
│   │   ├── chain/                  # Soroban RPC client, tx builders
│   │   ├── indexer/                # event listener → Postgres
│   │   └── storage/                # document hashing + IPFS pinning
│   └── prisma/ or migrations/      # DB schema
├── frontend/                       # (planned) React app
│   ├── src/
│   │   ├── pages/                  # title lookup, transfer, registrar dashboard
│   │   ├── wallet/                 # Freighter integration
│   │   └── components/
│   └── public/
└── scripts/
    ├── deploy.sh                   # contract build + deploy helper
    └── init_registrars.sh          # bootstrap registrar multisig
```

> **Status:** the contract crate scaffolding (`Cargo.toml`, workspace) is in place. Contract logic, backend, and frontend are being built out incrementally — see [Roadmap](#roadmap).

## Data model

**TitleRecord** (on-chain)

| Field         | Type      | Description                                              |
|---------------|-----------|------------------------------------------------------------|
| `id`          | `BytesN<32>` | Deterministic title ID (e.g. hash of parcel reference)  |
| `doc_hash`    | `BytesN<32>` | SHA-256 hash of the title document                       |
| `storage_ref` | `String`  | Off-chain pointer (IPFS CID or encrypted object URL)       |
| `gps_lat`     | `i64`     | Latitude, fixed-point (×10⁷)                                |
| `gps_lng`     | `i64`     | Longitude, fixed-point (×10⁷)                                |
| `owner`       | `Address` | Current owner's Stellar address                            |
| `status`      | `enum`    | `Active`, `PendingTransfer`, `Disputed`, `Revoked`         |
| `created_at`  | `u64`     | Ledger timestamp of first anchoring                        |
| `updated_at`  | `u64`     | Ledger timestamp of last state change                       |

**TransferProposal** (on-chain, ephemeral)

| Field           | Type            | Description                                    |
|-----------------|-----------------|-------------------------------------------------|
| `title_id`      | `BytesN<32>`    | Title being transferred                          |
| `new_owner`     | `Address`       | Proposed new owner                                |
| `proposer`      | `Address`       | Current owner initiating the transfer             |
| `approvals`     | `Vec<Address>`  | Registrars who have co-signed so far              |
| `threshold`     | `u32`           | Approvals required to execute                     |
| `expires_at`    | `u64`           | Ledger timestamp after which proposal lapses      |

Every state transition also emits a Soroban event, giving a complete, independently replayable audit trail without needing to trust the indexer.

## Smart contract interface

```rust
// Admin / setup
fn initialize(env: Env, admin: Address, registrars: Vec<Address>, threshold: u32);
fn add_registrar(env: Env, registrar: Address);
fn remove_registrar(env: Env, registrar: Address);
fn set_threshold(env: Env, threshold: u32);

// Title lifecycle
fn register_title(
    env: Env,
    registrar: Address,
    id: BytesN<32>,
    doc_hash: BytesN<32>,
    storage_ref: String,
    gps_lat: i64,
    gps_lng: i64,
    owner: Address,
) -> TitleRecord;

fn flag_dispute(env: Env, registrar: Address, title_id: BytesN<32>);
fn resolve_dispute(env: Env, registrar: Address, title_id: BytesN<32>, new_status: TitleStatus);

// Transfer flow (registrar multisig)
fn initiate_transfer(env: Env, owner: Address, title_id: BytesN<32>, new_owner: Address) -> TransferProposal;
fn co_sign_transfer(env: Env, registrar: Address, title_id: BytesN<32>);
fn execute_transfer(env: Env, title_id: BytesN<32>); // auto-fires once threshold met
fn cancel_transfer(env: Env, caller: Address, title_id: BytesN<32>);

// Read-only
fn get_title(env: Env, title_id: BytesN<32>) -> TitleRecord;
fn get_pending_transfer(env: Env, title_id: BytesN<32>) -> Option<TransferProposal>;
fn verify_hash(env: Env, title_id: BytesN<32>, candidate_hash: BytesN<32>) -> bool;
```

`register_title` and dispute functions require `registrar.require_auth()`. `initiate_transfer` requires `owner.require_auth()`. `co_sign_transfer` requires `registrar.require_auth()` and is idempotent per registrar (a registrar cannot double-count their own approval).

## Trust & security model

- **What the chain guarantees:** a document hash and GPS metadata existed at a specific ledger time, and that the recorded ownership history has not been silently altered — any edit is a new, timestamped, signed transaction.
- **What the chain does *not* guarantee:** that the underlying document is legally valid, or that the registrar correctly verified the claimant's identity before anchoring. GroundTruth is a tamper-evidence layer, not a legal-validity oracle — the government registry remains the authoritative legal record.
- **Registrar compromise:** a single compromised registrar key cannot move ownership alone; the M-of-N threshold bounds the damage a single bad actor or single stolen key can do. Threshold and registrar set changes should themselves require multisig (handled by `add_registrar`/`remove_registrar`/`set_threshold` being admin- or multisig-gated).
- **Document privacy:** only hashes and coarse GPS coordinates are on-chain. Actual documents live in encrypted off-chain storage; access control to that storage is a backend/infrastructure concern, not a contract concern.
- **Key custody:** citizens and registrars need a credible key-management story (hardware wallets for registrars at minimum) — this is a deployment/operations concern the contract cannot solve by itself.

## Getting started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable) + `wasm32-unknown-unknown` target
- [Stellar CLI](https://developers.stellar.org/docs/tools/cli/install-cli) (`stellar-cli`, formerly `soroban-cli`)
- Node.js 18+ and a package manager (`pnpm` recommended) for backend/frontend once those are scaffolded
- A funded Stellar testnet account for local development

```bash
git clone https://github.com/<your-org>/groundtruth.git
cd groundtruth

rustup target add wasm32-unknown-unknown
```

### Build the contract

```bash
cd contracts/land-registry
cargo build --target wasm32-unknown-unknown --release
```

### Run contract tests

```bash
cargo test
```

### Deploy to testnet

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/land_registry.wasm \
  --source <your-testnet-identity> \
  --network testnet
```

Then initialize with your registrar set:

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <admin-identity> \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --registrars '["<REGISTRAR_1>","<REGISTRAR_2>","<REGISTRAR_3>"]' \
  --threshold 2
```

## Development workflow

1. Work on contract logic under `contracts/land-registry/src/`, with unit tests in `contracts/land-registry/tests/` using `soroban-sdk`'s `testutils`.
2. Once the contract stabilizes, generate TypeScript bindings (`stellar contract bindings typescript`) for the backend/frontend to consume.
3. Backend indexer subscribes to contract events via Soroban RPC's `getEvents` and materializes them into Postgres for fast querying (full history always remains re-verifiable directly from the chain).
4. Frontend never trusts the indexer for anything security-critical — verification actions (e.g. "does this document match the chain?") always recompute the hash client-side and check it against a direct RPC read of `get_title`.

## Deployment

| Environment | Network            | Notes                                                        |
|-------------|---------------------|----------------------------------------------------------------|
| Local       | Standalone/Futurenet | Fast iteration, ephemeral state                                |
| Staging     | Testnet              | Pilot with a real registrar office, synthetic titles            |
| Production  | Pubnet (Mainnet)     | Requires audited contract, registrar key ceremony, legal sign-off |

Contract upgrades should go through Soroban's contract-versioning/migration pattern rather than redeploying a fresh contract ID, so existing `TitleRecord`s remain addressable.

## Roadmap

- [x] Repository and workspace scaffolding
- [ ] Core contract: title registration + read APIs
- [ ] Registrar multisig + transfer proposal/co-sign flow
- [ ] Dispute flagging and resolution states
- [ ] Contract unit + integration test suite
- [ ] Backend: document hashing service + IPFS pinning
- [ ] Backend: Soroban event indexer + Postgres schema
- [ ] Frontend: title lookup and public verification page
- [ ] Frontend: registrar multisig co-sign dashboard
- [ ] Freighter wallet integration
- [ ] KYC/identity anchor integration
- [ ] Testnet pilot with a partner registrar office
- [ ] External security audit ahead of mainnet deployment

## FAQ

**Does this replace the land registry?**
No. It anchors evidence of what the registry recorded and when, so tampering becomes detectable. Legal title still flows from the government registry.

**What happens if a registrar loses their key?**
The M-of-N threshold means the remaining registrars can still operate; `add_registrar`/`remove_registrar` (itself governed by admin or multisig policy) is used to rotate in a replacement.

**Can someone forge a GPS location?**
The contract can't independently verify GPS truth — that's a registrar-side data-quality responsibility at the point of registration. The contract guarantees that whatever was submitted at anchoring time cannot be silently changed afterward.

**What if the off-chain document storage goes down?**
The on-chain hash still lets anyone verify a document they already hold. Redundant pinning (multiple IPFS pinning services, or a backend mirror) is recommended for availability.

## Contributing

Issues and PRs are welcome. For contract changes, please include tests using `soroban-sdk::testutils`. For anything touching the multisig or transfer flow, include a written threat-model note in the PR description.

## License

Apache-2.0
