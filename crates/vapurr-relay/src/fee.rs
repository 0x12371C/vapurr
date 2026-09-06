//! The honest math behind "up to N% less gas, treasury keeps the rest."
//!
//! Batching amortizes ONE fixed per-transaction base cost across many
//! requests instead of paying it once per request — that's the entire
//! mechanism, no external compute-offload service involved. But it isn't
//! free: `VapurrForwarder._runFields` does real per-item work (ecrecover,
//! a nonce SLOAD+SSTORE, the external CALL, an event) that a plain
//! self-submitted tx wouldn't pay. `FORWARDER_PER_ITEM_OVERHEAD_GAS` below
//! is a rough EVM-opcode-cost estimate (~13k gas: 3000 ecrecover + ~2100
//! cold SLOAD + ~2900 warm-nonzero SSTORE + ~2600 cold CALL + ~1900 for a
//! 2-indexed-topic LOG + slack) — NOT a measurement. Until it's replaced
//! with real numbers from RHC testnet (submit a batch, read `gasUsed` off
//! `Executed` events and the tx receipt), do not treat any percentage this
//! module reports as a marketing number.
//!
//! Worked example at this estimate: with `EVM_BASE_TX_GAS = 21_000` and
//! `FORWARDER_PER_ITEM_OVERHEAD_GAS ≈ 13_000`, batching only turns a net
//! gas win somewhere around a 3-4 request batch (below that, the
//! forwarder's own overhead costs more than the shared base fee it's
//! saving — sponsoring a lone request is a real treasury cost, not a
//! discount), and the savings fraction *shrinks* as the forwarded call
//! itself gets more expensive, because the fixed base-cost saving is a
//! smaller slice of a bigger number. In this rough model the ceiling as
//! batch size grows is roughly `(EVM_BASE_TX_GAS - FORWARDER_PER_ITEM_OVERHEAD_GAS)
//! / EVM_BASE_TX_GAS` for cheap forwarded calls — call it ~35% here, not 50%.
//! Getting to "up to 50%" needs either a lower real per-item overhead on
//! RHC than this estimate, a lower RHC base-tx cost than mainnet's 21,000,
//! or a second mechanism this pass deliberately doesn't build (e.g. an
//! actual compute-offload service for what the forwarded call itself does).

pub const EVM_BASE_TX_GAS: u64 = 21_000;
/// TODO(calibrate): replace with a real measurement from RHC testnet.
pub const FORWARDER_PER_ITEM_OVERHEAD_GAS: u64 = 13_000;

#[derive(Clone, Debug, serde::Serialize)]
pub struct SavingsEstimate {
    pub batch_size: usize,
    pub solo_total_gas: u64,
    pub batch_total_gas: u64,
    /// Positive = batching wins. Negative = the relayer is subsidizing
    /// this batch out of treasury margin, not "saving" anything.
    pub saved_gas: i64,
    /// Basis points of `solo_total_gas` saved; 10_000 = 100%. Negative if
    /// `saved_gas` is negative.
    pub saved_bps: i64,
}

/// `call_gas` is each forwarded call's OWN execution gas (the `gas` field
/// of its ForwardRequest, or a fresher `eth_estimateGas` if you have one) —
/// not including any transaction-level overhead, which this function adds.
pub fn estimate_savings(call_gas: &[u64]) -> SavingsEstimate {
    let n = call_gas.len();
    let solo_total_gas: u64 = call_gas.iter().map(|g| EVM_BASE_TX_GAS.saturating_add(*g)).sum();
    let batch_total_gas: u64 = EVM_BASE_TX_GAS
        + call_gas
            .iter()
            .map(|g| g.saturating_add(FORWARDER_PER_ITEM_OVERHEAD_GAS))
            .sum::<u64>();
    let saved_gas = solo_total_gas as i64 - batch_total_gas as i64;
    let saved_bps = if solo_total_gas == 0 { 0 } else { saved_gas * 10_000 / solo_total_gas as i64 };
    SavingsEstimate { batch_size: n, solo_total_gas, batch_total_gas, saved_gas, saved_bps }
}

/// What to charge a user for sponsorship, in the same gas units as
/// `call_gas` (convert to a token amount at whatever gas price / oracle
/// rate is current — not this module's job). `fee_bps` is the fraction of
/// their OWN solo cost they pay; 5_000 = "you pay half of what self-paying
/// would have cost." The gap between what they pay and their pro-rata
/// share of `batch_total_gas` is the treasury's margin — real only when
/// `estimate_savings` for this batch actually came back positive.
pub fn user_fee_gas(solo_cost_gas: u64, fee_bps: u64) -> u64 {
    ((solo_cost_gas as u128 * fee_bps as u128) / 10_000) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lone_request_is_a_subsidy_not_a_saving() {
        let est = estimate_savings(&[50_000]);
        assert!(est.saved_gas < 0, "a batch of one should cost MORE than self-submitting, not less");
    }

    #[test]
    fn savings_turn_positive_at_realistic_batch_sizes() {
        let est = estimate_savings(&[50_000; 8]);
        assert!(est.saved_gas > 0, "an 8-request batch should beat 8 solo submissions");
    }

    #[test]
    fn fifty_percent_fee_is_never_worse_than_self_pay() {
        let solo = 71_000u64; // 21_000 + 50_000
        let fee = user_fee_gas(solo, 5_000);
        assert!(fee <= solo / 2 + 1);
    }
}
