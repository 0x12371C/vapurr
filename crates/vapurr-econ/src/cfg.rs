//! Live market address book. `market.json` in the device data dir.

use serde::{Deserialize, Serialize};

pub(crate) const GEN: u32 = 3;

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
    pub(crate) usdg: String,
    #[serde(default)]
    pub(crate) net: String,
}

impl MarketCfg {
    fn path() -> std::path::PathBuf {
        vapurr_wallet::data_dir().join("market.json")
    }

    pub(crate) fn load() -> Self {
        if let Ok(bytes) = std::fs::read(Self::path()) {
            if let Ok(c) = serde_json::from_slice::<MarketCfg>(&bytes) {
                if c.gen >= GEN {
                    return c;
                }
                // prior generation — keep the mintable test USDG, drop the old market
                return Self {
                    gen: 0,
                    usdg: c.usdg,
                    net: if c.net.is_empty() { "testnet".into() } else { c.net },
                    ..Self::default()
                };
            }
        }
        Self {
            net: "testnet".into(),
            ..Self::default()
        }
    }

    pub(crate) fn save(&self) {
        if let Ok(bytes) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::create_dir_all(vapurr_wallet::data_dir());
            let _ = std::fs::write(Self::path(), bytes);
        }
    }
}
