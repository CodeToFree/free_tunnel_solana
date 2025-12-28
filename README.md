# free_tunnel_solana

`free_tunnel_solana` is a native Solana program (non-Anchor) that implements a simple bridge “tunnel” with:

- A Solana **admin** that configures the program (tokens, proposers, executor sets).
- A set of **proposers** that can create bridge requests on-chain.
- A rotating **executor set** (Ethereum `secp256k1` addresses) that authorizes execution via threshold signatures.

The program supports two deployment modes:

- **Mint mode** (`mint_or_lock = true`): propose/execute **mint** and **burn** flows.
- **Lock mode** (`mint_or_lock = false`): propose/execute **lock** and **unlock** flows.

Token operations work with both **SPL Token** and **Token-2022** programs.

---

## Key Concepts

### Roles

- **Admin**: initializes the program, can transfer admin, manage proposers, manage tokens, and update executor sets.
- **Proposers**: submit requests (`req_id`) to be executed later.
- **Executors (EVM addresses)**: provide `secp256k1` signatures that must meet the configured threshold.

### Program Accounts (PDAs)

All program state is stored in PDAs derived from the deployed `program_id`.

- **Basic storage**: `PDA([b"basic-storage"])`
  - Stores: mode (mint/lock), admin, proposers, token list, per-token vault ATA, decimals, locked balances, and executor-set length.
- **Contract signer**: `PDA([b"contract-signer"])`
  - Used as the authority for vault ATAs and token operations (via `invoke_signed`).
- **Executors info**: `PDA([b"executors", exe_index_le_bytes])`
  - Stores: `threshold`, `active_since`, `inactive_after`, and the executor address list.
- **Per-request PDAs**:
  - Mint request: `PDA([b"mint", req_id_bytes])`
  - Burn request: `PDA([b"burn", req_id_bytes])`
  - Lock request: `PDA([b"lock", req_id_bytes])`
  - Unlock request: `PDA([b"unlock", req_id_bytes])`

Account data is stored as: `u32_le_length_prefix || borsh_payload`.

---

## Instruction Set (High Level)

Instruction data is `borsh` serialized with a 1-byte variant prefix (see `src/instruction.rs`).

### Admin / Configuration

- `Initialize { is_mint_contract, executors, threshold, exe_index }`
  - Creates `basic-storage` and the initial `executors` PDA for `exe_index`.
- `TransferAdmin { new_admin }`
- `AddProposer { new_proposer }` / `RemoveProposer { proposer }`
- `UpdateExecutors { new_executors, threshold, active_since, signatures, executors, exe_index }`
  - Executor rotation is time-gated and must be authorized by the current executor set.
- `AddToken { token_index }`
  - Creates the contract vault ATA (owned by the contract signer PDA) and stores mint/vault/decimals.
- `RemoveToken { token_index }`
  - Requires vault balance and locked balance to be zero.

### Mint Mode (mint/burn)

- `ProposeMint { req_id, recipient }` → `ExecuteMint { req_id, signatures, executors, exe_index }` → `CancelMint { req_id }`
- `ProposeBurn { req_id }` → `ExecuteBurn { req_id, signatures, executors, exe_index }` → `CancelBurn { req_id }`

### Lock Mode (lock/unlock)

- `ProposeLock { req_id }` → `ExecuteLock { req_id, signatures, executors, exe_index }` → `CancelLock { req_id }`
- `ProposeUnlock { req_id, recipient }` → `ExecuteUnlock { req_id, signatures, executors, exe_index }` → `CancelUnlock { req_id }`

---

## `req_id` Format and Signing

`req_id` is a 32-byte identifier with a compact layout (see `src/logic/req_helpers.rs`):

- Byte `0`: version
- Bytes `1..6`: created time (`uint40`, big-endian)
- Byte `6`: action (low 4 bits are used to distinguish lock-mint / burn-unlock / burn-mint)
- Byte `7`: token index
- Bytes `8..16`: amount (`u64`, big-endian) in a 6-decimal “bridge unit” (adjusted to mint decimals on-chain)
- Bytes `16` and `17`: “from/to” hub routing bytes (validated against `HUB_ID`)

Executors authorize execution by signing an **EIP-191 style** message that is constructed on-chain from `req_id` and the bridge channel label (see `ReqId::msg_from_req_signing_message`). The program verifies signatures by recovering an Ethereum address via `secp256k1_recover`.

Signature format note: the program expects a 64-byte “compact” signature where the recovery id is encoded in the highest bit of byte `32` (the first byte of `s`), matching the logic in `SignatureUtils::recover_eth_address`.

If you are building an off-chain relayer/client, implement the message construction exactly as in:

- `src/logic/req_helpers.rs` (execute lock-mint / burn-unlock / burn-mint)
- `src/logic/permissions.rs` (update executors message)

---

## Build, Test, Deploy

### Prerequisites

- Rust toolchain (this repo pins `nightly` via `rust-toolchain.toml`)
- Solana CLI / SBF build tools such that `cargo build-sbf` is available

### Build (SBF)

```bash
cargo build-sbf
```

Output: `target/deploy/free_tunnel_solana.so`

### Unit Tests

```bash
cargo test
```

### Deploy (Local Validator)

```bash
solana-test-validator --reset
solana config set --url localhost
solana program deploy target/deploy/free_tunnel_solana.so
```

### pnpm helpers

This repo includes `pnpm` scripts as convenience wrappers:

```bash
pnpm build
pnpm deploy
```

`pnpm deploy` assumes you maintain local Solana CLI config/key material under `.solana/` (ignored by git). Do not commit keypairs to the repository.

---

## Notes

- Limits (hardcoded): max 32 proposers, 32 executors, 32 tokens (see `src/constants.rs`).
- This code has not been audited; use at your own risk.

---

## License

MIT
