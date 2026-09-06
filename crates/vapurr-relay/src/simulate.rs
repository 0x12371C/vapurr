//! Real numbers instead of fee.rs's fixed-constant guess.
//!
//! vapurr has a live RHC node and its own scan/explorer infra right here
//! — there's no reason to keep assuming `FORWARDER_PER_ITEM_OVERHEAD_GAS`
//! once a real `eth_estimateGas` call can answer the same question against
//! actual chain state. fee.rs's formula stays as the fallback for when a
//! simulation call fails or hasn't run yet (pre-deployment planning, a
//! quote for a request that doesn't exist), never as a substitute for a
//! real number once one's available.
//!
//! Neither function here is load-bearing for correctness — a failed
//! simulation degrades to the formula estimate (see `submit.rs` and
//! `api.rs`), it never blocks a submission. A gas estimate is advice from
//! the node about the state it saw a moment ago, not a promise; treat a
//! successful call here as "the best number available right now," not as
//! ground truth immune from failing to cover a real execution.

use vapurr_rhc::Rpc;
use vapurr_wallet::Address;

use crate::error::RelayError;

async fn estimate(rpc_url: &str, from: Address, to: Address, data: Vec<u8>) -> Result<u64, RelayError> {
    let rpc_url = rpc_url.to_string();
    let from_hex = from.to_hex();
    let to_hex = to.to_hex();
    let data_hex = format!("0x{}", hex::encode(&data));
    tokio::task::spawn_blocking(move || {
        let rpc = Rpc::at(rpc_url);
        rpc.eth_estimate_gas(&from_hex, Some(&to_hex), &data_hex)
    })
    .await
    .map_err(|e| RelayError::Rpc(format!("join: {e}")))?
    .map_err(|e| RelayError::Rpc(e.to_string()))
}

/// What one forwarded call would cost submitted on its own, right now —
/// the real "solo cost" fee.rs otherwise has to assume from a flat
/// `avgCallGas` guess. Simulated as `from` calling `to` directly with
/// `data` (NOT through the forwarder) — that's what "self-submitting"
/// actually means.
pub async fn solo_call_gas(rpc_url: &str, from: Address, to: Address, data: &[u8]) -> Result<u64, RelayError> {
    estimate(rpc_url, from, to, data.to_vec()).await
}

/// What THIS exact `executeBatch` call costs, simulated against live
/// chain state immediately before it's actually submitted — replaces
/// `submit::batch_gas_limit`'s formula guess whenever the node answers.
pub async fn batch_call_gas(
    rpc_url: &str,
    relayer: Address,
    forwarder: Address,
    calldata: &[u8],
) -> Result<u64, RelayError> {
    estimate(rpc_url, relayer, forwarder, calldata.to_vec()).await
}
