use vapurr_rhc::Rpc;
use vapurr_wallet::tx::{sign_tx, Tx};

use crate::abi::{encode_execute_batch, BatchItem};
use crate::config::Config;
use crate::eip712::{self, Domain};
use crate::error::RelayError;
use crate::fee::{self, FORWARDER_PER_ITEM_OVERHEAD_GAS};
use crate::queue::PendingRequest;
use crate::simulate;

pub struct SubmitOutcome {
    pub tx_hash: String,
    pub accepted_ids: Vec<String>,
    pub rejected: Vec<(String, String)>,
    /// The real gas this batch actually took, from simulating the exact
    /// calldata right before submission — `None` if that simulation
    /// failed and the formula estimate had to cover for it. Real
    /// economics reporting should prefer this over `fee::estimate_savings`
    /// whenever it's present; see `fee::measured_savings`.
    pub measured_batch_gas: Option<u64>,
}

/// Re-verify every request (never trust the queue alone — a request could
/// in principle sit there a while before it's this batch's turn, and its
/// nonce may already have moved), build the batch calldata, sign it with
/// the relayer's own key, and submit it. One bad or stale request is
/// dropped from THIS batch rather than failing everyone riding with it —
/// the contract's own per-item error handling backs that up on-chain too.
///
/// Takes `&[PendingRequest]` rather than owning it: on an `Err` return
/// (an RPC/signing failure, not a per-item rejection — those come back in
/// `SubmitOutcome::rejected`), the caller still owns the batch and can
/// requeue it instead of it being silently dropped.
pub async fn submit_batch(cfg: &Config, batch: &[PendingRequest]) -> Result<SubmitOutcome, RelayError> {
    let domain = Domain { chain_id: cfg.chain_id, verifying_contract: cfg.forwarder };

    let mut items = Vec::new();
    let mut solo_gas = Vec::new();
    let mut accepted_ids = Vec::new();
    let mut rejected = Vec::new();

    for p in batch {
        // Verify the ORIGINAL 65-byte signature the wallet actually
        // produced first — compaction below only changes how an already-
        // verified signature is encoded on-chain, never what gets checked.
        match eip712::recover_and_verify(&domain, &p.req, &p.sig).and_then(|()| eip712::to_compact(&p.sig)) {
            Ok(compact_sig) => {
                accepted_ids.push(p.id.clone());
                // Prefer the real simulated solo cost from submit time
                // over the client-declared `req.gas` — the client's
                // number is what it asked the forwarder to allot for
                // execution, not necessarily an honest self-pay estimate.
                solo_gas.push(p.solo_gas.unwrap_or(p.req.gas));
                items.push(BatchItem {
                    from: p.req.from,
                    to: p.req.to,
                    value: p.req.value,
                    gas: p.req.gas,
                    nonce: p.req.nonce,
                    data: p.req.data.clone(),
                    valid_until: p.req.valid_until,
                    sig: compact_sig.to_vec(),
                });
            }
            Err(e) => rejected.push((p.id.clone(), e.to_string())),
        }
    }

    if items.is_empty() {
        return Err(RelayError::Queue(
            "nothing left to submit — every request in this batch failed re-verification".into(),
        ));
    }

    let calldata = encode_execute_batch(&items);

    // Simulate the REAL cost of this exact calldata against live chain
    // state before trusting a formula for it. Falls back to the
    // formula's more conservative 20%-margin estimate if the node can't
    // answer right now — never blocks submission on it.
    let simulated_gas = simulate::batch_call_gas(&cfg.rpc_url, cfg.relayer_address, cfg.forwarder, &calldata)
        .await
        .ok();
    let gas_limit = match simulated_gas {
        // A real simulated number is far more trustworthy than the
        // formula's guess, so it only needs a small margin for gas-price
        // and state drift between simulation and inclusion, not the
        // formula's full 20% margin against its own uncertainty.
        Some(g) => g + g / 10,
        None => batch_gas_limit(&items),
    };

    if let Some(g) = simulated_gas {
        let report = fee::measured_savings(&solo_gas, g);
        tracing::info!(
            batch_size = report.batch_size,
            solo_total_gas = report.solo_total_gas,
            batch_total_gas = report.batch_total_gas,
            saved_gas = report.saved_gas,
            saved_bps = report.saved_bps,
            "measured batch economics (real eth_estimateGas, not the fee.rs formula)"
        );
    }

    let rpc_url = cfg.rpc_url.clone();
    let relayer_addr_hex = cfg.relayer_address.to_hex();

    let nonce = blocking(&rpc_url, move |rpc| rpc.eth_nonce(&relayer_addr_hex)).await?;
    let gas_price = blocking(&rpc_url, |rpc| rpc.eth_gas_price()).await?;
    let max_fee = gas_price.saturating_mul(2).saturating_add(cfg.priority_fee_wei);

    let tx = Tx {
        chain_id: cfg.chain_id,
        nonce,
        max_priority_fee: cfg.priority_fee_wei,
        max_fee,
        gas: gas_limit,
        to: Some(cfg.forwarder),
        value: 0,
        data: calldata,
    };

    let raw = sign_tx(&cfg.relayer_key, &tx).map_err(|e| RelayError::Rpc(format!("sign: {e}")))?;
    let raw_hex = format!("0x{}", hex::encode(raw));

    let tx_hash = blocking(&rpc_url, move |rpc| rpc.eth_send_raw(&raw_hex)).await?;

    Ok(SubmitOutcome { tx_hash, accepted_ids, rejected, measured_batch_gas: simulated_gas })
}

/// Sum of each item's own call gas plus the forwarder's per-item overhead
/// (the conservative first-time-signer number — correct here, since gas
/// LIMITS must not assume every signer in an upcoming batch is a cheaper
/// repeat one), plus the batch's real measured fixed cost, plus 20%
/// headroom — an out-of-gas partway through `executeBatch`'s loop unwinds
/// the WHOLE batch (it is not one of the contract's own per-item failure
/// paths), so under-estimating here is a real "everyone's request gets
/// dropped" failure mode, not just a wasted-gas one.
fn batch_gas_limit(items: &[BatchItem]) -> u64 {
    let base: u64 = fee::BATCH_FIXED_GAS + items.iter().map(|i| i.gas + FORWARDER_PER_ITEM_OVERHEAD_GAS).sum::<u64>();
    base + base / 5
}

async fn blocking<T, F>(rpc_url: &str, f: F) -> Result<T, RelayError>
where
    T: Send + 'static,
    F: FnOnce(&Rpc) -> Result<T, vapurr_rhc::rpc::RpcError> + Send + 'static,
{
    let rpc_url = rpc_url.to_string();
    tokio::task::spawn_blocking(move || {
        let rpc = Rpc::at(rpc_url);
        f(&rpc)
    })
    .await
    .map_err(|e| RelayError::Rpc(format!("join: {e}")))?
    .map_err(|e| RelayError::Rpc(e.to_string()))
}
