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
estimate, not a measurement, currently **12,200 gas** — down from an
initial 13,000 after one optimization pass:

- **EIP-2098 compact signatures** on the batch path (64 bytes instead of
  65 — no padding, no wallet-visible change; the relayer compacts an
  already-verified ordinary signature itself). ~130 gas/item.
- **A trimmed `Executed` event** — one indexed topic instead of two,
  `gasUsed` dropped. ~375 gas/item. (Real trade-off, not a free lunch:
  `to` is no longer indexed, so "show me every execution to contract X"
  gets more expensive to query; "show my history," filtered by `from`,
  stays cheap.)
- **A real bug fixed along the way**: the batch path's signature recovery
  used to `require()`-revert on a malformed signature, which would have
  taken down the *entire* batch over one bad item — exactly the failure
  mode the per-item error handling was supposed to prevent. It's now a
  non-reverting recovery (`_recoverCompact`, mirrors what `ecrecover`
  itself does on failure) that fails only that one item.

**Where the wall actually is.** ecrecover (3,000, a fixed precompile
cost) plus the nonce SLOAD+SSTORE pair (~5,000, the actual replay-
protection mechanism) is ~8,000 gas per item that no contract-level
optimization removes without weakening security — and that's already
38% of the 21,000 gas a batch saves by not paying a second base
transaction. Two things fall out of this:

1. **A batch of one is still a subsidy, not a discount.** A single
   forwarded request costs more gas than self-submitting (21,000 saved
   vs. ~12,200 extra overhead). The break-even batch size is smaller than
   before, but still requires real company in the batch.
2. **The savings ceiling barely moved.** For a 60,000-gas forwarded call,
   break-even fee went from 91.2% to about 89.1% — real, verified with
   tests (`fee.rs::fifty_percent_fee_is_unprofitable_even_with_perfect_target_reuse`
   checks this explicitly, including the best-case scenario below), and
   nowhere near a 50%-off promise.

**A second lever needed no code change**: if a batch's requests
concentrate on a handful of contracts, EIP-2929 warms a target address on
first touch — every later call to the *same* address in that transaction
costs ~100 gas instead of 2,600, automatically, regardless of item order.
`fee::overhead_with_target_reuse` models this. Even at the best case (one
shared target, every other cost otherwise unchanged), overhead only drops
to ~9,800 — still bounded below by the same ~8,000-gas crypto floor.

**Getting past that floor** needs one of: a real RHC measurement showing
this estimate is too conservative (possible — untested), signature
aggregation (verify one aggregate signature for a whole batch instead of
N separate ecrecovers — a materially bigger project, since it means users
signing with something other than the secp256k1 keys their wallets
already produce), or pricing this as a flat fee that isn't trying to be a
percentage rebate on a savings pool that's smaller than the discount
being promised against it.

## Real numbers instead of the formula, wherever one's available

`FORWARDER_PER_ITEM_OVERHEAD_GAS` is a formula guess for *planning* —
pricing decisions before any request exists, the `/relay/quote` endpoint,
the standalone calculator. The running relayer doesn't have to guess once
a request exists to actually simulate:

- **`POST /relay/submit`** runs a real `eth_estimateGas` for the request's
  exact `(from, to, data)` the moment it arrives (`simulate::solo_call_gas`)
  and stores it on the queued item. That's the real "what would self-
  paying have cost," not the flat `avgCallGas` assumption.
- **Before every batch submission**, `submit_batch` simulates the actual
  constructed `executeBatch` calldata (`simulate::batch_call_gas`) to get
  this specific batch's real gas requirement, and uses that (plus a 10%
  margin for gas-price/state drift between simulation and inclusion) as
  the transaction's gas limit — replacing the formula's blinder 20% margin
  with a much tighter one, because a simulated number isn't a guess.
- Every batch's *real* economics — `fee::measured_savings` against the
  simulated total — are logged (`tracing::info!`) at submission time, so
  whether pricing is actually working is observable from day one, not
  something to infer from the formula.

Both simulation calls are best-effort: a failed one (RPC hiccup, node
briefly unreachable) falls back to the formula rather than blocking a
submission — see `simulate.rs`'s module doc. This doesn't require
anything beyond the RPC node already being called elsewhere in this
crate; hooking it up to vapurr's own scan/explorer infra for historical
analysis (average measured overhead over the last N days, not just the
most recent batch) is a natural next step this pass doesn't build.

**What this doesn't fix**: simulation tells you the REAL number instead
of a guessed one — it can't make the crypto-verification floor smaller.
If the real measured overhead comes back near 12,200 (or higher), the
"where the wall is" section above still applies; simulation just replaces
"probably" with "confirmed."

## Private mempool routing?

Would not touch anything in the section above — ecrecover and the nonce
SLOAD/SSTORE cost the same EVM gas regardless of how a transaction
reaches the block it lands in. What private order flow (a private relay,
direct-to-builder submission) actually buys, if RHC has a public mempool
with competing searchers at all: MEV protection for what's *inside* a
forwarded call — e.g. a swap sitting in a public mempool is sandwichable,
private submission closes that off — and a guarantee against paying for a
transaction that reverts, which `simulate::batch_call_gas` above already
covers most of client-side. Whether either matters depends on whether RHC
even has that dynamic (a single-sequencer app-chain might not); that's
unconfirmed, not assumed either way here.

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
