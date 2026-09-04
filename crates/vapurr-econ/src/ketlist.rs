//! Ketcharts listing board. Rank is $PUSD paid.

use serde_json::{json, Map, Value};

use vapurr_wallet::tx::{
    decode_dyn_string, decode_hex_bytes, decode_word_addr, decode_word_u128, encode_fn,
    encode_fn_bytes32, encode_fn_two_addr_three_str_u256, encode_fn_u256, hex0x,
};
use vapurr_wallet::{addr_from_hex, keccak256, Address};

use crate::{fmt_tok, parse_amt, Client, EconError, DEC, MIN_GAS_WEI};

const KETLIST_HEX: &str = include_str!("ketlist.hex");
const MIN_LIST: u128 = 50 * DEC;
const MIN_OUTBID: u128 = 25 * DEC;
const MIN_RAISE: u128 = 10 * DEC;
const MAX_LIST: u128 = 999_999 * DEC;
const MAX_LISTINGS: u128 = 256;
const MAX_SNAP: u128 = 96;
const MAX_SYM: usize = 16;
const MAX_NAME: usize = 64;
const MAX_META: usize = 512;

impl Client {
    pub(crate) fn ketlist_snap(&self) -> Value {
        match self.ketlist_snap_inner() {
            Ok(v) => v,
            Err(e) => self.ketlist_base(&e.to_string()),
        }
    }

    pub(crate) fn ketlist_pay(
        &mut self,
        token: &str,
        pool: &str,
        symbol: &str,
        name: &str,
        amt: &str,
        meta: &str,
    ) -> Result<Value, EconError> {
        let token = parse_ca(token)?;
        let pool = parse_ca(pool).map_err(|_| EconError::BadPool)?;
        if pool.0 == token.0 {
            return Err(EconError::BadPool);
        }
        let symbol = canon_sym(symbol)?;
        let name = canon_name(name)?;
        let meta = canon_meta(meta)?;
        let want = parse_amt(amt)?;
        let board = self.live_ketlist().ok_or(EconError::NeedBoard)?;
        if self.live_pusd().is_none() {
            return Err(EconError::NotLive);
        }
        if !self.code_at(&token) {
            return Err(EconError::BadToken);
        }
        if !self.code_at(&pool) {
            return Err(EconError::BadPool);
        }
        let (n, top) = self.board_stats(&board)?;
        let existing = self.ket_listing_at(&board, &token)?;
        let pull = check_list(want, top, n, existing, self.key.address)?;
        self.ensure_pusd(board, pull)?;
        let data = encode_fn_two_addr_three_str_u256(
            "list(address,address,string,string,string,uint256)",
            token,
            pool,
            &symbol,
            &name,
            &meta,
            want,
        );
        self.send(Some(board), &data)?;
        Ok(self.ketlist_snap())
    }

    pub(crate) fn ketlist_deploy(&mut self) -> Result<String, EconError> {
        if self.live_ketlist().is_some() {
            return Ok(self.cfg.ketlist.clone());
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
        let mut bytecode = ketlist_bytecode()?;
        bytecode.extend_from_slice(&vapurr_wallet::tx::abi_addr(pusd));
        let hash = self.send(None, &bytecode)?;
        let receipt = self.wait(&hash)?;
        let status = receipt
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0");
        if status != "0x1" {
            return Err(EconError::Rpc("ketlist deploy reverted".into()));
        }
        let ca = receipt
            .get("contractAddress")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EconError::Rpc("no contractAddress".into()))?;
        let addr = addr_from_hex(ca).ok_or_else(|| EconError::Rpc("bad ca".into()))?;
        self.cfg.ketlist = addr.to_checksum();
        self.cfg.gen = crate::GEN;
        self.cfg.save();
        Ok(hash)
    }

    fn ketlist_snap_inner(&self) -> Result<Value, EconError> {
        let from = self.key.address;
        let eth = self.rpc.eth_balance(&from.to_hex()).unwrap_or(0);
        let market = self.live_market();
        let pusd = self.live_pusd();
        let board = self.live_ketlist();
        let mut pusd_bal = 0u128;
        if let Some(p) = pusd {
            pusd_bal = self.token_raw(p, from);
        }
        if market.is_none() || pusd.is_none() {
            let mut v = self.ketlist_base("");
            v["eth"] = json!(fmt_eth(eth));
            v["need_eth"] = json!(eth < MIN_GAS_WEI);
            v["need_market"] = json!(true);
            v["status"] = json!("Mint $PUSD first.");
            return Ok(v);
        }
        if board.is_none() {
            let mut v = self.ketlist_base("");
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
        let n = decode_word_u128(&bytes, 0).unwrap_or(0).min(MAX_SNAP);
        let pot = decode_word_u128(&bytes, 1).unwrap_or(0);
        let top = decode_word_u128(&bytes, 2).unwrap_or(0);
        let mut listings = Vec::new();
        for i in 0..n {
            if let Some(row) = self.read_list_row(&b, i) {
                listings.push(row);
            }
        }
        listings.sort_by(|a, b| match b.paid.cmp(&a.paid) {
            std::cmp::Ordering::Equal => a.first_at.cmp(&b.first_at),
            o => o,
        });
        let mine = listings.iter().find(|r| r.lister == from).cloned();
        let mut by_token = Map::new();
        let rows: Vec<Value> = listings
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let rank = i + 1;
                let rec = listing_json(rank, r, from);
                by_token.insert(r.token.to_hex().to_ascii_lowercase(), rec.clone());
                rec
            })
            .collect();
        write_inbox(&b.to_checksum(), &rows);
        let quote = quote_first(top, mine.as_ref().map(|m| m.paid));
        Ok(json!({
            "live": true,
            "need_deploy": false,
            "need_market": false,
            "need_eth": eth < MIN_GAS_WEI,
            "address": from.to_checksum(),
            "board": b.to_checksum(),
            "pusd_token": pusd.unwrap().to_checksum(),
            "explorer": self.explorer(),
            "board_url": format!("{}/address/{}", self.explorer(), b.to_hex()),
            "tx": self.last_tx,
            "tx_url": if self.last_tx.is_empty() {
                String::new()
            } else {
                format!("{}/tx/{}", self.explorer(), self.last_tx)
            },
            "chain_id": self.chain_id,
            "net": if self.chain_id == vapurr_rhc::TESTNET_CHAIN_ID { "testnet" } else { "mainnet" },
            "eth": fmt_eth(eth),
            "pusd": fmt_tok(pusd_bal),
            "pot": fmt_tok(pot),
            "top": fmt_tok(top),
            "min_list": "50",
            "min_raise": "10",
            "min_outbid": "25",
            "quote_first": fmt_tok(quote),
            "listings": rows,
            "by_token": by_token,
            "mine": mine.map(|m| listing_json(0, &m, from)),
            "status": "",
        }))
    }

    fn ketlist_base(&self, err: &str) -> Value {
        json!({
            "live": false,
            "need_deploy": true,
            "need_market": self.live_market().is_none(),
            "need_eth": true,
            "address": self.key.address.to_checksum(),
            "board": self.cfg.ketlist,
            "pusd_token": self.cfg.pusd,
            "explorer": self.explorer(),
            "board_url": "",
            "tx": self.last_tx,
            "tx_url": "",
            "chain_id": self.chain_id,
            "net": if self.chain_id == vapurr_rhc::TESTNET_CHAIN_ID { "testnet" } else { "mainnet" },
            "eth": "0.000000",
            "pusd": "0.00",
            "pot": "0.00",
            "top": "0.00",
            "min_list": "50",
            "min_raise": "10",
            "min_outbid": "25",
            "quote_first": "50.00",
            "listings": [],
            "by_token": {},
            "mine": null,
            "status": if err.is_empty() { String::new() } else { err.to_string() },
            "error": err,
        })
    }

    fn live_ketlist(&self) -> Option<Address> {
        if self.cfg.ketlist.is_empty() {
            return None;
        }
        let addr = addr_from_hex(&self.cfg.ketlist)?;
        if self.code_at(&addr) {
            Some(addr)
        } else {
            None
        }
    }

    fn code_at(&self, a: &Address) -> bool {
        let code = self.rpc.eth_code(&a.to_hex()).unwrap_or_default();
        let hex = code.trim().trim_start_matches("0x").trim();
        hex.len() > 2
    }

    fn ket_listing_at(&self, board: &Address, token: &Address) -> Result<Option<(Address, u128)>, EconError> {
        let key = keccak256(&token.0);
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
        if bytes.len() < 6 * 32 {
            return Ok(None);
        }
        let first_at = decode_word_u128(&bytes, 4).unwrap_or(0);
        if first_at == 0 {
            return Ok(None);
        }
        let lister = decode_word_addr(&bytes, 0).ok_or_else(|| EconError::Rpc("listing".into()))?;
        let paid = decode_word_u128(&bytes, 3).unwrap_or(0);
        Ok(Some((lister, paid)))
    }

    fn read_list_row(&self, board: &Address, i: u128) -> Option<Row> {
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
        if bytes.len() < 10 * 32 {
            return None;
        }
        let lister = decode_word_addr(&bytes, 1)?;
        let token = decode_word_addr(&bytes, 2)?;
        let pool = decode_word_addr(&bytes, 3)?;
        let paid = decode_word_u128(&bytes, 4)?;
        let first_at = decode_word_u128(&bytes, 5)?;
        let sym_off = decode_word_u128(&bytes, 7)? as usize;
        let name_off = decode_word_u128(&bytes, 8)? as usize;
        let meta_off = decode_word_u128(&bytes, 9).unwrap_or(0) as usize;
        let symbol = decode_dyn_string(&bytes, sym_off).unwrap_or_default();
        let name = decode_dyn_string(&bytes, name_off).unwrap_or_default();
        let meta = if meta_off >= 32 {
            decode_dyn_string(&bytes, meta_off).unwrap_or_default()
        } else {
            String::new()
        };
        if token.0.iter().all(|&b| b == 0) {
            return None;
        }
        Some(Row {
            lister,
            token,
            pool,
            paid,
            first_at,
            symbol,
            name,
            meta,
        })
    }
}

#[derive(Clone)]
struct Row {
    lister: Address,
    token: Address,
    pool: Address,
    paid: u128,
    first_at: u128,
    symbol: String,
    name: String,
    meta: String,
}

fn listing_json(rank: usize, r: &Row, from: Address) -> Value {
    let p = parse_meta(&r.meta);
    json!({
        "rank": rank,
        "token": r.token.to_checksum(),
        "pool": r.pool.to_checksum(),
        "symbol": r.symbol,
        "name": r.name,
        "paid": fmt_tok(r.paid),
        "paid_raw": r.paid.to_string(),
        "lister": r.lister.to_checksum(),
        "mine": r.lister == from,
        "first_at": r.first_at,
        "website": p.website,
        "twitter": p.twitter,
        "telegram": p.telegram,
        "discord": p.discord,
        "bio": p.bio,
        "logo": p.logo,
    })
}

fn write_inbox(board: &str, listings: &[Value]) {
    let v = json!({
        "board": board,
        "listings": listings,
    });
    if let Ok(bytes) = serde_json::to_vec_pretty(&v) {
        let _ = std::fs::create_dir_all(vapurr_wallet::data_dir());
        let _ = std::fs::write(vapurr_wallet::data_dir().join("ketlist.json"), bytes);
    }
}

fn ketlist_bytecode() -> Result<Vec<u8>, EconError> {
    let s = KETLIST_HEX.trim().trim_start_matches("0x");
    hex::decode(s).map_err(|_| EconError::Rpc("ketlist bytecode".into()))
}

fn fmt_eth(v: u128) -> String {
    let whole = v / DEC;
    let frac = (v % DEC) / (DEC / 1_000_000);
    format!("{whole}.{frac:06}")
}

fn parse_ca(s: &str) -> Result<Address, EconError> {
    let a = addr_from_hex(s.trim()).ok_or(EconError::BadToken)?;
    if a.0.iter().all(|&b| b == 0) {
        return Err(EconError::BadToken);
    }
    Ok(a)
}

fn canon_sym(s: &str) -> Result<String, EconError> {
    let t: String = s
        .trim()
        .chars()
        .take(MAX_SYM)
        .collect::<String>()
        .trim()
        .to_ascii_uppercase();
    if t.is_empty()
        || t.len() > MAX_SYM
        || !t
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '$' || c == '.' || c == '_')
    {
        return Err(EconError::BadTicker);
    }
    Ok(t)
}

fn canon_name(s: &str) -> Result<String, EconError> {
    let t: String = s.trim().chars().take(MAX_NAME).collect();
    if t.is_empty() {
        return Err(EconError::BadTicker);
    }
    Ok(t)
}

struct Profile {
    website: String,
    twitter: String,
    telegram: String,
    discord: String,
    bio: String,
    logo: String,
}

fn parse_meta(s: &str) -> Profile {
    let mut p = Profile {
        website: String::new(),
        twitter: String::new(),
        telegram: String::new(),
        discord: String::new(),
        bio: String::new(),
        logo: String::new(),
    };
    let Ok(v) = serde_json::from_str::<Value>(s) else {
        return p;
    };
    let obj = match v.as_object() {
        Some(o) => o,
        None => return p,
    };
    p.website = take_field(obj, &["w", "web", "website"], 128);
    p.twitter = take_field(obj, &["x", "twitter"], 32);
    p.telegram = take_field(obj, &["t", "tg", "telegram"], 64);
    p.discord = take_field(obj, &["d", "dc", "discord"], 64);
    p.bio = take_field(obj, &["b", "bio"], 160);
    p.logo = take_field(obj, &["l", "logo"], 160);
    p
}

fn take_field(obj: &Map<String, Value>, keys: &[&str], max: usize) -> String {
    for k in keys {
        if let Some(s) = obj.get(*k).and_then(|x| x.as_str()) {
            let t = s.trim();
            if !t.is_empty() {
                return t.chars().take(max).collect();
            }
        }
    }
    String::new()
}

fn canon_link(s: &str, web: bool) -> Result<String, EconError> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(String::new());
    }
    if t.contains('\0') || t.contains('<') || t.contains('\n') {
        return Err(EconError::BadTicker);
    }
    let low = t.to_ascii_lowercase();
    if low.starts_with("javascript:") || low.starts_with("data:") {
        return Err(EconError::BadTicker);
    }
    if web && t.contains("://") && !(low.starts_with("https://") || low.starts_with("http://")) {
        return Err(EconError::BadTicker);
    }
    Ok(t.to_string())
}

fn canon_meta(raw: &str) -> Result<String, EconError> {
    let s = raw.trim();
    if s.is_empty() {
        return Ok(String::new());
    }
    let p = parse_meta(s);
    let mut o = Map::new();
    let w = canon_link(&p.website, true)?;
    let x = canon_link(&p.twitter.trim_start_matches('@'), false)?;
    let t = canon_link(&p.telegram, false)?;
    let d = canon_link(&p.discord, false)?;
    let b = p.bio.trim().chars().take(160).collect::<String>();
    let l = canon_link(&p.logo, true)?;
    if !w.is_empty() {
        o.insert("w".into(), json!(w));
    }
    if !x.is_empty() {
        o.insert("x".into(), json!(x));
    }
    if !t.is_empty() {
        o.insert("t".into(), json!(t));
    }
    if !d.is_empty() {
        o.insert("d".into(), json!(d));
    }
    if !b.is_empty() {
        o.insert("b".into(), json!(b));
    }
    if !l.is_empty() {
        o.insert("l".into(), json!(l));
    }
    if o.is_empty() {
        return Ok(String::new());
    }
    let out = serde_json::to_string(&Value::Object(o)).unwrap_or_default();
    if out.len() > MAX_META {
        return Err(EconError::Tiny);
    }
    Ok(out)
}

fn check_list(
    want: u128,
    top: u128,
    n: u128,
    existing: Option<(Address, u128)>,
    from: Address,
) -> Result<u128, EconError> {
    if want < MIN_RAISE || want > MAX_LIST || want % DEC != 0 {
        return Err(EconError::Tiny);
    }
    match existing {
        None => {
            if want < MIN_LIST {
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
        Some((lister, paid)) => {
            if lister != from {
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
    let need = if top == 0 { MIN_LIST } else { top + MIN_OUTBID };
    match mine {
        Some(p) if p >= need => p + MIN_RAISE,
        Some(p) if need > p => need,
        _ => need,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_place_quote() {
        assert_eq!(quote_first(0, None), MIN_LIST);
        assert_eq!(quote_first(80 * DEC, None), 105 * DEC);
        assert_eq!(quote_first(80 * DEC, Some(50 * DEC)), 105 * DEC);
        assert_eq!(quote_first(80 * DEC, Some(105 * DEC)), 115 * DEC);
    }

    #[test]
    fn list_rules_pull_the_difference() {
        let a = Address([1u8; 20]);
        let b = Address([2u8; 20]);
        assert_eq!(check_list(50 * DEC, 0, 0, None, a).unwrap(), 50 * DEC);
        assert!(check_list(49 * DEC, 0, 0, None, a).is_err());
        assert_eq!(
            check_list(105 * DEC, 80 * DEC, 1, None, a).unwrap(),
            105 * DEC
        );
        assert!(matches!(
            check_list(90 * DEC, 80 * DEC, 1, None, a),
            Err(EconError::Top)
        ));
        assert_eq!(
            check_list(60 * DEC, 80 * DEC, 1, Some((a, 50 * DEC)), a).unwrap(),
            10 * DEC
        );
        assert!(matches!(
            check_list(60 * DEC, 80 * DEC, 1, Some((b, 50 * DEC)), a),
            Err(EconError::Owned)
        ));
        assert_eq!(
            check_list(105 * DEC, 80 * DEC, 1, Some((a, 80 * DEC)), a).unwrap(),
            25 * DEC
        );
    }

    #[test]
    fn ticker_rules() {
        assert_eq!(canon_sym("foo").unwrap(), "FOO");
        assert_eq!(canon_sym("$PUSD").unwrap(), "$PUSD");
        assert!(canon_sym("").is_err());
        assert!(canon_sym("this-is-not-ok").is_err());
        assert_eq!(canon_name("  Foo Token  ").unwrap(), "Foo Token");
        assert!(canon_name("   ").is_err());
    }

    #[test]
    fn meta_keeps_safe_fields() {
        let m = canon_meta(r#"{"w":"https://foo.hood","x":"@Cat","b":"lime cat","js":"no"}"#).unwrap();
        assert!(m.contains("https://foo.hood"));
        let p = parse_meta(&m);
        assert_eq!(p.website, "https://foo.hood");
        assert_eq!(p.twitter, "Cat");
        assert_eq!(p.bio, "lime cat");
        assert!(p.logo.is_empty());
        assert!(canon_meta(r#"{"w":"javascript:alert(1)"}"#).is_err());
        assert_eq!(canon_meta("").unwrap(), "");
    }

    #[test]
    fn bytecode_loads() {
        let b = ketlist_bytecode().unwrap();
        assert!(b.len() > 500);
        assert!(b[0] == 0x60 || b[0] == 0x61, "got {:#x}", b[0]);
    }

    #[test]
    fn selectors() {
        assert_eq!(vapurr_wallet::keccak4("stats()").len(), 4);
        assert_eq!(
            vapurr_wallet::keccak4("list(address,address,string,string,string,uint256)").len(),
            4
        );
        assert_eq!(vapurr_wallet::keccak4("row(uint256)").len(), 4);
        assert_eq!(vapurr_wallet::keccak4("listings(bytes32)").len(), 4);
    }

    #[test]
    fn token_key_is_packed_address() {
        let a = Address([0xabu8; 20]);
        let k = keccak256(&a.0);
        assert_eq!(k.len(), 32);
        assert_ne!(k, [0u8; 32]);
    }
}
