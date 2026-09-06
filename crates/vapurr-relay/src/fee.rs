//! The honest math behind "up to N% less gas, treasury keeps the rest."
//!
//! Batching amortizes ONE fixed per-transaction base cost across many
//! requests instead of paying it once per request — that's the entire
//! mechanism, no external compute-offload service involved. But it isn't
//! free: `VapurrForwarder._runFields` does real per-item work (ecrecover,
//! a nonce SLOAD+SSTORE, the external CALL, an event) that a plain
//! self-submitted tx wouldn't pay.
//!
//! `FORWARDER_PER_ITEM_OVERHEAD_GAS` below has been optimized once already
//! — EIP-2098 compact signatures (saves ~130 gas/item of calldata) and a
//! trimmed `Executed` event (one indexed topic instead of two, `gasUsed`
//! dropped — saves ~375 gas/item) — bringing the estimate from ~13,000 down
//! to ~12,200. Both are real, shipped, and still an *estimate*: ecrecover
//! (3,000, a fixed precompile cost — not optimizable) + ~2,100 cold SLOAD +
//! ~2,900 warm-nonzero SSTORE for the nonce (this pair is the actual floor:
//! shrinking it means weakening replay protection, not writing better
//! code) + ~2,600 cold CALL to the target contract + ~1,518 for the
//! trimmed LOG + slack. Until it's replaced with a real RHC testnet
//! measurement (submit a batch, read `gasUsed` off the tx receipt), do not
//! treat any percentage this module reports as a marketing number.
//!
//! **One more optimization exists that needed no code change at all**: if
//! a batch's requests concentrate on a handful of target contracts (very
//! plausible for vapurr — most traffic hits PusdMarket, a payment router,
//! a handful of others), EIP-2929 warms an address on its FIRST access
//! per transaction and every later access in the same tx costs ~100 gas
//! instead of 2,600, automatically, regardless of item order. That's not
//! something to implement — it already happens for whatever a real batch's
//! composition turns out to be. The constant below stays at the
//! conservative all-cold assumption; see `overhead_with_target_reuse` for
//! how much better a concentrated batch could realistically do.
//!
//! **Where the wall actually is.** ecrecover + the nonce SLOAD/SSTORE pair
//! — signature verification and replay protection, the two things this
//! contract cannot skip without becoming insecure — already cost ~8,000
//! gas per item, which is 38% of the 21,000 being saved by NOT paying a
//! separate base tx. That number is a floor for ANY forwarder built this
//! way (independent nonce + independent ecrecover per item), not a
//! reflection of how well this one is written. Worked example at
//! `FORWARDER_PER_ITEM_OVERHEAD_GAS ≈ 12,200`: break-even for a 60,000-gas
//! forwarded call moves from 91.2% (pre-optimization) to about 89.1% — real,
//! and nowhere near enough to make a 50%-off promise solvent. Crossing that
//! requires a different cryptographic scheme (signature aggregation —
//! verify one aggregate signature for the whole batch instead of N
//! separate ecrecovers — which means users signing with something other
//! than the secp256k1 keys their wallets already have, a materially
//! bigger project) or abandoning percentage-of-savings pricing for a flat
//! fee that isn't trying to be a rebate at all.

pub const EVM_BASE_TX_GAS: u64 = 21_000;
/// TODO(calibrate): replace with a real measurement from RHC testnet.
/// Was 13,000 before EIP-2098 compact sigs + the trimmed `Executed` event
/// (see the module doc for the full breakdown of both).
pub const FORWARDER_PER_ITEM_OVERHEAD_GAS: u64 = 12_200;
/// The floor: ecrecover + the nonce SLOAD/SSTORE pair, the two costs no
/// version of this forwarder can shed without weakening security. Every
/// other cost (the CALL, the event, calldata padding) can shrink toward
/// zero under favorable conditions; this one cannot.
pub const CRYPTO_VERIFICATION_FLOOR_GAS: u64 = 3_000 + 2_100 + 2_900;

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

/// The same shape as `estimate_savings`, built from REAL numbers instead
/// of the formula guess: `solo_gas` from `simulate::solo_call_gas` per
/// item, and `batch_total_gas_measured` from `simulate::batch_call_gas`
/// on the actual constructed calldata. Prefer this whenever both are
/// available — it isn't an estimate, it's what the batch actually costs
/// (as of the moment it was simulated; real inclusion can still vary).
pub fn measured_savings(solo_gas: &[u64], batch_total_gas_measured: u64) -> SavingsEstimate {
    let n = solo_gas.len();
    let solo_total_gas: u64 = solo_gas.iter().map(|g| EVM_BASE_TX_GAS.saturating_add(*g)).sum();
    let saved_gas = solo_total_gas as i64 - batch_total_gas_measured as i64;
    let saved_bps = if solo_total_gas == 0 { 0 } else { saved_gas * 10_000 / solo_total_gas as i64 };
    SavingsEstimate { batch_size: n, solo_total_gas, batch_total_gas: batch_total_gas_measured, saved_gas, saved_bps }
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

/// What `FORWARDER_PER_ITEM_OVERHEAD_GAS` would be if every item in a
/// batch after the first one that shares its `to` address gets EIP-2929's
/// warm-access discount on the external CALL (2,600 -> ~100 gas) — a real
/// effect that needs no relayer code to capture, only a batch composition
/// where `unique_targets` is meaningfully smaller than the batch size.
/// This is a best case for a given batch, not a new default: it's exactly
/// as good as this specific batch's mix of targets happens to be.
pub fn overhead_with_target_reuse(batch_size: usize, unique_targets: usize) -> u64 {
    if batch_size == 0 {
        return FORWARDER_PER_ITEM_OVERHEAD_GAS;
    }
    const COLD_CALL_GAS: u64 = 2_600;
    const WARM_CALL_GAS: u64 = 100;
    let unique_targets = unique_targets.max(1).min(batch_size) as u64;
    let n = batch_size as u64;
    let total_call_gas = unique_targets * COLD_CALL_GAS + (n - unique_targets) * WARM_CALL_GAS;
    let avg_call_gas = total_call_gas / n;
    // Same overhead as the default estimate, minus the assumed 2,600 cold
    // CALL it already includes, plus this batch's real average.
    (FORWARDER_PER_ITEM_OVERHEAD_GAS - COLD_CALL_GAS) + avg_call_gas
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
    fn measured_savings_matches_formula_when_inputs_agree() {
        // If a "measured" batch total happens to equal exactly what the
        // formula would have predicted for the same call gas, the two
        // functions must agree — they're describing the same quantity two
        // different ways, not two different quantities.
        let call_gas = [50_000u64; 6];
        let formula = estimate_savings(&call_gas);
        let measured = measured_savings(&call_gas, formula.batch_total_gas);
        assert_eq!(formula.solo_total_gas, measured.solo_total_gas);
        assert_eq!(formula.saved_gas, measured.saved_gas);
    }

    #[test]
    fn target_reuse_never_beats_the_crypto_floor() {
        // Every item hitting the SAME contract — the best possible case.
        let best_case = overhead_with_target_reuse(24, 1);
        assert!(
            best_case >= CRYPTO_VERIFICATION_FLOOR_GAS,
            "even maximal call-target reuse cannot drop below ecrecover + nonce bookkeeping"
        );
        // All-distinct targets should reproduce the conservative default.
        assert_eq!(overhead_with_target_reuse(24, 24), FORWARDER_PER_ITEM_OVERHEAD_GAS);
    }

    #[test]
    fn fifty_percent_fee_is_unprofitable_even_with_perfect_target_reuse() {
        // The whole point of asking "can optimization alone get us to
        // 50%" — answer it directly, at the best case this module can
        // model, not just the conservative default.
        let call_gas = 60_000u64;
        let best_overhead = overhead_with_target_reuse(24, 1);
        let solo = EVM_BASE_TX_GAS + call_gas;
        let batch_share = call_gas + best_overhead;
        let fee_at_50pct = user_fee_gas(solo, 5_000);
        assert!(
            (fee_at_50pct as i64) < batch_share as i64,
            "a 50% discount should still be unprofitable even under the most favorable batching assumptions this module can construct"
        );
    }

    #[test]
    fn fifty_percent_fee_is_never_worse_than_self_pay() {
        let solo = 71_000u64; // 21_000 + 50_000
        let fee = user_fee_gas(solo, 5_000);
        assert!(fee <= solo / 2 + 1);
    }
}
