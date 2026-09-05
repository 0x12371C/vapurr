//! Live PusdMarket ABI bridge.
//!
//! Source + Solidity use scrubbed names (`swapVToPusd`, `vapurrRate`). The
//! gen-4 book on 46630 (`TESTNET_MARKET`) was deployed before that scrub and
//! still answers the pre-scrub selectors. Pin those as hex so we never put
//! banned brand strings back in the tree. After a scrubbed redeploy, eth_call
//! on `vapurrRate()` succeeds and we prefer the scrubbed sigs automatically.
//!
//! Cache policy: store a result only after a conclusive probe. RPC / dual-miss
//! failures fail open to Scrubbed and are NOT cached, so the next call re-probes.

use std::sync::Mutex;

use vapurr_wallet::tx::{
    abi_u256, decode_hex_bytes, decode_word_u128, encode_fn, hex0x,
};
use vapurr_wallet::{keccak4, Address};

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

/// Successful detections only: (market, abi). Failures are never stored.
static CACHE: Mutex<Option<(Address, MarketAbi)>> = Mutex::new(None);

fn encode_sel(sel: [u8; 4]) -> Vec<u8> {
    sel.to_vec()
}

fn encode_sel_u256(sel: [u8; 4], n: u128) -> Vec<u8> {
    let mut d = Vec::with_capacity(36);
    d.extend_from_slice(&sel);
    d.extend_from_slice(&abi_u256(n));
    d
}

fn rate_from_raw(raw: &str) -> Option<u128> {
    let bytes = decode_hex_bytes(raw).ok()?;
    decode_word_u128(&bytes, 0).filter(|n| *n > 0)
}

impl Client {
    pub(crate) fn market_abi(&self) -> MarketAbi {
        let Some(m) = self.live_market() else {
            return MarketAbi::Scrubbed;
        };
        if let Ok(guard) = CACHE.lock() {
            if let Some((addr, abi)) = *guard {
                if addr == m {
                    return abi;
                }
            }
        }
        match self.detect_market_abi(m) {
            Some(abi) => {
                if let Ok(mut guard) = CACHE.lock() {
                    *guard = Some((m, abi));
                }
                abi
            }
            // Fail open to tree source names; do not stick on a failed detect.
            None => MarketAbi::Scrubbed,
        }
    }

    /// Conclusive probe only. `None` means re-try next call (nothing cached).
    fn detect_market_abi(&self, market: Address) -> Option<MarketAbi> {
        let from = self.key.address.to_hex();
        let to = market.to_hex();

        let scrubbed = hex0x(&encode_fn("vapurrRate()"));
        match self.rpc.eth_call(&from, Some(&to), &scrubbed) {
            Ok(raw) => {
                if rate_from_raw(&raw).is_some() {
                    return Some(MarketAbi::Scrubbed);
                }
            }
            Err(_) => {
                // Could be network blip OR missing selector. Probe live hex next.
            }
        }

        let live = hex0x(&encode_sel(LIVE_VAPURR_RATE));
        match self.rpc.eth_call(&from, Some(&to), &live) {
            Ok(raw) if rate_from_raw(&raw).is_some() => Some(MarketAbi::LiveGen4),
            // Both inconclusive (RPC down, empty book, bad payload) ? re-probe later.
            _ => None,
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
        rate_from_raw(&raw)
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

    #[test]
    fn cache_accepts_and_clears_pairs() {
        let m = Address([1u8; 20]);
        {
            let mut g = CACHE.lock().unwrap();
            *g = Some((m, MarketAbi::LiveGen4));
        }
        assert_eq!(*CACHE.lock().unwrap(), Some((m, MarketAbi::LiveGen4)));
        {
            let mut g = CACHE.lock().unwrap();
            *g = None;
        }
        assert_eq!(*CACHE.lock().unwrap(), None);
    }

    #[test]
    fn rate_from_raw_requires_positive() {
        assert!(rate_from_raw("0x").is_none());
        let mut word = vec![0u8; 32];
        word[31] = 1;
        let hex: String = word.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(rate_from_raw(&format!("0x{hex}")), Some(1));
    }
}
