//! Pay-to-rank board. Rank is $PUSD paid.

use serde_json::{json, Value};
use url::Url;


use vapurr_wallet::tx::{
    decode_dyn_string, decode_hex_bytes, decode_word_addr, decode_word_u128, encode_fn,
    encode_fn_addr, encode_fn_addr_addr, encode_fn_addr_u256, encode_fn_bytes32,
    encode_fn_two_str_u256, encode_fn_u256, hex0x,
};
use vapurr_wallet::{addr_from_hex, keccak256, Address};

use crate::{fmt_tok, parse_amt, Client, EconError, DEC, MIN_GAS_WEI};

const OUTBID_HEX: &str = include_str!("outbid.hex");
const MIN_BID: u128 = 10 * DEC;
const MIN_OUTBID: u128 = 5 * DEC;
const MIN_RAISE: u128 = DEC;
const MAX_BID: u128 = 999_999 * DEC;
const MAX_LISTINGS: u128 = 256;

impl Client {
    pub(crate) fn outbid_snap(&self) -> Value {
        match self.outbid_snap_inner() {
            Ok(v) => v,
            Err(e) => self.outbid_base(&e.to_string()),
        }
    }

    pub(crate) fn outbid_bid(
        &mut self,
        url: &str,
        title: &str,
        amt: &str,
    ) -> Result<Value, EconError> {
        let url = canon_url(url)?;
        let title = canon_title(title, &url);
        let want = parse_amt(amt)?;
        let board = self.live_outbid().ok_or(EconError::NeedBoard)?;
        if self.live_pusd().is_none() {
            return Err(EconError::NotLive);
        }
        let (n, top) = self.board_stats(&board)?;
        let existing = self.listing_at(&board, &url)?;
        let pull = check_bid(want, top, n, existing, self.key.address)?;
        self.ensure_pusd(board, pull)?;
        let data = encode_fn_two_str_u256("bid(string,string,uint256)", &url, &title, want);
        self.send(Some(board), &data)?;
        Ok(self.outbid_snap())
    }

    pub(crate) fn outbid_deploy(&mut self) -> Result<String, EconError> {
        if self.live_outbid().is_some() {
            return Ok(self.cfg.outbid.clone());
        }
        let pusd = self.live_pusd().ok_or(EconError::NotLive)?;
        if self.cfg.pusd.is_empty() {
            self.cfg.pusd = pusd.to_checksum();
        }
        let from = self.key.address.to_hex();
        let eth = self.rpc.eth_balance(&from).map_err(crate::econ_rpc)?;
        if eth < MIN_GAS_WEI {
            return Err(EconError::NeedGas);
        }
        let mut bytecode = outbid_bytecode()?;
        bytecode.extend_from_slice(&vapurr_wallet::tx::abi_addr(pusd));
        let hash = self.send(None, &bytecode)?;
        let receipt = self.wait(&hash)?;
        let status = receipt
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0");
        if status != "0x1" {
            return Err(EconError::Rpc("outbid deploy reverted".into()));
        }
        let ca = receipt
            .get("contractAddress")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EconError::Rpc("no contractAddress".into()))?;
        let addr = addr_from_hex(ca).ok_or_else(|| EconError::Rpc("bad ca".into()))?;
        self.cfg.outbid = addr.to_checksum();
        self.cfg.save();
        Ok(hash)
    }

    fn outbid_snap_inner(&self) -> Result<Value, EconError> {
        let from = self.key.address;
        let eth = self.rpc.eth_balance(&from.to_hex()).unwrap_or(0);
        let market = self.live_market();
        let pusd = self.live_pusd();
        let board = self.live_outbid();
        let mut pusd_bal = 0u128;
        if let Some(p) = pusd {
            let data = encode_fn_addr("balanceOf(address)", from);
            if let Ok(raw) = self
                .rpc
                .eth_call(&from.to_hex(), Some(&p.to_hex()), &hex0x(&data))
            {
                if let Ok(bytes) = decode_hex_bytes(&raw) {
                    pusd_bal = decode_word_u128(&bytes, 0).unwrap_or(0);
                }
            }
        }
        if market.is_none() || pusd.is_none() {
            let mut v = self.outbid_base("");
            v["eth"] = json!(fmt_eth(eth));
            v["need_eth"] = json!(eth < MIN_GAS_WEI);
            v["need_market"] = json!(true);
            v["status"] = json!("Mint $PUSD first.");
            return Ok(v);
        }
        if board.is_none() {
            let mut v = self.outbid_base("");
            v["eth"] = json!(fmt_eth(eth));
            v["need_eth"] = json!(eth < MIN_GAS_WEI);
            v["need_market"] = json!(false);
            v["need_deploy"] = json!(true);
            v["pusd"] = json!(fmt_tok(pusd_bal));
            v["pusd_token"] = json!(pusd.unwrap().to_checksum());
            v["address"] = json!(from.to_checksum());
            v["status"] = json!(if eth < MIN_GAS_WEI {
                "Need gas."
            } else {
                ""
            });
            return Ok(v);
        }
        let b = board.unwrap();
        let stats = encode_fn("stats()");
        let raw = self
            .rpc
            .eth_call(&from.to_hex(), Some(&b.to_hex()), &hex0x(&stats))
            .map_err(crate::econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).map_err(|_| EconError::Rpc("stats".into()))?;
        let n = decode_word_u128(&bytes, 0).unwrap_or(0).min(64);
        let pot = decode_word_u128(&bytes, 1).unwrap_or(0);
        let top = decode_word_u128(&bytes, 2).unwrap_or(0);
        let mut listings = Vec::new();
        for i in 0..n {
            if let Some(row) = self.read_row(&b, i) {
                listings.push(row);
            }
        }
        listings.sort_by(|a, b| match b.paid.cmp(&a.paid) {
            std::cmp::Ordering::Equal => a.first_at.cmp(&b.first_at),
            o => o,
        });
        let mine = listings.iter().find(|r| r.bidder == from).cloned();
        let rows: Vec<Value> = listings
            .iter()
            .enumerate()
            .map(|(i, r)| {
                json!({
                    "rank": i + 1,
                    "url": r.url,
                    "title": r.title,
                    "paid": fmt_tok(r.paid),
                    "paid_raw": r.paid.to_string(),
                    "bidder": r.bidder.to_checksum(),
                    "mine": r.bidder == from,
                    "first_at": r.first_at,
                    "href": href(&r.url),
                })
            })
            .collect();
        let quote = quote_first(top, mine.as_ref().map(|m| m.paid));
        Ok(json!({
            "live": true,
            "need_deploy": false,
            "need_market": false,
            "need_eth": eth < MIN_GAS_WEI,
            "address": from.to_checksum(),
            "board": b.to_checksum(),
            "pusd_token": pusd.unwrap().to_checksum(),
            "explorer": format!("{}/address/{}", self.explorer(), b.to_hex()),
            "tx": self.last_tx,
            "tx_url": if self.last_tx.is_empty() {
                String::new()
            } else {
                format!("{}/tx/{}", self.explorer(), self.last_tx)
            },
            "eth": fmt_eth(eth),
            "pusd": fmt_tok(pusd_bal),
            "pot": fmt_tok(pot),
            "top": fmt_tok(top),
            "min_bid": "10",
            "min_outbid": "5",
            "quote_first": fmt_tok(quote),
            "listings": rows,
            "mine": mine.map(|m| json!({
                "url": m.url,
                "title": m.title,
                "paid": fmt_tok(m.paid),
            })),
            "status": "",
        }))
    }

    fn outbid_base(&self, err: &str) -> Value {
        json!({
            "live": false,
            "need_deploy": true,
            "need_market": self.live_market().is_none(),
            "need_eth": true,
            "address": self.key.address.to_checksum(),
            "board": self.cfg.outbid,
            "pusd_token": self.cfg.pusd,
            "explorer": "",
            "tx": self.last_tx,
            "tx_url": "",
            "eth": "0.000000",
            "pusd": "0.00",
            "pot": "0.00",
            "top": "0.00",
            "min_bid": "10",
            "min_outbid": "5",
            "quote_first": "10.00",
            "listings": [],
            "mine": null,
            "status": if err.is_empty() { String::new() } else { err.to_string() },
            "error": err,
        })
    }

    fn live_outbid(&self) -> Option<Address> {
        if self.cfg.outbid.is_empty() {
            return None;
        }
        let addr = addr_from_hex(&self.cfg.outbid)?;
        let code = self.rpc.eth_code(&addr.to_hex()).ok()?;
        let hex = code.trim().trim_start_matches("0x").trim();
        if hex.len() <= 2 {
            return None;
        }
        Some(addr)
    }

    pub(crate) fn live_pusd(&self) -> Option<Address> {
        if !self.cfg.pusd.is_empty() {
            if let Some(a) = addr_from_hex(&self.cfg.pusd) {
                return Some(a);
            }
        }
        let m = self.live_market()?;
        let data = encode_fn_addr("snapshot(address)", self.key.address);
        let raw = self
            .rpc
            .eth_call(&self.key.address.to_hex(), Some(&m.to_hex()), &hex0x(&data))
            .ok()?;
        let bytes = decode_hex_bytes(&raw).ok()?;
        decode_word_addr(&bytes, 9)
    }

    pub(crate) fn ensure_pusd(&mut self, spender: Address, need: u128) -> Result<(), EconError> {
        let pusd = self.live_pusd().ok_or(EconError::NotLive)?;
        let from = self.key.address;
        let data = encode_fn_addr("balanceOf(address)", from);
        let raw = self
            .rpc
            .eth_call(&from.to_hex(), Some(&pusd.to_hex()), &hex0x(&data))
            .map_err(crate::econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).unwrap_or_default();
        let bal = decode_word_u128(&bytes, 0).unwrap_or(0);
        if bal < need {
            return Err(EconError::NeedPusd);
        }
        let allow_data = encode_fn_addr_addr("allowance(address,address)", from, spender);
        let raw = self
            .rpc
            .eth_call(&from.to_hex(), Some(&pusd.to_hex()), &hex0x(&allow_data))
            .map_err(crate::econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).unwrap_or_default();
        let allow = decode_word_u128(&bytes, 0).unwrap_or(0);
        if allow >= need {
            return Ok(());
        }
        let approve = encode_fn_addr_u256("approve(address,uint256)", spender, u128::MAX);
        self.send(Some(pusd), &approve)?;
        Ok(())
    }

    pub(crate) fn board_stats(&self, board: &Address) -> Result<(u128, u128), EconError> {
        let data = encode_fn("stats()");
        let raw = self
            .rpc
            .eth_call(
                &self.key.address.to_hex(),
                Some(&board.to_hex()),
                &hex0x(&data),
            )
            .map_err(crate::econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).map_err(|_| EconError::Rpc("stats".into()))?;
        let n = decode_word_u128(&bytes, 0).unwrap_or(0);
        let top = decode_word_u128(&bytes, 2).unwrap_or(0);
        Ok((n, top))
    }

    fn listing_at(&self, board: &Address, url: &str) -> Result<Option<(Address, u128)>, EconError> {
        let key = keccak256(url.as_bytes());
        let data = encode_fn_bytes32("listings(bytes32)", &key);
        let raw = self
            .rpc
            .eth_call(
                &self.key.address.to_hex(),
                Some(&board.to_hex()),
                &hex0x(&data),
            )
            .map_err(crate::econ_rpc)?;
        let bytes = decode_hex_bytes(&raw).map_err(|_| EconError::Rpc("listing".into()))?;
        if bytes.len() < 4 * 32 {
            return Ok(None);
        }
        let first_at = decode_word_u128(&bytes, 2).unwrap_or(0);
        if first_at == 0 {
            return Ok(None);
        }
        let bidder = decode_word_addr(&bytes, 0).ok_or_else(|| EconError::Rpc("listing".into()))?;
        let paid = decode_word_u128(&bytes, 1).unwrap_or(0);
        Ok(Some((bidder, paid)))
    }

    fn read_row(&self, board: &Address, i: u128) -> Option<Row> {
        let data = encode_fn_u256("row(uint256)", i);
        let raw = self
            .rpc
            .eth_call(
                &self.key.address.to_hex(),
                Some(&board.to_hex()),
                &hex0x(&data),
            )
            .ok()?;
        let bytes = decode_hex_bytes(&raw).ok()?;
        if bytes.len() < 7 * 32 {
            return None;
        }
        let bidder = decode_word_addr(&bytes, 1)?;
        let paid = decode_word_u128(&bytes, 2)?;
        let first_at = decode_word_u128(&bytes, 3)?;
        let url_off = decode_word_u128(&bytes, 5)? as usize;
        let title_off = decode_word_u128(&bytes, 6)? as usize;
        let url = decode_dyn_string(&bytes, url_off).unwrap_or_default();
        let title = decode_dyn_string(&bytes, title_off).unwrap_or_default();
        if url.is_empty() {
            return None;
        }
        Some(Row {
            bidder,
            paid,
            first_at,
            url,
            title,
        })
    }
}

#[derive(Clone)]
struct Row {
    bidder: Address,
    paid: u128,
    first_at: u128,
    url: String,
    title: String,
}

fn outbid_bytecode() -> Result<Vec<u8>, EconError> {
    let s = OUTBID_HEX.trim().trim_start_matches("0x");
    hex::decode(s).map_err(|_| EconError::Rpc("outbid bytecode".into()))
}

fn fmt_eth(v: u128) -> String {
    let whole = v / DEC;
    let frac = (v % DEC) / (DEC / 1_000_000);
    format!("{whole}.{frac:06}")
}

fn check_bid(
    want: u128,
    top: u128,
    n: u128,
    existing: Option<(Address, u128)>,
    from: Address,
) -> Result<u128, EconError> {
    if want < MIN_RAISE || want > MAX_BID || want % DEC != 0 {
        return Err(EconError::Tiny);
    }
    match existing {
        None => {
            if want < MIN_BID {
                return Err(EconError::Tiny);
            }
            if n >= MAX_LISTINGS {
                return Err(EconError::Full);
            }
            if top != 0 && want > top && want < top + MIN_OUTBID {
                return Err(EconError::Top);
            }
            Ok(want)
        }
        Some((bidder, paid)) => {
            if bidder != from {
                return Err(EconError::Owned);
            }
            if want < paid + MIN_RAISE {
                return Err(EconError::Tiny);
            }
            if want > top && want < top + MIN_OUTBID {
                return Err(EconError::Top);
            }
            Ok(want - paid)
        }
    }
}

fn quote_first(top: u128, mine: Option<u128>) -> u128 {
    let need = if top == 0 { MIN_BID } else { top + MIN_OUTBID };
    match mine {
        Some(p) if p >= need => p + MIN_RAISE,
        Some(p) if need > p => need,
        _ => need,
    }
}

pub fn canon_url(raw: &str) -> Result<String, EconError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(EconError::BadUrl);
    }
    if s.starts_with('@') {
        let h = s
            .trim_start_matches('@')
            .trim()
            .trim_start_matches('@')
            .to_ascii_lowercase();
        if h.is_empty()
            || h.len() > 32
            || !h
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            return Err(EconError::BadUrl);
        }
        return Ok(format!("@{h}"));
    }
    let with = if s.contains("://") {
        s.to_string()
    } else {
        format!("https://{s}")
    };
    let mut u = Url::parse(&with).map_err(|_| EconError::BadUrl)?;
    if !matches!(u.scheme(), "http" | "https") {
        return Err(EconError::BadUrl);
    }
    if let Some(h) = u.host_str() {
        let mut h = h.to_ascii_lowercase();
        if let Some(rest) = h.strip_prefix("www.") {
            h = rest.to_string();
        }
        let _ = u.set_host(Some(&h));
    } else {
        return Err(EconError::BadUrl);
    }
    u.set_fragment(None);
    let drop = [
        "utm_source",
        "utm_medium",
        "utm_campaign",
        "utm_term",
        "utm_content",
        "utm_id",
        "fbclid",
        "gclid",
        "gbraid",
        "wbraid",
        "mc_cid",
        "mc_eid",
        "igshid",
        "si",
        "ref",
        "ref_src",
        "s",
    ];
    let kept: Vec<(String, String)> = u
        .query_pairs()
        .filter(|(k, _)| {
            let k = k.to_ascii_lowercase();
            !drop.contains(&k.as_str()) && !k.starts_with("utm_")
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if kept.is_empty() {
        u.set_query(None);
    } else {
        u.query_pairs_mut()
            .clear()
            .extend_pairs(kept.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    }
    let mut out = u.to_string();
    if out.ends_with('/') && u.path() == "/" && u.query().is_none() {
        out.pop();
    }
    if out.len() > 256 {
        return Err(EconError::BadUrl);
    }
    Ok(out)
}

fn canon_title(title: &str, url: &str) -> String {
    let t = title.trim();
    if !t.is_empty() {
        return t.chars().take(64).collect();
    }
    if let Some(h) = url.strip_prefix('@') {
        return format!("@{h}");
    }
    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| url.chars().take(64).collect())
}

pub fn href(url: &str) -> String {
    if let Some(h) = url.strip_prefix('@') {
        format!("https://x.com/{h}")
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canon_handle_and_url() {
        assert_eq!(canon_url("@Vapurr").unwrap(), "@vapurr");
        assert_eq!(
            canon_url("https://WWW.Example.com/").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            canon_url("https://foo.com/?utm_source=x&id=1").unwrap(),
            "https://foo.com/?id=1"
        );
        assert!(canon_url("").is_err());
        assert!(canon_url("ftp://x.com").is_err());
    }

    #[test]
    fn first_place_quote() {
        assert_eq!(quote_first(0, None), MIN_BID);
        assert_eq!(quote_first(20 * DEC, None), 25 * DEC);
        assert_eq!(quote_first(20 * DEC, Some(12 * DEC)), 25 * DEC);
        assert_eq!(quote_first(20 * DEC, Some(25 * DEC)), 26 * DEC);
    }

    #[test]
    fn bid_rules_pull_the_difference() {
        let a = Address([1u8; 20]);
        let b = Address([2u8; 20]);
        assert_eq!(check_bid(10 * DEC, 0, 0, None, a).unwrap(), 10 * DEC);
        assert!(check_bid(9 * DEC, 0, 0, None, a).is_err());
        assert_eq!(check_bid(25 * DEC, 20 * DEC, 1, None, a).unwrap(), 25 * DEC);
        assert!(matches!(
            check_bid(21 * DEC, 20 * DEC, 1, None, a),
            Err(EconError::Top)
        ));
        assert_eq!(
            check_bid(12 * DEC, 20 * DEC, 1, Some((a, 10 * DEC)), a).unwrap(),
            2 * DEC
        );
        assert!(matches!(
            check_bid(12 * DEC, 20 * DEC, 1, Some((b, 10 * DEC)), a),
            Err(EconError::Owned)
        ));
        assert!(matches!(
            check_bid(21 * DEC, 20 * DEC, 1, Some((a, 20 * DEC)), a),
            Err(EconError::Top)
        ));
        assert_eq!(
            check_bid(25 * DEC, 20 * DEC, 1, Some((a, 20 * DEC)), a).unwrap(),
            5 * DEC
        );
    }

    #[test]
    fn bytecode_loads() {
        let b = outbid_bytecode().unwrap();
        assert!(b.len() > 500);
        assert!(b[0] == 0x60 || b[0] == 0x61, "got {:#x}", b[0]);
    }

    #[test]
    fn selectors() {
        assert_eq!(vapurr_wallet::keccak4("stats()").len(), 4);
        assert_eq!(vapurr_wallet::keccak4("bid(string,string,uint256)").len(), 4);
        assert_eq!(vapurr_wallet::keccak4("row(uint256)").len(), 4);
        assert_eq!(vapurr_wallet::keccak4("listings(bytes32)").len(), 4);
    }
}
