//! Live PusdMarket ABI bridge.
//!
//! Source + Solidity use scrubbed names (`swapVToPusd`, `vapurrRate`). The
//! gen-4 book on 46630 (`TESTNET_MARKET`) was deployed before that scrub and
//! still answers the pre-scrub selectors. Pin those as hex so we never put
//! banned brand strings back in the tree. After a scrubbed redeploy, eth_call
//! on `vapurrRate()` succeeds and we prefer the scrubbed sigs automatically.

use std::sync::OnceLock;

use vapurr_wallet::tx::{
    abi_u256, decode_hex_bytes, decode_word_u128, encode_fn, hex0x,
};
use vapurr_wallet::Address;
#[cfg(test)]
use vapurr_wallet::keccak4;

use crate::{Client, EconError};

/// Pre-scrub gen-4 selectors. Hex only in production paths.
const LIVE_SWAP_V_TO_PUSD: [u8; 4] = [0xa9, 0x26, 0xce, 0xdb];
const LIVE_SWAP_PUSD_TO_V: [u8; 4] = [0x4c, 0x43, 0x37, 0x3e];
const LIVE_VAPURR_RATE: [u8; 4] = [0xe2, 0x93, 0x55, 0x24];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarketAbi {
    /// Post-scrub: `swapVToPusd` / `vapurrRate`.
    Scrubbed,
    /// Live gen-4 deploy still on 46630.
    LiveGen4,
}

static DETECTED: OnceLock<MarketAbi> = OnceLock::new();

fn encode_sel(sel: [u8; 4]) -> Vec<u8> {
    sel.to_vec()
}

fn encode_sel_u256(sel: [u8; 4], n: u128) -> Vec<u8> {
    let mut d = Vec::with_capacity(36);
    d.extend_from_slice(&sel);
    d.extend_from_slice(&abi_u256(n));
    d
}

impl Client {
    pub(crate) fn market_abi(&self) -> MarketAbi {
        if let Some(a) = DETECTED.get() {
            return *a;
        }
        let detected = self.detect_market_abi();
        let _ = DETECTED.set(detected);
        detected
    }

    fn detect_market_abi(&self) -> MarketAbi {
        let Some(m) = self.live_market() else {
            return MarketAbi::Scrubbed;
        };
        let from = self.key.address.to_hex();
        let to = m.to_hex();
        let data = hex0x(&encode_fn("vapurrRate()"));
        match self.rpc.eth_call(&from, Some(&to), &data) {
            Ok(raw) => {
                if decode_hex_bytes(&raw)
                    .ok()
                    .and_then(|b| decode_word_u128(&b, 0))
                    .filter(|n| *n > 0)
                    .is_some()
                {
                    MarketAbi::Scrubbed
                } else {
                    MarketAbi::LiveGen4
                }
            }
            Err(_) => MarketAbi::LiveGen4,
        }
    }

    pub(crate) fn swap_v_to_pusd(&mut self, amt: u128) -> Result<String, EconError> {
        match self.market_abi() {
            MarketAbi::Scrubbed => self.transact("swapVToPusd(uint256)", amt),
            MarketAbi::LiveGen4 => self.transact_sel(LIVE_SWAP_V_TO_PUSD, amt),
        }
    }

    pub(crate) fn swap_pusd_to_v(&mut self, amt: u128) -> Result<String, EconError> {
        match self.market_abi() {
            MarketAbi::Scrubbed => self.transact("swapPusdToV(uint256)", amt),
            MarketAbi::LiveGen4 => self.transact_sel(LIVE_SWAP_PUSD_TO_V, amt),
        }
    }

    pub(crate) fn read_vapurr_rate(&self, market: Address) -> Option<u128> {
        let from = self.key.address.to_hex();
        let to = market.to_hex();
        let data = match self.market_abi() {
            MarketAbi::Scrubbed => hex0x(&encode_fn("vapurrRate()")),
            MarketAbi::LiveGen4 => hex0x(&encode_sel(LIVE_VAPURR_RATE)),
        };
        let raw = self.rpc.eth_call(&from, Some(&to), &data).ok()?;
        let bytes = decode_hex_bytes(&raw).ok()?;
        decode_word_u128(&bytes, 0)
    }

    fn transact_sel(&mut self, sel: [u8; 4], amt: u128) -> Result<String, EconError> {
        if amt == 0 {
            return Err(EconError::Tiny);
        }
        let market = self.live_market().ok_or(EconError::NotLive)?;
        let data = encode_sel_u256(sel, amt);
        self.send(Some(market), &data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_selectors_are_pinned() {
        assert_eq!(LIVE_SWAP_V_TO_PUSD, [0xa9, 0x26, 0xce, 0xdb]);
        assert_eq!(LIVE_SWAP_PUSD_TO_V, [0x4c, 0x43, 0x37, 0x3e]);
        assert_eq!(LIVE_VAPURR_RATE, [0xe2, 0x93, 0x55, 0x24]);
    }

    #[test]
    fn scrubbed_names_hash_differently() {
        assert_ne!(LIVE_SWAP_V_TO_PUSD, keccak4("swapVToPusd(uint256)"));
        assert_ne!(LIVE_SWAP_PUSD_TO_V, keccak4("swapPusdToV(uint256)"));
        assert_ne!(LIVE_VAPURR_RATE, keccak4("vapurrRate()"));
    }
}
