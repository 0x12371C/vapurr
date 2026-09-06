use vapurr_rhc::Rpc;
use vapurr_wallet::tx::{sign_tx, Tx};

use crate::abi::{encode_execute_batch, BatchItem};
use crate::config::Config;
use crate::eip712::{self, Domain};
use crate::error::RelayError;
use crate::fee::FORWARDER_PER_ITEM_OVERHEAD_GAS;
use crate::queue::PendingRequest;

pub struct SubmitOutcome {
    pub tx_hash: String,
    pub accepted_ids: Vec<String>,
    pub rejected: Vec<(String, String)>,
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
    let mut accepted_ids = Vec::new();
    let mut rejected = Vec::new();

    for p in batch {
        match eip712::recover_and_verify(&domain, &p.req, &p.sig) {
            Ok(()) => {
                accepted_ids.push(p.id.clone());
                items.push(BatchItem {
                    from: p.req.from,
                    to: p.req.to,
                    value: p.req.value,
                    gas: p.req.gas,
                    nonce: p.req.nonce,
                    data: p.req.data.clone(),
                    valid_until: p.req.valid_until,
                    sig: p.sig.clone(),
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
    let gas_limit = batch_gas_limit(&items);

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

    Ok(SubmitOutcome { tx_hash, accepted_ids, rejected })
}

/// Sum of each item's own call gas plus the forwarder's per-item overhead,
/// plus the outer transaction's base cost, plus 20% headroom — an
/// out-of-gas partway through `executeBatch`'s loop unwinds the WHOLE
/// batch (it is not one of the contract's own per-item failure paths), so
/// under-estimating here is a real "everyone's request gets dropped"
/// failure mode, not just a wasted-gas one.
fn batch_gas_limit(items: &[BatchItem]) -> u64 {
    let base: u64 = 21_000 + items.iter().map(|i| i.gas + FORWARDER_PER_ITEM_OVERHEAD_GAS).sum::<u64>();
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
