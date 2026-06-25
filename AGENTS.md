# AI Agent Guide — Stellar Unified Price Oracle Aggregator

> **Note for AI agents:** This file is your primary onboarding document. Read it fully before making any changes.

## Project Overview

A Soroban (Stellar smart contract) **price oracle aggregator** that collects price data from multiple permissioned oracle sources, aggregates via **median**, and exposes price data (current + historical) for consumer contracts via SEP-40.

- **Language:** Rust, compiled to WASM (`wasm32v1-none`)
- **SDK:** soroban-sdk v26
- **Testing:** `#[cfg(test)]` with `soroban-sdk/testutils`

## Repository Structure

```
.
├── .github/workflows/ci.yml    # CI: frontend / backend / contract
├── .husky/pre-push              # Pre-push hook: builds + tests contract
├── AGENTS.md                    # This file
├── Cargo.toml                   # Workspace root
├── package.json                 # Root scripts + husky devDep
├── contracts/
│   └── price-oracle/
│       ├── .cargo/config.toml   # WASM link flags
│       ├── Cargo.toml
│       └── src/
│           ├── admin.rs         # Admin management functions
│           ├── assets.rs        # Asset registration and management
│           ├── errors.rs        # Error types and handling
│           ├── events.rs        # Contract event definitions
│           ├── history.rs       # Price history management
│           ├── lib.rs           # Contract entrypoint + all endpoints
│           ├── prices.rs        # Price submission and aggregation
│           ├── prop_tests.rs    # Property-based tests (5 tests)
│           ├── sources.rs       # Oracle source management
│           ├── storage.rs       # Storage helpers + median computation
│           ├── test.rs          # Test suite (71 tests)
│           ├── test_helpers.rs  # Shared test utilities
│           └── types.rs         # DataKey, error codes, structs
├── frontend/                    # (not yet created — place frontend here)
├── backend/                     # (not yet created — place backend here)
```

### Source Code Map

| File | Purpose |
|---|---|
| `lib.rs` | Contract struct, all 26 public endpoints, SEP-40 interface. Imports helpers from sibling modules. |
| `types.rs` | `DataKey` enum (storage keys), `ErrorCode` enum, all data structs (`PriceEntry`, `AggregatePrice`, `PriceHistoryEntry`, `OracleSources`, `Asset`, `PriceData`) |
| `storage.rs` | `get_admin`, `check_source`, `check_registered_asset`, median sorting/`compute_median`, `read_*`/`write_*` helpers |
| `admin.rs` | Admin management functions |
| `assets.rs` | Asset registration and management |
| `errors.rs` | Error types and handling |
| `events.rs` | 13 event types + 2 manual publish functions (`emit_initialized`, `emit_timestamp_threshold_changed`) |
| `test.rs` | 76 unit tests covering admin, sources, assets, submissions, queries, history, SEP-40, auth, upgrades |

## What NOT to Push

- **Never commit** files under `target/`, `node_modules/`, `test_snapshots/`, or `.kiro/` (they are gitignored)
- **Never commit** API keys, secrets, or identity files
- **Never commit** `.env` files or wallet keypairs
- **Never commit** WASM binaries (they are build artifacts — `*.wasm` is gitignored)

## What to Push

- Rust source code (`.rs` files under `contracts/`)
- `Cargo.toml` / `Cargo.lock` (lockfile should be committed for deterministic builds)
- `package.json` / `package-lock.json` (if frontend/backend are added)
- `AGENTS.md`, `CONTRIBUTING.md`, `README.md`, `LICENSE`
- `.github/` workflows, issue templates, PR templates
- `.husky/` hooks and config
- `.gitignore`

## How to Verify Task Completion

Before marking any task complete, run the full verification suite:

```bash
# 1. Build the contract
cargo build -p price-oracle --target wasm32v1-none --release

# 2. Run all tests
cargo test -p price-oracle --lib

# 3. Check for lint/style issues
cargo clippy -p price-oracle -- -D warnings
cargo fmt --manifest-path contracts/price-oracle/Cargo.toml -- --check

# 4. If frontend/backend exist, also build them
[ -d frontend ] && cd frontend && npm ci && npm run build
[ -d backend ] && { [ -f backend/Cargo.toml ] && cargo build --manifest-path backend/Cargo.toml || [ -f backend/package.json ] && cd backend && npm ci && npm run build; }
```

All 76 tests must pass with zero compiler warnings and zero clippy warnings.

## Key Constraints

- `#![no_std]` — no standard library available in contract code
- `wasm32v1-none` target — only `soroban-sdk` and `core` dependencies
- Storage uses `env.storage().persistent()` and `env.storage().temporary()`
- Admin functions require `admin.require_auth()`
- Source submit functions require `source.require_auth()`
