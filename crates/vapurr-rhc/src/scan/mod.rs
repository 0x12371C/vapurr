//! Native Robinhood Chain explorer. JSON-RPC only. Index is history, not chrome.

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::rpc::{fmt_gwei, hex_u128 as hex_u128_val, hex_u64, Rpc, RpcError, TRANSFER_TOPIC};
use crate::{
    CHAIN_ID, CHAIN_NAME, ENTRY_POINT_V07, ENTRY_POINT_V08, NATIVE, NATIVE_SYMBOL, PERMIT2,
    RPC_HTTP, SUSHI_V3_FACTORY, UNI_V2_FACTORY, UNI_V3_FACTORY, UNI_V4_POOL_MANAGER,
    UNI_V4_POSITION_MANAGER, USDG, USDG_DECIMALS, USDE, WETH,
};

mod labels;
mod decode;
mod search;
mod token;
mod tx;
mod addr;
mod head;

use labels::*;
use decode::*;
use search::*;
use token::*;
use tx::*;
use addr::*;
use head::*;


const HEAD_TTL: Duration = Duration::from_millis(1600);
const ERR_TTL: Duration = Duration::from_millis(800);
const LOG_SPAN: u64 = 2048;
const OVERVIEW_BLOCKS: u64 = 12;
const OVERVIEW_TXS: usize = 32;
const BLOCK_TX_PAGE: usize = 50;

/// Approval(address,address,uint256)
const APPROVAL_TOPIC: &str =
    "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925";

static RPC: OnceLock<Rpc> = OnceLock::new();
static HEAD: Mutex<Option<(Instant, Value)>> = Mutex::new(None);
static HEAD_LOOP: AtomicBool = AtomicBool::new(false);
static TOKEN_META: Mutex<Option<HashMap<String, Option<(u8, String)>>>> = Mutex::new(None);

pub fn warm() {
    kick_head();
    crate::index::kick();
    crate::liq::warm();
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    BlockNumber(u64),
    Hash(String),
    Address(String),
    Unknown,
}

pub fn classify(raw: &str) -> Query {
    let s = raw.trim();
    let s = s
        .trim_start_matches("block:")
        .trim_start_matches("block ")
        .trim_start_matches("blk:")
        .trim_start_matches("blk ")
        .trim_start_matches("scan:")
        .trim_start_matches("scan ")
        .trim_start_matches('#')
        .trim();
    if s.is_empty() {
        return Query::Unknown;
    }
    if s.eq_ignore_ascii_case("USDG") || s.eq_ignore_ascii_case("$USDG") {
        return Query::Address(USDG.to_ascii_lowercase());
    }
    let hex = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or("");
    if !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        let n = hex.len();
        if n == 40 {
            return Query::Address(format!("0x{}", hex.to_ascii_lowercase()));
        }
        if n == 64 {
            return Query::Hash(format!("0x{}", hex.to_ascii_lowercase()));
        }
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(n) = s.parse::<u64>() {
            return Query::BlockNumber(n);
        }
    }
    Query::Unknown
}

pub fn is_scan_query(raw: &str) -> bool {
    let s = raw.trim().trim_start_matches('@');
    if s.len() > 5 && s.to_ascii_lowercase().ends_with(".hood") {
        return true;
    }
    !matches!(classify(raw), Query::Unknown)
}

pub fn api(kind: &str, query: &str) -> String {
    let (kind, query) = parse_kind_query(kind, query);
    match dispatch(&kind, &query) {
        Ok(v) => v.to_string(),
        Err(e) => json!({ "ok": false, "error": e }).to_string(),
    }
}

/// Custom-protocol fetches often drop `?h=` / `?id=`. Prefer `/scan/api/tx/{hash}`.
/// Extras ride in the path (`block/{id}/page/{n}`, `addr/{a}/tab/{t}/cursor/{c}`)
/// or as a stuffed `?…` segment when the URI parser never splits query.
fn parse_kind_query(kind: &str, query: &str) -> (String, String) {
    let kind = kind.trim_start_matches('/').trim_end_matches('/');
    let (kind, path_qs) = match kind.split_once('?') {
        Some((k, q)) => (k, q),
        None => (kind, ""),
    };
    let mut parts: Vec<String> = kind
        .split('/')
        .filter(|s| !s.is_empty())
        .map(urldecode)
        .collect();
    let verb = if parts.is_empty() {
        kind.to_string()
    } else {
        parts.remove(0)
    };
    let mut q = String::new();
    fn push_q(q: &mut String, s: &str) {
        if s.is_empty() {
            return;
        }
        if !q.is_empty() {
            q.push('&');
        }
        q.push_str(s);
    }
    push_q(&mut q, path_qs);
    push_q(&mut q, query);
    const EXTRA: &[&str] = &["page", "cursor", "tab", "holders"];
    let mut i = 0;
    let mut id: Option<String> = None;
    while i < parts.len() {
        let p = parts[i].as_str();
        if let Some(rest) = p.strip_prefix('?') {
            push_q(&mut q, rest);
            i += 1;
            continue;
        }
        if EXTRA.iter().any(|k| p.eq_ignore_ascii_case(k)) {
            if i + 1 < parts.len() {
                let key = p.to_ascii_lowercase();
                let val = parts[i + 1].clone();
                if param(&q, &key).is_none() {
                    push_q(&mut q, &format!("{}={}", key, urlencode(&val)));
                }
                i += 2;
                continue;
            }
        }
        if id.is_none() {
            id = Some(p.to_string());
        }
        i += 1;
    }
    if let Some(extra) = id {
        let key = match verb.as_str() {
            "tx" => "h",
            "block" => "id",
            "addr" | "token" | "holders" => "a",
            "search" | "suggest" => "q",
            "txs" | "blocks" | "tokens" | "head" | "gas" | "liq" => "",
            _ => "id",
        };
        if !key.is_empty() && param(&q, key).is_none() {
            push_q(&mut q, &format!("{}={}", key, urlencode(&extra)));
        }
    }
    (verb, q)
}

fn dispatch(kind: &str, query: &str) -> Result<Value, String> {
    match kind {
        "head" => {
            kick_head();
            Ok(head_snapshot())
        }
        "search" => search(&param(query, "q").unwrap_or_default()),
        "suggest" => Ok(json!({
            "ok": true,
            "items": suggest(&param(query, "q").unwrap_or_default()),
        })),
        "block" => {
            let id = param(query, "id").ok_or("missing id")?;
            let page = param(query, "page")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1);
            block(&id, page)
        }
        "tx" => {
            let h = param(query, "h").ok_or("missing hash")?;
            tx(&h)
        }
        "addr" => {
            let a = param(query, "a").ok_or("missing address")?;
            addr(
                &a,
                param(query, "tab").as_deref(),
                param(query, "cursor").as_deref(),
            )
        }
        "gas" => {
            kick_head();
            Ok(head_snapshot().get("gas").cloned().unwrap_or(json!({
                "ok": false,
                "loading": true,
                "error": "rpc wait"
            })))
        }
        "txs" => list_txs(param(query, "cursor").as_deref()),
        "blocks" => list_blocks(param(query, "cursor").as_deref()),
        "tokens" => list_tokens(param(query, "cursor").as_deref()),
        "token" => {
            let a = param(query, "a").ok_or("missing address")?;
            token_api(
                &a,
                param(query, "cursor").as_deref(),
                param(query, "holders").as_deref(),
            )
        }
        "holders" => {
            let a = param(query, "a").ok_or("missing address")?;
            holders_api(&a, param(query, "cursor").as_deref())
        }
        "liq" => {
            crate::liq::warm();
            Ok(crate::liq::snapshot())
        }
        _ => Err("unknown".into()),
    }
}
fn param(query: &str, key: &str) -> Option<String> {
    for part in query.split('&') {
        let (k, v) = part.split_once('=').unwrap_or((part, ""));
        if k == key && !v.is_empty() {
            return Some(urldecode(v));
        }
    }
    None
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn urldecode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => {
                let h = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("00"), 16)
                    .unwrap_or(b'?');
                out.push(h as char);
                i += 3;
            }
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

fn rpc() -> &'static Rpc {
    RPC.get_or_init(Rpc::new)
}

fn with_rpc<T>(f: impl FnOnce(&Rpc) -> Result<T, RpcError>) -> Result<T, String> {
    f(rpc()).map_err(|e| e.to_string())
}

fn call(method: &str, params: Value) -> Result<Value, String> {
    with_rpc(|rpc| rpc.call(method, params))
}

fn batch(reqs: Vec<Value>) -> Result<Vec<Value>, String> {
    if reqs.is_empty() {
        return Ok(Vec::new());
    }
    match with_rpc(|rpc| rpc.batch(&reqs)) {
        Ok(v) => Ok(v),
        Err(_) => {
            let mut out = Vec::with_capacity(reqs.len());
            for r in reqs {
                let method = r
                    .get("method")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let params = r.get("params").cloned().unwrap_or(json!([]));
                out.push(call(&method, params).unwrap_or(Value::Null));
            }
            Ok(out)
        }
    }
}
fn is_rpc_cursor(cursor: Option<&str>) -> bool {
    let Some(c) = cursor.map(str::trim).filter(|s| !s.is_empty()) else {
        return false;
    };
    if let Ok(Value::Object(o)) = serde_json::from_str::<Value>(c) {
        return o.get("to").is_some() && o.get("items_count").is_none();
    }
    c.chars().all(|ch| ch.is_ascii_digit())
}

fn log_window(head: u64, cursor: Option<&str>) -> (u64, u64, Option<Value>) {
    let to = cursor
        .and_then(|c| {
            if let Ok(v) = serde_json::from_str::<Value>(c) {
                v.get("to")
                    .and_then(|x| x.as_u64())
                    .or_else(|| v.get("block").and_then(|x| x.as_u64()))
            } else {
                c.parse().ok()
            }
        })
        .unwrap_or(head)
        .min(head);
    let from = to.saturating_sub(LOG_SPAN);
    let next = if from > 0 {
        Some(json!({ "to": from.saturating_sub(1) }))
    } else {
        None
    };
    (from, to, next)
}
fn balance_of_data(addr: &str) -> String {
    let hex = addr.trim_start_matches("0x").trim_start_matches("0X");
    format!("0x70a08231000000000000000000000000{hex}")
}

fn pad_topic(addr: &str) -> String {
    let hex = addr.trim_start_matches("0x").trim_start_matches("0X");
    format!("0x000000000000000000000000{hex}")
}

fn unpad_addr(topic: &str) -> String {
    let h = topic.trim_start_matches("0x").trim_start_matches("0X");
    if h.len() >= 40 {
        format!("0x{}", h[h.len() - 40..].to_ascii_lowercase())
    } else {
        format!("0x{}", h.to_ascii_lowercase())
    }
}

fn hex_n(n: u64) -> String {
    format!("0x{n:x}")
}

fn str_of(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn str_val(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        _ => "0x0".into(),
    }
}

fn opt_u64(v: Option<&Value>) -> Value {
    match v {
        None | Some(Value::Null) => Value::Null,
        Some(Value::String(s)) if s.is_empty() || s.eq_ignore_ascii_case("0x") => Value::Null,
        Some(x) => json!(hex_u64(x)),
    }
}

fn hex_u128(s: &str) -> u128 {
    let h = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    if h.is_empty() {
        return 0;
    }
    let h = if h.len() > 32 { &h[h.len() - 32..] } else { h };
    u128::from_str_radix(h, 16).unwrap_or(0)
}

fn fmt_eth_hex(s: &str) -> String {
    fmt_fixed(hex_u128(s), 18, NATIVE_SYMBOL)
}

fn fmt_token(s: &str, decimals: u8, unit: &str) -> String {
    fmt_fixed(hex_u128(s), decimals, unit)
}

fn group_int(n: u128) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub(crate) fn fmt_fixed(raw: u128, decimals: u8, unit: &str) -> String {
    if raw == 0 {
        return if unit.is_empty() {
            "0".into()
        } else {
            format!("0 {unit}")
        };
    }
    let base = 10u128.pow(decimals as u32);
    let whole = raw / base;
    let frac = raw % base;
    let w = group_int(whole);
    if frac == 0 {
        return if unit.is_empty() {
            w
        } else {
            format!("{w} {unit}")
        };
    }
    let mut f = format!("{frac:0width$}", width = decimals as usize);
    while f.ends_with('0') {
        f.pop();
    }
    if f.len() > 6 {
        f.truncate(6);
        while f.ends_with('0') {
            f.pop();
        }
    }
    if unit.is_empty() {
        format!("{w}.{f}")
    } else {
        format!("{w}.{f} {unit}")
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_block_number() {
        assert_eq!(classify("1882031"), Query::BlockNumber(1_882_031));
        assert_eq!(classify("blk 12"), Query::BlockNumber(12));
        assert_eq!(classify("block:99"), Query::BlockNumber(99));
    }

    #[test]
    fn classifies_address_and_hash() {
        let a = "0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168";
        match classify(a) {
            Query::Address(x) => assert_eq!(x.len(), 42),
            _ => panic!("addr"),
        }
        let h = "0x".to_string() + &"ab".repeat(32);
        match classify(&h) {
            Query::Hash(x) => assert_eq!(x.len(), 66),
            _ => panic!("hash"),
        }
        assert_eq!(classify("hello"), Query::Unknown);
    }

    #[test]
    fn pads_and_unpads() {
        let a = "0x5fc5360d0400a0fd4f2af552add042d716f1d168";
        let t = pad_topic(a);
        assert_eq!(t.len(), 66);
        assert_eq!(unpad_addr(&t), a);
    }

    #[test]
    fn formats_units() {
        assert_eq!(fmt_eth_hex("0x0"), "0 ETH");
        assert_eq!(fmt_eth_hex("0xde0b6b3a7640000"), "1 ETH");
        assert_eq!(fmt_token("0xf4240", 6, "USDG"), "1 USDG");
        assert_eq!(fmt_token("0x0", 6, "USDG"), "0 USDG");
        assert_eq!(fmt_raw_amount("0xde0b6b3a7640000"), "1000000000000000000");
        assert!(!fmt_raw_amount("0xde0b6b3a7640000").contains("ETH"));
        assert_eq!(fmt_fixed(1_000 * 10u128.pow(18), 18, "ETH"), "1,000 ETH");
    }

    #[test]
    fn method_of_native() {
        assert_eq!(method_name_val("0x", 0), "call");
        assert_eq!(method_name_val("0x", 1), "transfer");
        assert_eq!(method_name("0xa9059cbb0001"), "transfer");
        assert_eq!(method_name("0xdeadbeef"), "0xdeadbeef");
        assert_eq!(method_name("0x38ed1739"), "swapExactTokensForTokens");
        assert_eq!(method_name("0x414bf389"), "exactInputSingle");
        assert_eq!(method_name("0x5ae401dc"), "multicall");
    }

    #[test]
    fn scan_query_detect() {
        assert!(is_scan_query("0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168"));
        assert!(!is_scan_query("google"));
        assert!(is_scan_query("USDG"));
        assert!(is_scan_query("alice.hood"));
        assert!(is_scan_query("@alice.hood"));
    }

    #[test]
    fn pending_block_is_null() {
        assert_eq!(opt_u64(None), Value::Null);
        assert_eq!(opt_u64(Some(&Value::Null)), Value::Null);
        assert_eq!(opt_u64(Some(&json!("0x"))), Value::Null);
        assert_eq!(opt_u64(Some(&json!("0xa"))), json!(10));
        assert_eq!(opt_u64(Some(&json!("0x0"))), json!(0));
    }

    #[test]
    fn decode_transfer_usdg_not_eth_on_unknown() {
        let usdg_log = json!({
            "address": USDG,
            "topics": [
                TRANSFER_TOPIC,
                "0x0000000000000000000000001111111111111111111111111111111111111111",
                "0x0000000000000000000000002222222222222222222222222222222222222222"
            ],
            "data": "0x00000000000000000000000000000000000000000000000000000000000f4240",
            "logIndex": "0x1",
            "transactionHash": "0xabc"
        });
        let other = json!({
            "address": "0x0000000000000000000000000000000000000001",
            "topics": [
                TRANSFER_TOPIC,
                "0x0000000000000000000000001111111111111111111111111111111111111111",
                "0x0000000000000000000000002222222222222222222222222222222222222222"
            ],
            "data": "0x0000000000000000000000000000000000000000000000000de0b6b3a7640000",
            "logIndex": "0x2",
            "transactionHash": "0xdef"
        });
        let metas = HashMap::new();
        let u = decode_log(&usdg_log, &metas);
        assert_eq!(u["event"], "Transfer");
        assert!(u["amount"].as_str().unwrap().contains("USDG"));
        assert!(u["amount"].as_str().unwrap().starts_with("1 "), "{}", u["amount"]);
        assert!(u["topics"].as_array().unwrap().len() >= 3);
        assert_eq!(u["address"].as_str().unwrap().to_ascii_lowercase(), USDG.to_ascii_lowercase());
        let o = decode_log(&other, &metas);
        assert_eq!(o["event"], "Transfer");
        let amt = o["amount"].as_str().unwrap();
        assert!(!amt.contains("ETH"));
        assert_eq!(amt, "1000000000000000000");
    }

    #[test]
    fn decode_approval_and_plain_log() {
        let approval = json!({
            "address": USDG,
            "topics": [
                APPROVAL_TOPIC,
                "0x0000000000000000000000001111111111111111111111111111111111111111",
                "0x0000000000000000000000002222222222222222222222222222222222222222"
            ],
            "data": "0x00000000000000000000000000000000000000000000000000000000000f4240",
            "logIndex": "0x3",
            "transactionHash": "0xaaa"
        });
        let plain = json!({
            "address": "0x0000000000000000000000000000000000000002",
            "topics": ["0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"],
            "data": "0x01",
            "logIndex": "0x4",
            "transactionHash": "0xbbb"
        });
        let metas = HashMap::new();
        let a = decode_log(&approval, &metas);
        assert_eq!(a["event"], "Approval");
        assert_eq!(a["spender"], "0x2222222222222222222222222222222222222222");
        assert!(a.get("to").is_none());
        let p = decode_log(&plain, &metas);
        assert_eq!(p["event"], "log");
        assert!(p.get("from").is_none());
        assert!(p.get("to").is_none());
        assert_eq!(p["data"], "0x01");
        assert_eq!(p["index"], 4);
    }

    #[test]
    fn decode_nft_token_id() {
        let nft = json!({
            "address": "0x0000000000000000000000000000000000000003",
            "topics": [
                TRANSFER_TOPIC,
                "0x0000000000000000000000001111111111111111111111111111111111111111",
                "0x0000000000000000000000002222222222222222222222222222222222222222",
                "0x0000000000000000000000000000000000000000000000000000000000000007"
            ],
            "data": "0x",
            "logIndex": "0x0",
            "transactionHash": "0xccc"
        });
        let d = decode_log(&nft, &HashMap::new());
        assert_eq!(d["amount"], "#7");
        assert_eq!(d["nft"], true);
        assert_eq!(d["kind"], "ERC-721");
        assert!(!d["amount"].as_str().unwrap().contains("ETH"));
    }

    #[test]
    fn decode_input_transfer_words() {
        let to = "0000000000000000000000001111111111111111111111111111111111111111";
        let amt = "00000000000000000000000000000000000000000000000000000000000f4240";
        let input = format!("0xa9059cbb{to}{amt}");
        let d = decode_input(&input, "");
        assert_eq!(d["method"], "transfer");
        assert_eq!(d["selector"], "0xa9059cbb");
        assert_eq!(d["args"][0]["value"], "0x1111111111111111111111111111111111111111");
        assert_eq!(d["params"][0]["type"], "address");
        assert_eq!(d["words"].as_array().unwrap().len(), 2);
        let call = decode_input(
            "0xdeadbeef0000000000000000000000000000000000000000000000000000000000000001",
            "",
        );
        assert_eq!(call["method"], "0xdeadbeef");
        assert_eq!(call["words"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn index_page_false_has_no_host() {
        let v = index_page("transactions", Err("index transport: https://example.invalid".into()));
        assert_eq!(v["index"], false);
        assert_eq!(v["error"], "index wait");
        let s = v.to_string();
        assert!(!s.contains("http"));
        assert!(!s.to_ascii_lowercase().contains("blockscout"));
    }

    #[test]
    fn abi_string_decodes() {
        let bytes32 = "0x5553444700000000000000000000000000000000000000000000000000000000";
        assert_eq!(decode_abi_string(bytes32).as_deref(), Some("USDG"));
        let dyn_s = "0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000045553444700000000000000000000000000000000000000000000000000000000";
        assert_eq!(decode_abi_string(dyn_s).as_deref(), Some("USDG"));
    }

    #[test]
    fn path_kind_injects_ids() {
        let h = format!("0x{}", "ab".repeat(32));
        let (k, q) = parse_kind_query(&format!("tx/{h}"), "");
        assert_eq!(k, "tx");
        assert_eq!(q, format!("h={h}"));
        let (k, q) = parse_kind_query(&format!("tx?h={h}"), "");
        assert_eq!(k, "tx");
        assert_eq!(q, format!("h={h}"));
        let (k, q) = parse_kind_query("block/42", "page=2");
        assert_eq!(k, "block");
        assert!(q.contains("id=42"), "{q}");
        assert!(q.contains("page=2"), "{q}");
        let (k, q) = parse_kind_query("block/42/page/2", "");
        assert_eq!(k, "block");
        assert!(q.contains("id=42"), "{q}");
        assert!(q.contains("page=2"), "{q}");
        let (k, q) = parse_kind_query("txs/cursor/abc", "");
        assert_eq!(k, "txs");
        assert!(q.contains("cursor=abc"), "{q}");
        let a = "0x5fc5360d0400a0fd4f2af552add042d716f1d168";
        let (k, q) = parse_kind_query(&format!("addr/{a}/tab/events/cursor/n1"), "");
        assert_eq!(k, "addr");
        assert!(q.contains(&format!("a={a}")), "{q}");
        assert!(q.contains("tab=events"), "{q}");
        assert!(q.contains("cursor=n1"), "{q}");
        let a = "0x5fc5360d0400a0fd4f2af552add042d716f1d168";
        let (k, q) = parse_kind_query(&format!("addr/{a}"), "");
        assert_eq!(k, "addr");
        assert_eq!(q, format!("a={a}"));
        let s = api("tx", "");
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"], "missing hash");
    }

    #[test]
    fn tx_type_labels() {
        assert_eq!(tx_type_name(0), "legacy");
        assert_eq!(tx_type_name(2), "EIP-1559");
    }

    #[test]
    fn txs_include_string_hashes() {
        let h1 = format!("0x{}", "ab".repeat(32));
        let h2 = format!("0x{}", "cd".repeat(32));
        let raw = json!({
            "timestamp": "0x1",
            "transactions": [
                h1,
                {
                    "hash": h2,
                    "from": "0x1111111111111111111111111111111111111111",
                    "to": "0x2222222222222222222222222222222222222222",
                    "value": "0x0",
                    "input": "0x",
                    "nonce": "0x7",
                    "transactionIndex": "0x3",
                    "gas": "0x5208",
                    "gasPrice": "0x3b9aca00"
                }
            ]
        });
        let txs = txs_from_block_range(&raw, 9, 0, 10);
        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0]["hash"].as_str().unwrap().len(), 66);
        assert_eq!(txs[1]["from"].as_str().unwrap().len(), 42);
        assert!(!txs[1]["hash"].as_str().unwrap().is_empty());
        assert_eq!(txs[1]["nonce"], 7);
        assert_eq!(txs[1]["index"], 3);
        assert_eq!(txs[1]["gas"], 21000);
        assert_eq!(txs[1]["gas_price"], "1.00 GWEI");
        assert_eq!(txs[1]["block"], 9);
    }

    #[test]
    fn log_window_pages_back() {
        let (from, to, next) = log_window(5000, None);
        assert_eq!(to, 5000);
        assert_eq!(from, 5000 - LOG_SPAN);
        assert_eq!(next.unwrap()["to"], 5000 - LOG_SPAN - 1);
        let (from2, to2, next2) = log_window(5000, Some(r#"{"to":100}"#));
        assert_eq!(to2, 100);
        assert_eq!(from2, 0);
        assert!(next2.is_none());
        assert!(is_rpc_cursor(Some(r#"{"to":9}"#)));
        assert!(!is_rpc_cursor(Some(
            r#"{"block_number":9,"index":2,"items_count":50}"#
        )));
    }

    #[test]
    fn word_amt_uses_token_meta() {
        let w = "0x00000000000000000000000000000000000000000000000000000000000f4240";
        let usdg = word_amt(w, USDG);
        assert!(usdg.contains("USDG"), "{usdg}");
        assert!(usdg.starts_with("1 "), "{usdg}");
        if let Ok(mut g) = TOKEN_META.lock() {
            g.get_or_insert_with(HashMap::new).insert(
                "0x0000000000000000000000000000000000000001".into(),
                Some((6, "USDC".into())),
            );
        }
        let other = word_amt(w, "0x0000000000000000000000000000000000000001");
        assert!(other.contains("USDC"), "{other}");
        assert!(other.starts_with("1 "), "{other}");
        let input = format!(
            "0xa9059cbb0000000000000000000000001111111111111111111111111111111111111111{}",
            &w[2..]
        );
        let d = decode_input(&input, USDG);
        assert!(
            d["params"][1]["value"].as_str().unwrap().contains("USDG"),
            "{}",
            d["params"][1]["value"]
        );
    }

    #[test]
    fn pin_usdg_sorts_first() {
        let items = vec![
            json!({"address": "0x1", "symbol": "FOO", "usdg": false}),
            json!({"address": USDG, "symbol": "USDG", "usdg": true}),
        ];
        let pinned = pin_usdg_first(items, false);
        assert_eq!(pinned[0]["usdg"], true);
        let empty = pin_usdg_first(vec![], true);
        assert_eq!(empty[0]["usdg"], true);
        let h = empty[0].get("holders");
        assert!(
            h.is_none() || h == Some(&Value::Null),
            "pinned USDG must not invent a holder census, got {h:?}"
        );
    }

    #[test]
    fn l1_fee_from_orbit_receipt() {
        let r = json!({
            "l1BlockNumber": "0x4d2",
            "l1Fee": "0x38d7ea4c68000",
            "gasUsedForL1": "0x5208",
            "l1GasPrice": "0x3b9aca00"
        });
        assert!(l1_fee_from_receipt(&r).as_str().unwrap().contains("ETH"));
        assert_eq!(l1_gas_from_receipt(&r), json!(0x5208));
        assert!(l1_gas_price_from_receipt(&r).as_str().unwrap().contains("GWEI"));
        assert!(l1_fee_from_receipt(&json!({})).is_null());
    }

    #[test]
    fn merge_decoded_prefers_index_params() {
        let rpc = json!({
            "name": "0xdeadbeef",
            "method": "0xdeadbeef",
            "params": [],
            "args": []
        });
        let ix = json!({
            "name": "swap",
            "method": "swap",
            "params": [{"name": "amount", "type": "uint256", "value": "1000000"}],
            "args": [{"name": "amount", "type": "uint256", "value": "1000000"}]
        });
        let d = merge_decoded(rpc, Some(ix), USDG);
        assert_eq!(d["name"], "swap");
        assert!(d["params"][0]["value"].as_str().unwrap().contains("USDG"));
    }

    #[test]
    fn overlay_keeps_rpc_formatted_amount() {
        let mut logs = vec![json!({
            "index": 1,
            "address": USDG,
            "amount": "1 USDG",
            "decoded": {
                "name": "Transfer",
                "params": [
                    {"name": "from", "type": "address", "value": "0x1"},
                    {"name": "to", "type": "address", "value": "0x2"},
                    {"name": "amount", "type": "uint256", "value": "1 USDG"}
                ]
            }
        })];
        let ix = vec![json!({
            "index": 1,
            "amount": "1000000",
            "decoded": {
                "name": "Transfer",
                "params": [
                    {"name": "from", "type": "address", "value": "0x1"},
                    {"name": "to", "type": "address", "value": "0x2"},
                    {"name": "amount", "type": "uint256", "value": "1000000"}
                ]
            }
        })];
        overlay_decoded_logs(&mut logs, &ix);
        assert_eq!(logs[0]["amount"], "1 USDG");
        assert!(
            logs[0]["decoded"]["params"][2]["value"]
                .as_str()
                .unwrap()
                .contains("USDG"),
            "{}",
            logs[0]["decoded"]["params"][2]["value"]
        );
    }

    #[test]
    fn enrich_head_always_inserts_liq() {
        let out = enrich_head(json!({
            "ok": true,
            "blocks": [],
            "load": 0.0,
            "base_fee_n": 1.0,
            "gwei_n": 1.0,
            "gwei": "1",
            "base_fee": "1",
            "gas_used": 0,
            "block": 1,
            "tx_source": "rpc",
        }));
        let liq = out.get("liq").expect("head always includes liq");
        assert!(liq.is_object());
        assert_eq!(out.get("tx_source").and_then(|v| v.as_str()), Some("rpc"));
        if liq.get("loading").and_then(|v| v.as_bool()) == Some(true) {
            assert_eq!(liq.get("ok").and_then(|v| v.as_bool()), Some(false));
            assert_eq!(liq.get("tvl_usd").and_then(|v| v.as_f64()), Some(0.0));
            assert_eq!(liq.get("vol24_usd").and_then(|v| v.as_f64()), Some(0.0));
            assert_eq!(liq.get("pools").and_then(|v| v.as_u64()), Some(0));
            assert_eq!(liq.get("tokens").and_then(|v| v.as_u64()), Some(0));
        } else {
            assert!(liq.get("tvl_usd").is_some());
            assert!(liq.get("pools").is_some());
        }
    }

    #[test]
    fn tx_transfers_get_symbol_and_action() {
        let raw = vec![
            json!({
                "event": "Transfer",
                "token": USDG,
                "address": USDG,
                "amount": "1 USDG",
                "from": "0x1111111111111111111111111111111111111111",
                "to": "0x2222222222222222222222222222222222222222"
            }),
            json!({
                "event": "Transfer",
                "token": "0x0000000000000000000000000000000000000001",
                "address": "0x0000000000000000000000000000000000000001",
                "amount": "1000",
                "symbol": "FOO"
            }),
            json!({
                "event": "Transfer",
                "token": "0x0000000000000000000000000000000000000002",
                "address": "0x0000000000000000000000000000000000000002",
                "amount": "42"
            }),
        ];
        let enriched = enrich_tx_transfers(raw);
        assert_eq!(enriched[0]["symbol"], "USDG");
        assert_eq!(enriched[0]["usdg"], true);
        assert_eq!(enriched[1]["symbol"], "FOO");
        assert_eq!(enriched[1]["usdg"], false);
        assert_eq!(enriched[2]["symbol"], "");
        assert_eq!(enriched[2]["usdg"], false);

        assert_eq!(
            tx_action_summary(1, "0xde0b6b3a7640000", &[], "transfer"),
            "Transfer 1 ETH"
        );
        assert_eq!(
            tx_action_summary(0, "0x0", &enriched, "transfer"),
            "Transfer 1 USDG"
        );
        assert_eq!(
            tx_action_summary(0, "0x0", &enriched[1..], "approve"),
            "Transfer 1000 FOO"
        );
        assert_eq!(
            tx_action_summary(0, "0x0", &enriched[2..], "call"),
            "Transfer 42 0x0000000000000000000000000000000000000002"
        );
        assert_eq!(tx_action_summary(0, "0x0", &[], "swap"), "swap");
        assert_eq!(
            tx_action_summary(0, "0x0", &[], "Contract creation"),
            "Contract creation"
        );
    }

    #[test]
    fn labels_and_headline() {
        assert_eq!(addr_label(USDG), "USDG");
        assert_eq!(addr_label(WETH), "WETH");
        assert_eq!(addr_label("0x1111111111111111111111111111111111111111"), "");
        assert_eq!(
            headline_value(1, "0xde0b6b3a7640000", &[]),
            "1 ETH"
        );
        let xfers = vec![json!({"amount": "1 USDG", "symbol": "USDG"})];
        assert_eq!(headline_value(0, "0x0", &xfers), "1 USDG");
        assert_eq!(parse_qty("1,234.5 USDG"), 1234.5);
        assert_eq!(fmt_usd_f(1.2), "$1.20");
        let mut holdings = vec![json!({
            "token": USDG,
            "usdg": true,
            "amount": "2 USDG"
        })];
        price_holdings(&mut holdings);
        assert_eq!(holdings[0]["usd"], "$2.00");
    }

    #[test]
    fn explorer_list_tabs_do_not_block() {
        use std::time::Instant;
        for kind in ["head", "tokens", "txs", "blocks", "gas", "liq"] {
            let t0 = Instant::now();
            let body = super::api(kind, "");
            let ms = t0.elapsed().as_millis();
            eprintln!("scan {kind}: {ms}ms {}B", body.len());
            assert!(
                ms < 250,
                "scan {kind} took {ms}ms; WebView protocol thread would freeze the chrome. body={}",
                body.chars().take(160).collect::<String>()
            );
            let v: serde_json::Value =
                serde_json::from_str(&body).expect("scan tab must return json");
            assert!(v.is_object(), "{kind} not a json object");
        }
        for kind in ["search"] {
            let t0 = Instant::now();
            let body = super::api(kind, &format!("q={USDG}"));
            let ms = t0.elapsed().as_millis();
            eprintln!("scan {kind}: {ms}ms {}B", body.len());
            assert!(
                ms < 250,
                "scan {kind} took {ms}ms; search would freeze the chrome"
            );
            let v: serde_json::Value = serde_json::from_str(&body).expect("json");
            assert_eq!(v["kind"], "token", "{body}");
        }
        let t0 = Instant::now();
        let body = super::api("token", &format!("a={USDG}"));
        let ms = t0.elapsed().as_millis();
        eprintln!("scan token: {ms}ms {}B", body.len());
        assert!(
            ms < 250,
            "scan token took {ms}ms; opening a token from Tokens would freeze the chrome"
        );
        let v: serde_json::Value =
            serde_json::from_str(&body).expect("scan token must return json");
        let h = v.get("holders");
        let census_ok = match h {
            None => true,
            Some(Value::Null) => true,
            Some(x) => x.as_u64().map(|n| n > 1_000).unwrap_or(false),
        };
        assert!(
            census_ok,
            "USDG holders must be a real census or blank, never pool degree / page length, got {h:?}"
        );
        if let Some(n) = v.get("degree").and_then(|x| x.as_u64()) {
            assert_ne!(
                v.get("holders").and_then(|x| x.as_u64()),
                Some(n),
                "holders must not equal liquidity degree {n}"
            );
        }
    }
}
