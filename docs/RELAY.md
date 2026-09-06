# vapurr-relay — gasless meta-transactions

Users sign a `ForwardRequest` for free (EIP-712, no gas, no transaction).
An authorized relayer batches many of them into one call to
`VapurrForwarder.executeBatch` and pays the gas itself. Batching is the
entire savings mechanism — one shared base-transaction cost instead of
one per user — not a third-party compute-offload service. See
`crates/vapurr-relay/src/fee.rs` for why "up to 50%" needs a number, not
an assertion (short version below).

## Pieces

| Piece | What it is |
|---|---|
| `contracts/VapurrForwarder.sol` | ERC-2771-shaped minimal forwarder. `execute`/`executeBatch` gated to `authorizedRelayers`, owner (treasury) controls that set on-chain. |
| `crates/vapurr-relay` | The relayer itself — a Rust/axum/tokio service. Accepts signed requests, batches them, submits with its own key. |
| `contracts/compile-forwarder.mjs` | Compiles the contract, writes `crates/vapurr-relay/src/forwarder.{hex,abi.json}`. |

## Security model — the part that was originally asked for as "kernel-level"

The actual requirement — only one specific signer can move things through
this system, and that authority is revocable without redeploying — is
enforced on-chain, not by process privilege level:

- `VapurrForwarder.authorizedRelayers` is an owner-controlled allowlist.
  The relayer's hot key gets added there once; if it's ever compromised,
  the owner (should be a multisig, not an EOA) calls `setRelayer(key, false)`
  and it's cut off immediately — no redeploy, no waiting on the process
  itself to notice anything.
- `execute`/`executeBatch` check `authorizedRelayers[msg.sender]` before
  touching anything. The relayer process itself holds no special
  privilege beyond "an address on this list" — losing the process, or
  running five of them, changes nothing about what's allowed on-chain.
- Every `ForwardRequest` is independently signed and nonce-checked
  regardless of who submits it — the relayer relaying garbage doesn't
  forge a user's intent, it just wastes its own gas trying.

**"Never goes down"** is a process-supervision problem, not a privilege-
ring one: run `vapurr-relay` under systemd with `Restart=always`, point
health checks at `GET /health`, and treat the process as disposable — its
only durable state is on-chain (nonces, the relayer allowlist); the
in-memory pending queue re-fills from client retries if it restarts.
**A hand-written kernel-mode component was not built, on purpose** — see
the conversation this came out of: a bug in kernel space doesn't fail a
transaction, it can take down or compromise the whole machine, which is a
strictly worse failure mode for a system that's supposed to be reliable,
not just fast.

```ini
# /etc/systemd/system/vapurr-relay.service (example)
[Service]
ExecStart=/opt/vapurr/bin/vapurr-relay
Restart=always
RestartSec=2
User=vapurr-relay
# secrets via EnvironmentFile, never inline here
EnvironmentFile=/etc/vapurr-relay/env
```

**Not built in this pass, and both are real work:**
- **Multi-instance HA.** Running two relayer processes against the same
  hot key needs nonce coordination (both would race
  `eth_getTransactionCount`) — either a single active instance with
  leader election, or splitting the relayer's own nonce space, or a
  per-instance sub-key each independently authorized on-chain. Pick one
  before running more than one instance.
- **Billing collection.** `fee.rs::user_fee_gas` computes what a user
  *should* pay for sponsorship; nothing here actually charges them yet.
  The natural integration is vapurr-pay's existing x402/KetPay rail
  (charge in `$PUSD`), but that's a separate change.

## The honest gas-savings math

`FORWARDER_PER_ITEM_OVERHEAD_GAS` in `fee.rs` is a rough EVM-opcode
estimate (~13k gas: ecrecover + a nonce SLOAD+SSTORE + the external CALL +
an event), not a measurement. Two things fall out of even that rough
model, and both matter before "up to 50%" goes in front of a user:

1. **A batch of one is not a discount — it's a subsidy.** Forwarding a
   single request costs *more* gas than that user just submitting it
   themselves (21,000 saved base cost vs. ~13,000 extra forwarder
   overhead, net loss). The mechanism only pays for itself once enough
   requests are riding in the same batch — roughly 4+ at this estimate.
   `fee::estimate_savings` returns a signed number for exactly this
   reason; a negative result means treasury is covering the gap, not
   that the user is being overcharged.
2. **The savings percentage shrinks as the forwarded call gets more
   expensive**, because the fixed per-batch savings is a smaller slice of
   a bigger number. In this rough model the ceiling as batch size grows
   is roughly `(21,000 − 13,000) / 21,000 ≈ 38%` for cheap forwarded
   calls, and less for expensive ones — not 50%.

None of that means "up to 50%" is wrong — it means it isn't validated
yet. Getting there needs real numbers: submit a batch on RHC testnet,
read `gasUsed` off the `Executed` events and the outer receipt, replace
the estimate in `fee.rs`, and redo this math with RHC's actual base-tx
cost (which may not match mainnet Ethereum's 21,000 — that's an L1
constant this hasn't confirmed against RHC's own gas schedule). Do that
before it's a marketing number, not after.

## Flow

1. Client builds a `ForwardRequest { from, to, value, gas, nonce, data, validUntil }`.
   `nonce` comes from `VapurrForwarder.nonces(from)`; `gas` is however
   much the forwarded call itself needs (get this from `eth_estimateGas`
   against `to`/`data` directly, not against the forwarder).
2. Wallet signs the EIP-712 digest (`domain: VapurrForwarder v1`,
   `verifyingContract` = the forwarder address, `chainId` = RHC's).
3. `POST /relay/submit` with `{ ...request fields, sig }` → `202 { id }`.
   This only enqueues it; nothing is on-chain yet.
4. `GET /relay/status/:id` → `pending` → `submitted { txHash }` →
   (once you look the receipt up yourself) confirmed.
5. `GET /relay/quote?callGas=50000&batchSize=8` → real computed numbers
   for "here's roughly what sponsored costs vs. self-paying," for a UI
   to show honestly instead of asserting a fixed percentage.

## Environment

| Var | Meaning |
|---|---|
| `VAPURR_RELAY_PRIVATE_KEY` | The relayer's own hot key (`0x`-hex, 32 bytes). Must be added to `authorizedRelayers` on-chain separately — this process never does that itself. |
| `VAPURR_RELAY_FORWARDER_ADDRESS` | Deployed `VapurrForwarder` address. |
| `VAPURR_RELAY_RPC_URL` | RHC RPC endpoint. |
| `VAPURR_RELAY_CHAIN_ID` | Must match the forwarder's actual `block.chainid` at deploy time. |
| `VAPURR_RELAY_BIND_ADDR` | Default `127.0.0.1:8792`. |
| `VAPURR_RELAY_BATCH_MAX_SIZE` | Default 32. |
| `VAPURR_RELAY_BATCH_MAX_WAIT_MS` | Default 2000 — how long a lone request waits hoping for company before it ships anyway. |
| `VAPURR_RELAY_USER_FEE_BPS` | Default 9500 (95% of solo cost) — deliberately *not* 5000. At this file's own overhead estimate, a 50% discount is priced above the real savings pool (~38% at best) and loses money on every batch; 9500 clears break-even across a wide range of call sizes at that estimate. Still a guess pending a real RHC measurement — see the math below before moving this number either direction. |
| `VAPURR_RELAY_PRIORITY_FEE_WEI` | Default 1 gwei. |
| `VAPURR_RELAY_LOCAL=1` | Defaults RPC/chain to RHC testnet for local runs. |

## What still needs a working Rust toolchain to confirm

This was written and unit-tested (ABI encoding and EIP-712 signing
against golden vectors generated independently with ethers.js — see the
tests in `abi.rs`/`eip712.rs`) in an environment where `cargo build`
itself doesn't work (broken MSVC/mingw linker, unrelated to this
change). Run `cargo test -p vapurr-relay` on a real toolchain, then a
real submission against RHC testnet, before this touches a mainnet key.
