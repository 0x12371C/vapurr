//! The honest math behind "up to N% less gas, treasury keeps the rest."
//!
//! Batching amortizes ONE fixed per-transaction base cost across many
//! requests instead of paying it once per request — that's the entire
//! mechanism, no external compute-offload service involved.
//!
//! **Every constant below is a REAL measurement**, not a formula guess —
//! deployed to RHC testnet (chain 46630) and run for real on 2026-09-06;
//! see docs/RELAY.md's testnet log for the deployed address, tx hashes,
//! and the scripts that produced these numbers. The formula-estimate
//! version of this module (EVM opcode arithmetic, ~12,200 gas/item) is
//! gone — it was closer than nothing, but real numbers beat it outright,
//! and one of them overturned an assumption this whole module had been
//! carrying: **RHC's base transaction cost is 25,732 gas, not Ethereum
//! mainnet's 21,000.** Everything downstream that assumed 21,000 was
//! wrong by construction, independent of anything about this contract.
//!
//! **The other real finding, bigger than any contract optimization
//! could have been**: a signer's FIRST use of this forwarder costs
//! dramatically more than their second. `nonces[from]` going from 0 to 1
//! is a full zero-to-nonzero SSTORE (~20,000 gas); going from 1 to 2 (or
//! any nonzero to nonzero) is ~2,900. That ~17,400-gas gap showed up
//! directly in the testnet run: a fresh signer's forwarded item costs
//! ~38,000 marginal gas; the same signer's SECOND forwarded item costs
//! ~20,600. **A first-time user's forwarded transaction costs MORE gas
//! than that user just submitting it themselves would have** — batching
//! a first-timer is never a discount, at any batch size, on this chain.
//! Batching only pays for itself with REPEAT signers, and even then the
//! break-even fee is around 80% of solo cost for a cheap forwarded call,
//! not 50% — see the worked numbers below.
//!
//! **One more real lever needs no code change**: if a batch's requests
//! concentrate on a handful of target contracts (plausible for vapurr —
//! most traffic hits PusdMarket, a payment router, a handful of others),
//! EIP-2929 warms an address on first access and every later access in
//! the same tx costs ~100 gas instead of ~2,600, automatically, regardless
//! of item order. `overhead_with_target_reuse` models this against the
//! conservative (first-time-user) default.
//!
//! **Where the wall actually is, restated with real numbers**: ecrecover
//! + the nonce SLOAD/SSTORE pair is signature verification and replay
//! protection — the two things this contract cannot skip without
//! becoming insecure. For a repeat user that's already ~5,900 gas before
//! anything else; for a first-timer it's ~23,000. Crossing further than
//! optimization or real measurement can needs a different cryptographic
//! scheme (signature aggregation — verify one aggregate signature for
//! the whole batch instead of N separate ecrecovers, meaning users sign
//! with something other than the secp256k1 keys their wallets already
//! have) or a flat fee that isn't trying to be a percentage rebate at all.

/// Real, measured: a plain, empty-calldata, zero-value transfer to a
/// never-touched address, on RHC testnet. NOT Ethereum mainnet's 21,000 —
/// this is what self-submitting actually costs on this chain.
pub const EVM_BASE_TX_GAS: u64 = 25_732;

/// Real, measured: the batch-size-INDEPENDENT part of one `executeBatch`
/// call — the transaction's own intrinsic cost, the ABI head's fixed
/// offset words, loop/dispatch setup. Distinct from `EVM_BASE_TX_GAS` on
/// purpose: a batch call costs more up front than a plain transfer does,
/// before any item's own work is counted at all.
pub const BATCH_FIXED_GAS: u64 = 43_143;

/// Real, measured: marginal gas for one more item whose SIGNER has never
/// used this forwarder before. This is the conservative default —
/// correct to use for a GAS LIMIT you're setting before you know who's
/// actually in the batch, since under-provisioning here can revert the
/// whole batch. For pricing a batch you already know is all repeat
/// signers, use `MARGINAL_GAS_REPEAT_USER` instead — it is NOT
/// interchangeable with this one; the ~17,400-gas gap between them is
/// the module's central finding.
pub const FORWARDER_PER_ITEM_OVERHEAD_GAS: u64 = 38_035;

/// Real, measured: marginal gas for one more item from a signer who has
/// used this forwarder before (their nonce slot is already nonzero).
/// Never use this for a gas limit on a batch whose composition you don't
/// actually know — only for pricing/reporting once you do.
pub const MARGINAL_GAS_REPEAT_USER: u64 = 20_582;

/// The theoretical floor for a REPEAT signer (ecrecover + a warm-nonzero
/// nonce SLOAD/SSTORE) — close to, but not identical to, what got
/// measured (`MARGINAL_GAS_REPEAT_USER` also includes calldata and the
/// external CALL). Kept as a sanity bound, not a substitute for the real
/// number above.
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
/// Uses the conservative first-time-signer overhead; call
/// `estimate_savings_with_overhead` directly with `MARGINAL_GAS_REPEAT_USER`
/// to model a batch of known-repeat signers instead.
pub fn estimate_savings(call_gas: &[u64]) -> SavingsEstimate {
    estimate_savings_with_overhead(call_gas, FORWARDER_PER_ITEM_OVERHEAD_GAS)
}

/// Same as `estimate_savings`, with the per-item marginal overhead
/// supplied explicitly — `FORWARDER_PER_ITEM_OVERHEAD_GAS` (first-time
/// signers) or `MARGINAL_GAS_REPEAT_USER` (repeat signers) are the two
/// real, measured choices; anything else is back to guessing.
pub fn estimate_savings_with_overhead(call_gas: &[u64], per_item_overhead: u64) -> SavingsEstimate {
    let n = call_gas.len();
    let solo_total_gas: u64 = call_gas.iter().map(|g| EVM_BASE_TX_GAS.saturating_add(*g)).sum();
    let batch_total_gas: u64 = BATCH_FIXED_GAS
        + call_gas
            .iter()
            .map(|g| g.saturating_add(per_item_overhead))
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
    fn quote_solo_cost_uses_measured_rhc_base_not_textbook_21000() {
        // /relay/quote exampleFirstRequest.soloCostGas must use this constant
        // (see api.rs). Textbook 21_000 would understate self-pay on RHC and
        // desync the gasless.html FALLBACK.
        assert_eq!(EVM_BASE_TX_GAS, 25_732);
        let call_gas = 60_000u64;
        let solo = EVM_BASE_TX_GAS.saturating_add(call_gas);
        assert_eq!(solo, 85_732);
        let fee_95 = user_fee_gas(solo, 9_500);
        assert_eq!(fee_95, (solo as u128 * 9_500 / 10_000) as u64);
    }

    #[test]
    fn lone_request_is_a_subsidy_not_a_saving() {
        let est = estimate_savings(&[50_000]);
        assert!(est.saved_gas < 0, "a batch of one should cost MORE than self-submitting, not less");
    }

    /// The central real-world finding: a first-time signer is not just
    /// "a batch of one is a subsidy" — batching them is unprofitable at
    /// realistic batch sizes too, because their nonce write alone
    /// (~38,035 gas marginal) costs more than the ~25,732 gas they'd
    /// have saved by not paying their own base tx. No batch size fixes a
    /// per-item loss.
    #[test]
    fn first_time_signers_never_pay_for_themselves_even_at_scale() {
        let est = estimate_savings(&[50_000; 24]); // default overhead = first-time signer
        assert!(
            est.saved_gas < 0,
            "batching first-time signers should stay a net loss regardless of batch size — \
             their marginal overhead alone already exceeds what one base tx saves"
        );
    }

    /// The other half of the same finding: REPEAT signers are a
    /// genuinely different, much better case — batching them turns
    /// profitable at a realistic batch size (real break-even is ~9 items
    /// for a 50,000-gas call; 16 is comfortably past it).
    #[test]
    fn repeat_signers_turn_profitable_at_realistic_batch_sizes() {
        let est = estimate_savings_with_overhead(&[50_000; 16], MARGINAL_GAS_REPEAT_USER);
        assert!(est.saved_gas > 0, "a 16-item batch of REPEAT signers should beat 16 solo submissions");
    }

    /// Regression guard on the module's central number: if this ever
    /// creeps back down near the old formula's 21,000/12,200 split,
    /// something reintroduced the assumption a real testnet run disproved.
    #[test]
    fn fresh_vs_repeat_gap_matches_the_testnet_measurement() {
        assert_eq!(FORWARDER_PER_ITEM_OVERHEAD_GAS - MARGINAL_GAS_REPEAT_USER, 17_453);
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
        let solo = 71_000u64; // an arbitrary example solo cost — not derived from the module's own constants
        let fee = user_fee_gas(solo, 5_000);
        assert!(fee <= solo / 2 + 1);
    }
}
