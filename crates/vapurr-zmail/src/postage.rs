//! Postage is a signed voucher in $PUSD or $VAPURR. Sender pays 0 ETH.
//!
//! Body never goes on-chain. If a relayer later posts a pointer, the all-in
//! cost (postage + gas) must stay under a cent.

use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

/// $0.0025. Under a cent by a wide margin.
pub const POSTAGE_USD_MICROS: u64 = 2_500;
/// Hard cap including any on-chain fallback.
pub const MAX_USD_MICROS: u64 = 10_000;
/// $PUSD / $VAPURR are 18 decimals. 0.0025 * 1e18.
pub const POSTAGE_TOKEN: u128 = 2_500_000_000_000_000;
pub const TOKEN_DECIMALS: u8 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Asset {
    Pusd,
    Vapurr,
}

impl Asset {
    pub fn parse(s: &str) -> Self {
        match s.trim() {
            s if s.eq_ignore_ascii_case("vapurr") || s.eq_ignore_ascii_case("$vapurr") => {
                Asset::Vapurr
            }
            _ => Asset::Pusd,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Asset::Pusd => "PUSD",
            Asset::Vapurr => "VAPURR",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub asset: Asset,
    pub amount: String,
    pub usd_micros: u64,
    pub gas_wei: u128,
    pub gasless: bool,
    pub label: String,
    pub note: String,
}

impl Quote {
    pub fn gasless(asset: Asset) -> Self {
        Self {
            asset,
            amount: POSTAGE_TOKEN.to_string(),
            usd_micros: POSTAGE_USD_MICROS,
            gas_wei: 0,
            gasless: true,
            label: format!("0.25¢ ${}", asset.symbol()),
            note: "voucher · 0 ETH · body is an IPFS CID".into(),
        }
    }
}

/// `eth_usd_micros` is $ETH * 1e6 (e.g. $3000 → 3_000_000_000).
pub fn total_usd_micros(postage: u64, gas_wei: u128, eth_usd_micros: u64) -> u128 {
    let gas = if gas_wei == 0 || eth_usd_micros == 0 {
        0
    } else {
        gas_wei.saturating_mul(eth_usd_micros as u128) / 1_000_000_000_000_000_000
    };
    (postage as u128).saturating_add(gas)
}

pub fn quote(asset: Asset, gas_wei: u128, eth_usd_micros: u64) -> Result<Quote, PostageError> {
    let total = total_usd_micros(POSTAGE_USD_MICROS, gas_wei, eth_usd_micros);
    if total > MAX_USD_MICROS as u128 {
        return Err(PostageError::OverCap);
    }
    if gas_wei == 0 {
        return Ok(Quote::gasless(asset));
    }
    let mut q = Quote::gasless(asset);
    q.gas_wei = gas_wei;
    q.gasless = false;
    q.usd_micros = total as u64;
    q.label = format!("0.25¢ ${} + gas", asset.symbol());
    q.note = "on-chain fallback stayed under a cent".into();
    Ok(q)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voucher {
    pub scheme: String,
    pub chain_id: u64,
    pub from: String,
    pub to: String,
    pub cid: String,
    pub asset: Asset,
    pub amount: String,
    pub nonce: u64,
    pub deadline: u64,
    #[serde(default)]
    pub sig: String,
}

impl Voucher {
    pub fn new(from: &str, to: &str, cid: &str, asset: Asset, nonce: u64, now: u64) -> Self {
        Self {
            scheme: "voucher".into(),
            chain_id: 4663,
            from: from.to_string(),
            to: to.to_string(),
            cid: cid.to_string(),
            asset,
            amount: POSTAGE_TOKEN.to_string(),
            nonce,
            deadline: now.saturating_add(86_400),
            sig: String::new(),
        }
    }

    pub fn digest(&self) -> [u8; 32] {
        let s = format!(
            "zzzmail-postage-v1|{}|{}|{}|{}|{}|{}|{}|{}",
            self.chain_id,
            self.from,
            self.to,
            self.cid,
            self.asset.symbol(),
            self.amount,
            self.nonce,
            self.deadline
        );
        let h = Keccak256::digest(s.as_bytes());
        let mut out = [0u8; 32];
        out.copy_from_slice(&h);
        out
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PostageError {
    #[error("postage plus gas is over a cent")]
    OverCap,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gasless_is_a_quarter_cent() {
        let q = Quote::gasless(Asset::Pusd);
        assert!(q.gasless);
        assert_eq!(q.usd_micros, 2_500);
        assert!(q.usd_micros < MAX_USD_MICROS);
        assert_eq!(q.label, "0.25¢ $PUSD");
        let v = Quote::gasless(Asset::Vapurr);
        assert_eq!(v.label, "0.25¢ $VAPURR");
    }

    #[test]
    fn onchain_over_a_cent_is_rejected() {
        // 21k gas at 1 gwei, ETH at $3000 → ~6.3¢. Over the cap.
        let gas_wei = 21_000u128 * 1_000_000_000;
        let eth = 3_000 * 1_000_000;
        assert_eq!(quote(Asset::Pusd, gas_wei, eth).unwrap_err(), PostageError::OverCap);
        assert!(quote(Asset::Pusd, 0, eth).is_ok());
    }

    #[test]
    fn cheap_l2_gas_still_under_cap() {
        // 21k gas at 0.001 gwei, ETH $3000 → << 1¢.
        let gas_wei = 21_000u128 * 1_000_000;
        let eth = 3_000 * 1_000_000;
        let q = quote(Asset::Pusd, gas_wei, eth).unwrap();
        assert!(q.usd_micros <= MAX_USD_MICROS);
        assert!(!q.gasless);
    }

    #[test]
    fn voucher_digest_is_stable() {
        let a = Voucher::new("0xabc", "@bob", "bafkreiabc", Asset::Pusd, 1, 1_000);
        let b = Voucher::new("0xabc", "@bob", "bafkreiabc", Asset::Pusd, 1, 1_000);
        assert_eq!(a.digest(), b.digest());
        let c = Voucher::new("0xabc", "@bob", "bafkreiabc", Asset::Pusd, 2, 1_000);
        assert_ne!(a.digest(), c.digest());
    }
}
