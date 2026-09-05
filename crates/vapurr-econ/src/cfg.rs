//! Live market address book. `market.json` in the device data dir.

use serde::{Deserialize, Serialize};
use vapurr_rhc as rhc;

pub(crate) const GEN: u32 = 5;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct MarketCfg {
    #[serde(default)]
    pub(crate) gen: u32,
    #[serde(default)]
    pub(crate) market: String,
    #[serde(default)]
    pub(crate) vapurr: String,
    #[serde(default)]
    pub(crate) pusd: String,
    #[serde(default)]
    pub(crate) outbid: String,
    #[serde(default)]
    pub(crate) ketlist: String,
    #[serde(default)]
    pub(crate) usdg: String,
    #[serde(default, rename = "loop")]
    pub(crate) loop_vault: String,
    #[serde(default)]
    pub(crate) house: String,
    #[serde(default)]
    pub(crate) swap: String,
    /// HousePairConfig address (wgV + PUSD SoT). Required for HouseLp/HouseSwap deploy.
    #[serde(default)]
    pub(crate) pair_config: String,
    /// Previous Lithe book retained for cutover provenance and the PUSD migration route.
    #[serde(default)]
    pub(crate) legacy_market: String,
    #[serde(default)]
    pub(crate) legacy_vapurr: String,
    #[serde(default)]
    pub(crate) legacy_pusd: String,
    /// Successor one-token deployment record.
    #[serde(default)]
    pub(crate) cutover_factory: String,
    #[serde(default)]
    pub(crate) v_converter: String,
    #[serde(default)]
    pub(crate) pusd_migrator: String,
    #[serde(default)]
    pub(crate) rebase_policy: String,
    #[serde(default)]
    pub(crate) gv: String,
    #[serde(default)]
    pub(crate) net: String,
}

fn dead_swap(s: &str) -> bool {
    let l = s.trim().to_ascii_lowercase();
    l.starts_with("0xb699") || l.starts_with("0xb10d") || l.starts_with("0xbd6b")
}

impl MarketCfg {
    fn path() -> std::path::PathBuf {
        vapurr_wallet::data_dir().join("market.json")
    }

    fn fill_canonical(&mut self) {
        if self.net.eq_ignore_ascii_case("mainnet") {
            return;
        }
        if self.net.is_empty() {
            self.net = "testnet".into();
        }
        if self.market.is_empty() && !rhc::TESTNET_MARKET.is_empty() {
            self.market = rhc::TESTNET_MARKET.into();
        }
        if self.vapurr.is_empty() && !rhc::TESTNET_VAPURR.is_empty() {
            self.vapurr = rhc::TESTNET_VAPURR.into();
        }
        if self.pusd.is_empty() && !rhc::TESTNET_PUSD.is_empty() {
            self.pusd = rhc::TESTNET_PUSD.into();
        }
        if self.outbid.is_empty() && !rhc::TESTNET_OUTBID.is_empty() {
            self.outbid = rhc::TESTNET_OUTBID.into();
        }
        if self.ketlist.is_empty() && !rhc::TESTNET_KETLIST.is_empty() {
            self.ketlist = rhc::TESTNET_KETLIST.into();
        }
        if self.usdg.is_empty() && !rhc::TESTNET_MOCK_USDG.is_empty() {
            self.usdg = rhc::TESTNET_MOCK_USDG.into();
        }
        if self.loop_vault.is_empty() && !rhc::TESTNET_LOOP.is_empty() {
            self.loop_vault = rhc::TESTNET_LOOP.into();
        }
        if self.house.is_empty() && !rhc::TESTNET_HOUSE.is_empty() {
            self.house = rhc::TESTNET_HOUSE.into();
        }
        if self.swap.is_empty() || dead_swap(&self.swap) {
            if !rhc::TESTNET_SWAP.is_empty() {
                self.swap = rhc::TESTNET_SWAP.into();
            }
        }
    }

    pub(crate) fn load() -> Self {
        let mut c = if let Ok(bytes) = std::fs::read(Self::path()) {
            if let Ok(c) = serde_json::from_slice::<MarketCfg>(&bytes) {
                if c.gen >= GEN {
                    c
                } else {
                    // prior generation — drop the retired book. Fresh deploy.
                    Self {
                        gen: 0,
                        net: if c.net.is_empty() { "testnet".into() } else { c.net },
                        ..Self::default()
                    }
                }
            } else {
                Self {
                    net: "testnet".into(),
                    ..Self::default()
                }
            }
        } else {
            Self {
                net: "testnet".into(),
                ..Self::default()
            }
        };
        c.fill_canonical();
        c
    }

    pub(crate) fn save(&self) {
        if let Ok(bytes) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::create_dir_all(vapurr_wallet::data_dir());
            let _ = std::fs::write(Self::path(), bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_testnet_adopts_gen4_book() {
        let mut c = MarketCfg {
            net: "testnet".into(),
            ..MarketCfg::default()
        };
        c.fill_canonical();
        assert_eq!(c.market, rhc::TESTNET_MARKET);
        assert_eq!(c.pusd, rhc::TESTNET_PUSD);
        assert_eq!(c.vapurr, rhc::TESTNET_VAPURR);
        assert_eq!(c.house, rhc::TESTNET_HOUSE);
        assert_eq!(c.loop_vault, rhc::TESTNET_LOOP);
        assert!(c.outbid.is_empty());
        assert!(c.ketlist.is_empty());
        assert!(c.usdg.is_empty());
        assert_eq!(rhc::TESTNET_MARKET.len(), 42);
        assert_eq!(rhc::TESTNET_HOUSE.len(), 42);
        assert_eq!(rhc::TESTNET_LOOP.len(), 42);
        assert_eq!(rhc::TESTNET_SWAP.len(), 42);
        assert_eq!(c.swap, rhc::TESTNET_SWAP);
    }

    #[test]
    fn dead_swapper_is_replaced() {
        let mut c = MarketCfg {
            net: "testnet".into(),
            swap: "0xb699c0CDA2C41f28A458e8Fd59Fa7e68d06e4FE2".into(),
            ..MarketCfg::default()
        };
        c.fill_canonical();
        assert_eq!(c.swap, rhc::TESTNET_SWAP);
    }

    #[test]
    fn mainnet_does_not_adopt_testnet_book() {
        let mut c = MarketCfg {
            net: "mainnet".into(),
            ..MarketCfg::default()
        };
        c.fill_canonical();
        assert!(c.market.is_empty());
        assert!(c.pusd.is_empty());
        assert!(c.loop_vault.is_empty());
        assert!(c.house.is_empty());
    }
}
