//! History index over HTTP. Not the product. Scan UI and live RPC stay ours.
//! Transport hosts never appear in API error strings the chrome renders.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::{NATIVE_SYMBOL, USDG, USDG_DECIMALS};

const TTL_STATS: Duration = Duration::from_secs(12);
const TTL_LIST: Duration = Duration::from_secs(6);
const TTL_TOKEN: Duration = Duration::from_secs(20);
const TTL_FAIL: Duration = Duration::from_millis(2500);

static HTTP: Mutex<Option<reqwest::blocking::Client>> = Mutex::new(None);
static STATS: Mutex<Option<(Instant, Value)>> = Mutex::new(None);
static TXS: Mutex<Option<(Instant, Value)>> = Mutex::new(None);
static BLOCKS: Mutex<Option<(Instant, Value)>> = Mutex::new(None);
static TOKENS: Mutex<Option<(Instant, Value)>> = Mutex::new(None);
static TOKEN_DETAIL: Mutex<Option<HashMap<String, (Instant, Value)>>> = Mutex::new(None);
static TOKEN_INFLIGHT: Mutex<Option<HashSet<String>>> = Mutex::new(None);
static ADDR_BUNDLE: Mutex<Option<HashMap<String, (Instant, Value)>>> = Mutex::new(None);
static ADDR_INFLIGHT: Mutex<Option<HashSet<String>>> = Mutex::new(None);
static TX_OVERLAY: Mutex<Option<HashMap<String, (Instant, Value)>>> = Mutex::new(None);
static TX_INFLIGHT: Mutex<Option<HashSet<String>>> = Mutex::new(None);
static CONTRACTS: Mutex<Option<HashMap<String, (Instant, Value)>>> = Mutex::new(None);
static CONTRACT_INFLIGHT: Mutex<Option<HashSet<String>>> = Mutex::new(None);
static FAIL: Mutex<Option<(Instant, String)>> = Mutex::new(None);
static STATS_LOOP: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Default)]
pub struct Page {
    pub items: Vec<Value>,
    pub next: Option<Value>,
}

fn client() -> Result<reqwest::blocking::Client, String> {
    let mut g = HTTP.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_none() {
        *g = Some(
            reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(10))
                .pool_idle_timeout(Duration::from_secs(60))
                .http1_only()
                .user_agent("vapurr-scan/0.1")
                .build()
                .map_err(|_| "index wait")?,
        );
    }
    Ok(g.as_ref().unwrap().clone())
}

fn base() -> String {
    format!("{}/api/v2", crate::EXPLORER.trim_end_matches('/'))
}

/// Log the full transport chain. Return copy the UI can render — no host, no URL.
fn index_wait(e: reqwest::Error) -> String {
    eprintln!("index wait: {e}");
    let mut src = std::error::Error::source(&e);
    while let Some(x) = src {
        eprintln!("index wait: {x}");
        src = x.source();
    }
    "index wait".into()
}

pub fn get(path: &str) -> Result<Value, String> {
    if let Ok(g) = FAIL.lock() {
        if let Some((at, e)) = g.as_ref() {
            if at.elapsed() < TTL_FAIL {
                return Err(e.clone());
            }
        }
    }
    let url = format!("{}{}", base(), path);
    let resp = match client()?.get(&url).send() {
        Ok(r) => r,
        Err(e) => {
            let msg = index_wait(e);
            remember_fail(&msg);
            return Err(msg);
        }
    };
    if !resp.status().is_success() {
        eprintln!("index offline: {}", resp.status().as_u16());
        let msg = "index offline";
        if resp.status().is_server_error() || resp.status().as_u16() == 429 {
            remember_fail(msg);
        }
        return Err(msg.into());
    }
    match resp.json() {
        Ok(v) => {
            clear_fail();
            Ok(v)
        }
        Err(_) => {
            remember_fail("index wait");
            Err("index wait".into())
        }
    }
}

fn remember_fail(e: &str) {
    if let Ok(mut g) = FAIL.lock() {
        *g = Some((Instant::now(), e.to_string()));
    }
}

fn clear_fail() {
    if let Ok(mut g) = FAIL.lock() {
        *g = None;
    }
}

fn cached(slot: &Mutex<Option<(Instant, Value)>>, ttl: Duration, path: &str) -> Result<Value, String> {
    if let Ok(g) = slot.lock() {
        if let Some((at, v)) = g.as_ref() {
            if at.elapsed() < ttl {
                return Ok(v.clone());
            }
        }
    }
    match get(path) {
        Ok(v) => {
            if let Ok(mut g) = slot.lock() {
                *g = Some((Instant::now(), v.clone()));
            }
            Ok(v)
        }
        Err(e) => {
            if let Ok(g) = slot.lock() {
                if let Some((_, v)) = g.as_ref() {
                    return Ok(v.clone());
                }
            }
            Err(e)
        }
    }
}

/// Cached stats only. Never hits explorer HTTP.
pub fn stats_if_ready() -> Option<Value> {
    let g = STATS.lock().ok()?;
    g.as_ref().map(|(_, v)| v.clone())
}

pub fn kick() {
    if STATS_LOOP.swap(true, Ordering::SeqCst) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("scan-index".into())
        .spawn(|| loop {
            let _ = stats();
            let _ = latest_txs(None);
            let _ = latest_blocks(None);
            let _ = tokens(None);
            std::thread::sleep(TTL_STATS);
        });
    if spawned.is_err() {
        STATS_LOOP.store(false, Ordering::SeqCst);
    }
}

pub fn cursor_qs(cursor: Option<&str>) -> String {
    let Some(c) = cursor.map(str::trim).filter(|s| !s.is_empty()) else {
        return String::new();
    };
    if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(c) {
        let mut parts = Vec::new();
        for (k, val) in obj {
            if !k.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
                continue;
            }
            let s = match val {
                Value::Null => continue,
                Value::String(s) => urlenc(&s),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => continue,
            };
            parts.push(format!("{k}={s}"));
        }
        return parts.join("&");
    }
    if c.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '=' | '&' | '_' | '-' | '.' | '%'))
    {
        c.to_string()
    } else {
        String::new()
    }
}

pub fn with_cursor(path: &str, cursor: Option<&str>) -> String {
    let qs = cursor_qs(cursor);
    if qs.is_empty() {
        path.to_string()
    } else if path.contains('?') {
        format!("{path}&{qs}")
    } else {
        format!("{path}?{qs}")
    }
}

fn next_of(raw: &Value) -> Option<Value> {
    let v = raw.get("next_page_params")?;
    match v {
        Value::Object(o) if !o.is_empty() => Some(v.clone()),
        _ => None,
    }
}

fn page_of(raw: &Value, f: fn(&Value) -> Option<Value>) -> Page {
    Page {
        items: map_items(raw, f),
        next: next_of(raw),
    }
}

fn fetch_page(
    path: &str,
    cursor: Option<&str>,
    slot: Option<&Mutex<Option<(Instant, Value)>>>,
    ttl: Duration,
    f: fn(&Value) -> Option<Value>,
) -> Result<Page, String> {
    let path = with_cursor(path, cursor);
    let raw = if cursor.map(|c| !c.trim().is_empty()).unwrap_or(false) || slot.is_none() {
        get(&path)?
    } else {
        cached(slot.unwrap(), ttl, &path)?
    };
    Ok(page_of(&raw, f))
}

fn wrap_stats(raw: &Value) -> Value {
    json!({
        "ok": true,
        "total_blocks": num(raw, "total_blocks"),
        "total_txs": num(raw, "total_transactions"),
        "total_addresses": num(raw, "total_addresses"),
        "avg_ms": raw.get("average_block_time").and_then(|v| v.as_f64()).unwrap_or(0.0),
        "txs_today": num(raw, "transactions_today"),
        "gas_today": raw.get("gas_used_today").and_then(|v| v.as_str()).unwrap_or(""),
        "util": raw.get("network_utilization_percentage").and_then(|v| v.as_f64()).unwrap_or(0.0),
    })
}

pub fn stats() -> Result<Value, String> {
    if let Ok(g) = STATS.lock() {
        if let Some((at, v)) = g.as_ref() {
            if at.elapsed() < TTL_STATS {
                return Ok(v.clone());
            }
        }
    }
    match get("/stats") {
        Ok(raw) => {
            let out = wrap_stats(&raw);
            if let Ok(mut g) = STATS.lock() {
                *g = Some((Instant::now(), out.clone()));
            }
            Ok(out)
        }
        Err(e) => {
            if let Ok(g) = STATS.lock() {
                if let Some((_, v)) = g.as_ref() {
                    return Ok(v.clone());
                }
            }
            Err(e)
        }
    }
}

fn peek_page(
    slot: &Mutex<Option<(Instant, Value)>>,
    f: fn(&Value) -> Option<Value>,
) -> Option<Page> {
    let g = slot.lock().ok()?;
    let (_, v) = g.as_ref()?;
    Some(page_of(v, f))
}

pub fn latest_txs_if_ready() -> Option<Page> {
    peek_page(&TXS, map_tx)
}

pub fn latest_blocks_if_ready() -> Option<Page> {
    peek_page(&BLOCKS, map_block)
}

pub fn tokens_if_ready() -> Option<Page> {
    peek_page(&TOKENS, map_token)
}

fn token_key(addr: &str, xfer: Option<&str>, holders: Option<&str>) -> String {
    format!(
        "{}|{}|{}",
        addr.trim().to_ascii_lowercase(),
        xfer.unwrap_or("").trim(),
        holders.unwrap_or("").trim()
    )
}

fn remember_token_page(addr: &str, xfer: Option<&str>, holders: Option<&str>, v: &Value) {
    let key = token_key(addr, xfer, holders);
    let Ok(mut g) = TOKEN_DETAIL.lock() else {
        return;
    };
    let map = g.get_or_insert_with(HashMap::new);
    if map.len() > 64 {
        map.clear();
    }
    map.insert(key, (Instant::now(), v.clone()));
}

/// Cached `/tokens/{addr}` page. Never hits explorer HTTP.
pub fn token_if_ready(addr: &str, xfer: Option<&str>, holders: Option<&str>) -> Option<Value> {
    let key = token_key(addr, xfer, holders);
    let g = TOKEN_DETAIL.lock().ok()?;
    Some(g.as_ref()?.get(&key)?.1.clone())
}

/// Fill `token_if_ready` on a worker. Scan's protocol thread must not wait on HTTP.
pub fn kick_token(addr: &str, xfer: Option<&str>, holders: Option<&str>) {
    let key = token_key(addr, xfer, holders);
    if let Ok(g) = TOKEN_DETAIL.lock() {
        if let Some(map) = g.as_ref() {
            if let Some((at, _)) = map.get(&key) {
                if at.elapsed() < TTL_TOKEN {
                    return;
                }
            }
        }
    }
    {
        let Ok(mut g) = TOKEN_INFLIGHT.lock() else {
            return;
        };
        if !g.get_or_insert_with(HashSet::new).insert(key.clone()) {
            return;
        }
    }
    let addr = addr.to_string();
    let xfer = xfer.map(str::to_string);
    let holders = holders.map(str::to_string);
    let inflight = key.clone();
    let spawned = std::thread::Builder::new()
        .name("scan-token".into())
        .spawn(move || {
            let got = token(&addr, xfer.as_deref(), holders.as_deref());
            if let Ok(v) = got {
                remember_token_page(&addr, xfer.as_deref(), holders.as_deref(), &v);
            }
            if let Ok(mut g) = TOKEN_INFLIGHT.lock() {
                if let Some(set) = g.as_mut() {
                    set.remove(&inflight);
                }
            }
        });
    if spawned.is_err() {
        if let Ok(mut g) = TOKEN_INFLIGHT.lock() {
            if let Some(set) = g.as_mut() {
                set.remove(&key);
            }
        }
    }
}

fn cache_get(slot: &Mutex<Option<HashMap<String, (Instant, Value)>>>, key: &str) -> Option<Value> {
    let g = slot.lock().ok()?;
    Some(g.as_ref()?.get(key)?.1.clone())
}

fn cache_fresh(slot: &Mutex<Option<HashMap<String, (Instant, Value)>>>, key: &str, ttl: Duration) -> bool {
    if let Ok(g) = slot.lock() {
        if let Some(map) = g.as_ref() {
            if let Some((at, _)) = map.get(key) {
                return at.elapsed() < ttl;
            }
        }
    }
    false
}

fn cache_put(slot: &Mutex<Option<HashMap<String, (Instant, Value)>>>, key: String, v: Value) {
    let Ok(mut g) = slot.lock() else {
        return;
    };
    let map = g.get_or_insert_with(HashMap::new);
    if map.len() > 64 {
        map.clear();
    }
    map.insert(key, (Instant::now(), v));
}

fn begin_inflight(slot: &Mutex<Option<HashSet<String>>>, key: &str) -> bool {
    let Ok(mut g) = slot.lock() else {
        return false;
    };
    g.get_or_insert_with(HashSet::new).insert(key.to_string())
}

fn end_inflight(slot: &Mutex<Option<HashSet<String>>>, key: &str) {
    if let Ok(mut g) = slot.lock() {
        if let Some(set) = g.as_mut() {
            set.remove(key);
        }
    }
}

fn page_json(p: Page) -> Value {
    json!({ "items": p.items, "next": p.next })
}

fn addr_key(addr: &str, tab: Option<&str>, cursor: Option<&str>) -> String {
    format!(
        "{}|{}|{}",
        addr.trim().to_ascii_lowercase(),
        tab.unwrap_or("").trim(),
        cursor.unwrap_or("").trim()
    )
}

fn fetch_addr_bundle(addr: &str, tab: Option<&str>, cursor: Option<&str>) -> Value {
    let cur = |want: &str| {
        if tab == Some(want) {
            cursor
        } else {
            None
        }
    };
    let info = address_info(addr).ok();
    let txs = address_txs(addr, cur("txs")).ok().map(page_json);
    let xfers = address_transfers(addr, cur("xfers")).ok().map(page_json);
    let tokens = address_tokens(addr, cur("tokens")).ok().map(page_json);
    let internal = address_internal(addr, cur("internal")).ok().map(page_json);
    let events = address_logs(addr, cur("events")).ok().map(page_json);
    json!({
        "ok": true,
        "info": info,
        "txs": txs,
        "xfers": xfers,
        "tokens": tokens,
        "internal": internal,
        "events": events,
    })
}

pub fn addr_bundle_if_ready(addr: &str, tab: Option<&str>, cursor: Option<&str>) -> Option<Value> {
    cache_get(&ADDR_BUNDLE, &addr_key(addr, tab, cursor))
}

pub fn kick_addr(addr: &str, tab: Option<&str>, cursor: Option<&str>) {
    let key = addr_key(addr, tab, cursor);
    if cache_fresh(&ADDR_BUNDLE, &key, TTL_TOKEN) {
        return;
    }
    if !begin_inflight(&ADDR_INFLIGHT, &key) {
        return;
    }
    let addr = addr.to_string();
    let tab = tab.map(str::to_string);
    let cursor = cursor.map(str::to_string);
    let inflight = key.clone();
    let spawned = std::thread::Builder::new()
        .name("scan-addr".into())
        .spawn(move || {
            let v = fetch_addr_bundle(&addr, tab.as_deref(), cursor.as_deref());
            cache_put(&ADDR_BUNDLE, inflight.clone(), v);
            end_inflight(&ADDR_INFLIGHT, &inflight);
        });
    if spawned.is_err() {
        end_inflight(&ADDR_INFLIGHT, &key);
    }
}

fn fetch_tx_overlay(hash: &str) -> Value {
    let tx = transaction(hash).ok();
    let logs = transaction_logs(hash).ok();
    let internal = transaction_internal(hash).ok();
    json!({
        "ok": true,
        "tx": tx,
        "logs": logs.as_ref().map(|p| json!(p.items)),
        "internal": internal.as_ref().map(|p| json!(p.items)),
    })
}

pub fn tx_overlay_if_ready(hash: &str) -> Option<Value> {
    cache_get(&TX_OVERLAY, &hash.trim().to_ascii_lowercase())
}

pub fn kick_tx(hash: &str) {
    let key = hash.trim().to_ascii_lowercase();
    if cache_fresh(&TX_OVERLAY, &key, TTL_TOKEN) {
        return;
    }
    if !begin_inflight(&TX_INFLIGHT, &key) {
        return;
    }
    let inflight = key.clone();
    let spawned = std::thread::Builder::new()
        .name("scan-tx".into())
        .spawn(move || {
            let v = fetch_tx_overlay(&inflight);
            cache_put(&TX_OVERLAY, inflight.clone(), v);
            end_inflight(&TX_INFLIGHT, &inflight);
        });
    if spawned.is_err() {
        end_inflight(&TX_INFLIGHT, &key);
    }
}

pub fn contract_if_ready(addr: &str) -> Option<Value> {
    cache_get(&CONTRACTS, &addr.trim().to_ascii_lowercase())
}

pub fn kick_contract(addr: &str) {
    let key = addr.trim().to_ascii_lowercase();
    if cache_fresh(&CONTRACTS, &key, TTL_TOKEN) {
        return;
    }
    if !begin_inflight(&CONTRACT_INFLIGHT, &key) {
        return;
    }
    let inflight = key.clone();
    let spawned = std::thread::Builder::new()
        .name("scan-contract".into())
        .spawn(move || {
            if let Ok(v) = smart_contract(&inflight) {
                cache_put(&CONTRACTS, inflight.clone(), v);
            }
            end_inflight(&CONTRACT_INFLIGHT, &inflight);
        });
    if spawned.is_err() {
        end_inflight(&CONTRACT_INFLIGHT, &key);
    }
}

pub fn smart_contract(addr: &str) -> Result<Value, String> {
    match get(&format!("/smart-contracts/{addr}")) {
        Ok(raw) => Ok(map_contract(&raw)),
        Err(_) => Ok(json!({ "verified": false })),
    }
}

fn map_contract(raw: &Value) -> Value {
    let source = raw
        .get("source_code")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let verified = raw
        .get("is_verified")
        .and_then(|v| v.as_bool())
        .unwrap_or(!source.is_empty());
    let impl_addr = raw
        .get("implementations")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .map(|x| hash_or(hash_of(x.get("address_hash")), hash_of(x.get("address"))))
        .unwrap_or_default();
    json!({
        "verified": verified,
        "name": raw.get("name").and_then(|v| v.as_str()).unwrap_or(""),
        "compiler": raw.get("compiler_version").and_then(|v| v.as_str()).unwrap_or(""),
        "optimization": raw.get("optimization_enabled").and_then(|v| v.as_bool()).unwrap_or(false),
        "optimization_runs": parse_u64(raw.get("optimization_runs")),
        "language": raw.get("language").and_then(|v| v.as_str()).unwrap_or(""),
        "license": raw.get("license_type").and_then(|v| v.as_str()).unwrap_or(""),
        "proxy": raw.get("proxy_type").and_then(|v| v.as_str()).unwrap_or(""),
        "implementation": if impl_addr.is_empty() { Value::Null } else { json!(impl_addr) },
        "source": source,
        "abi": raw.get("abi").cloned().unwrap_or(Value::Null),
    })
}

pub fn latest_txs(cursor: Option<&str>) -> Result<Page, String> {
    fetch_page("/transactions", cursor, Some(&TXS), TTL_LIST, map_tx)
}

pub fn latest_blocks(cursor: Option<&str>) -> Result<Page, String> {
    fetch_page("/blocks?type=block", cursor, Some(&BLOCKS), TTL_LIST, map_block)
}

pub fn tokens(cursor: Option<&str>) -> Result<Page, String> {
    fetch_page(
        "/tokens?type=ERC-20",
        cursor,
        Some(&TOKENS),
        Duration::from_secs(20),
        map_token,
    )
}

pub fn search(q: &str) -> Result<Vec<Value>, String> {
    let raw = get(&format!("/search?q={}", urlenc(q)))?;
    Ok(map_items(&raw, map_search))
}

pub fn address_info(addr: &str) -> Result<Value, String> {
    let raw = get(&format!("/addresses/{}", addr))?;
    Ok(json!({
        "ok": true,
        "name": raw.get("name").and_then(|v| v.as_str()).unwrap_or(""),
        "verified": raw.get("is_verified").and_then(|v| v.as_bool()).unwrap_or(false),
        "contract": raw.get("is_contract").and_then(|v| v.as_bool()).unwrap_or(false),
        "ens": raw.get("ens_domain_name").and_then(|v| v.as_str()).unwrap_or(""),
        "creator": hash_of(raw.get("creator_address_hash")),
        "creation_tx": hash_of(raw.get("creation_transaction_hash")),
    }))
}

pub fn address_txs(addr: &str, cursor: Option<&str>) -> Result<Page, String> {
    fetch_page(
        &format!("/addresses/{}/transactions", addr),
        cursor,
        None,
        TTL_LIST,
        map_tx,
    )
}

pub fn address_transfers(addr: &str, cursor: Option<&str>) -> Result<Page, String> {
    fetch_page(
        &format!("/addresses/{}/token-transfers", addr),
        cursor,
        None,
        TTL_LIST,
        map_transfer,
    )
}

pub fn address_tokens(addr: &str, cursor: Option<&str>) -> Result<Page, String> {
    let path = with_cursor(&format!("/addresses/{}/token-balances", addr), cursor);
    let raw = get(&path)?;
    let arr = if let Some(a) = raw.as_array() {
        a.clone()
    } else {
        raw.get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };
    Ok(Page {
        items: arr.iter().filter_map(map_balance).collect(),
        next: next_of(&raw),
    })
}

pub fn address_internal(addr: &str, cursor: Option<&str>) -> Result<Page, String> {
    fetch_page(
        &format!("/addresses/{}/internal-transactions", addr),
        cursor,
        None,
        TTL_LIST,
        map_internal,
    )
}

pub fn address_logs(addr: &str, cursor: Option<&str>) -> Result<Page, String> {
    fetch_page(
        &format!("/addresses/{}/logs", addr),
        cursor,
        None,
        TTL_LIST,
        map_log,
    )
}

pub fn transaction(hash: &str) -> Result<Value, String> {
    let raw = get(&format!("/transactions/{hash}"))?;
    let mut t = map_tx(&raw).ok_or("tx not found")?;
    if let Some(obj) = t.as_object_mut() {
        obj.insert("ok".into(), json!(true));
        obj.insert("decoded".into(), map_decoded(raw.get("decoded_input")));
        if let Some(input) = raw
            .get("raw_input")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            obj.insert("input".into(), json!(input));
        }
        let gas_used = parse_u64(raw.get("gas_used"));
        if gas_used > 0 {
            obj.insert("gas_used".into(), json!(gas_used));
        }
        let revert = map_revert(raw.get("revert_reason"));
        if !revert.is_null() {
            obj.insert("revert".into(), revert);
        }
    }
    Ok(t)
}

pub fn transaction_logs(hash: &str) -> Result<Page, String> {
    fetch_page(
        &format!("/transactions/{hash}/logs"),
        None,
        None,
        TTL_LIST,
        map_log,
    )
}

pub fn transaction_internal(hash: &str) -> Result<Page, String> {
    fetch_page(
        &format!("/transactions/{hash}/internal-transactions"),
        None,
        None,
        TTL_LIST,
        map_internal,
    )
}

pub fn token(addr: &str, xfer_cursor: Option<&str>, holders_cursor: Option<&str>) -> Result<Value, String> {
    let raw = get(&format!("/tokens/{}", addr))?;
    let mut t = map_token(&raw).ok_or("token not found")?;
    if let Ok(xfers) = get(&with_cursor(&format!("/tokens/{}/transfers", addr), xfer_cursor)) {
        if let Some(obj) = t.as_object_mut() {
            obj.insert("transfers".into(), json!(map_items(&xfers, map_transfer)));
            obj.insert("transfers_next".into(), json!(next_of(&xfers)));
        }
    }
    if let Ok(h) = get(&with_cursor(&format!("/tokens/{}/holders", addr), holders_cursor)) {
        if let Some(obj) = t.as_object_mut() {
            let dec = obj.get("decimals").and_then(|x| x.as_u64()).unwrap_or(18).min(18) as u8;
            let sym = obj
                .get("symbol")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let token_addr = obj
                .get("address")
                .and_then(|x| x.as_str())
                .unwrap_or(addr)
                .to_string();
            let holders: Vec<Value> = h
                .get("items")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|row| map_holder_ctx(row, dec, &sym, &token_addr))
                .collect();
            obj.insert("holder_list".into(), json!(holders));
            obj.insert("holders_next".into(), json!(next_of(&h)));
        }
    }
    if let Some(obj) = t.as_object_mut() {
        obj.insert("ok".into(), json!(true));
        obj.insert("index".into(), json!(true));
        obj.insert("source".into(), json!("index"));
        obj.insert("loading".into(), json!(false));
        obj.insert("degraded".into(), json!(false));
    }
    remember_token_page(addr, xfer_cursor, holders_cursor, &t);
    Ok(t)
}

fn nonempty(s: &str) -> Option<&str> {
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn latest_txs_page(qs: &str) -> Result<Page, String> {
    latest_txs(nonempty(qs))
}

pub fn latest_blocks_page(qs: &str) -> Result<Page, String> {
    latest_blocks(nonempty(qs))
}

pub fn tokens_page(qs: &str) -> Result<Page, String> {
    tokens(nonempty(qs))
}

pub fn token_page(addr: &str, xfer_qs: &str, holder_qs: &str) -> Result<Value, String> {
    token(addr, nonempty(xfer_qs), nonempty(holder_qs))
}

pub fn address_txs_page(addr: &str, qs: &str) -> Result<Page, String> {
    address_txs(addr, nonempty(qs))
}

pub fn address_transfers_page(addr: &str, qs: &str) -> Result<Page, String> {
    address_transfers(addr, nonempty(qs))
}

pub fn token_holders(addr: &str, qs: &str) -> Result<Page, String> {
    let raw = get(&with_cursor(
        &format!("/tokens/{}/holders", addr),
        nonempty(qs),
    ))?;
    Ok(Page {
        items: map_items(&raw, map_holder),
        next: next_of(&raw),
    })
}

fn map_items(raw: &Value, f: fn(&Value) -> Option<Value>) -> Vec<Value> {
    raw.get("items")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(f)
        .collect()
}

fn map_tx(v: &Value) -> Option<Value> {
    let hash = hash_of(v.get("hash"));
    if hash.is_empty() {
        return None;
    }
    let to = hash_or(hash_of(v.get("to")), hash_of(v.get("created_contract")));
    let decoded = map_decoded(v.get("decoded_input"));
    let raw_method = v.get("method").and_then(|x| x.as_str()).unwrap_or("");
    let decoded_name = decoded
        .get("name")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let method = if !decoded_name.is_empty() && !decoded_name.starts_with("0x") {
        decoded_name.to_string()
    } else if !raw_method.is_empty() {
        raw_method.to_string()
    } else {
        "call".into()
    };
    let nonce = parse_u64_opt(v.get("nonce").unwrap_or(&Value::Null));
    let index = parse_u64_opt(v.get("position").unwrap_or(&Value::Null))
        .or_else(|| parse_u64_opt(v.get("transaction_index").unwrap_or(&Value::Null)));
    let gas = parse_u64(v.get("gas_limit").or_else(|| v.get("gas")));
    let gas_used = parse_u64(v.get("gas_used"));
    let gas_price = fmt_gwei_dec(v.get("gas_price"));
    Some(json!({
        "hash": hash,
        "from": hash_of(v.get("from")),
        "to": if to.is_empty() { Value::Null } else { json!(to) },
        "value": fmt_dec_wei(str_num(v.get("value")).as_str(), 18, NATIVE_SYMBOL),
        "method": method,
        "block": v
            .get("block")
            .and_then(|x| x.as_u64())
            .or_else(|| v.get("block_number").and_then(|x| x.as_u64()))
            .map(|n| json!(n))
            .unwrap_or(Value::Null),
        "ts": ts_of(v.get("timestamp")),
        "status": match v.get("status").and_then(|x| x.as_str()).unwrap_or("") {
            "ok" | "success" => 1,
            "error" | "reverted" => 0,
            _ => 2,
        },
        "fee": fmt_dec_wei(nested_str(v, &["fee", "value"]).as_str(), 18, NATIVE_SYMBOL),
        "nonce": nonce.map(|n| json!(n)).unwrap_or(Value::Null),
        "index": index.map(|n| json!(n)).unwrap_or(Value::Null),
        "gas": if gas > 0 { json!(gas) } else { Value::Null },
        "gas_used": if gas_used > 0 { json!(gas_used) } else { Value::Null },
        "gas_price": gas_price,
        "revert": map_revert(v.get("revert_reason")),
        "l1": l1_of(v),
        "l1_fee": l1_fee_of(v),
        "l1_gas": l1_gas_of(v),
        "l1_gas_price": l1_gas_price_of(v),
        "decoded": decoded,
    }))
}

fn map_block(v: &Value) -> Option<Value> {
    let number = v
        .get("height")
        .and_then(|x| x.as_u64())
        .or_else(|| v.get("block_number").and_then(|x| x.as_u64()))?;
    let gas_used = parse_u64(v.get("gas_used"));
    let gas_limit = parse_u64(v.get("gas_limit"));
    let load = if gas_limit > 0 {
        (gas_used as f64 / gas_limit as f64).clamp(0.0, 1.0)
    } else {
        v.get("gas_used_percentage")
            .and_then(|x| x.as_f64())
            .map(|p| (p / 100.0).clamp(0.0, 1.0))
            .unwrap_or(0.0)
    };
    let gas_limit = gas_limit.max(1);
    Some(json!({
        "number": number,
        "hash": hash_of(v.get("hash")),
        "ts": ts_of(v.get("timestamp")),
        "txs": v.get("transactions_count").and_then(|x| x.as_u64()).unwrap_or(0),
        "gas_used": gas_used,
        "gas_limit": gas_limit,
        "load": load,
        "l1": l1_of(v),
        "miner": hash_of(v.get("miner")),
        "full": false,
    }))
}

fn l1_of(v: &Value) -> Value {
    for key in ["l1_block_number", "l1BlockNumber"] {
        if let Some(n) = v.get(key).and_then(parse_u64_opt) {
            if n > 0 {
                return json!(n);
            }
        }
    }
    for nest in ["arbitrum", "optimism", "scroll"] {
        if let Some(n) = v
            .get(nest)
            .and_then(|o| o.get("l1_block_number"))
            .and_then(parse_u64_opt)
        {
            if n > 0 {
                return json!(n);
            }
        }
    }
    Value::Null
}

fn map_token(v: &Value) -> Option<Value> {
    let addr = hash_or(hash_of(v.get("address_hash")), hash_of(v.get("address")));
    if addr.is_empty() {
        return None;
    }
    let dec = parse_u64(v.get("decimals")).min(18) as u8;
    let supply = str_num(v.get("total_supply"));
    let sym = v.get("symbol").and_then(|x| x.as_str()).unwrap_or("");
    Some(json!({
        "address": addr,
        "name": v.get("name").and_then(|x| x.as_str()).unwrap_or(""),
        "symbol": sym,
        "type": v.get("type").and_then(|x| x.as_str()).unwrap_or("ERC-20"),
        "decimals": dec,
        "holders": holders_census(v),
        "supply": fmt_dec_wei(&supply, dec, sym),
        "verified": v.get("is_smart_contract_verified").and_then(|x| x.as_bool()).unwrap_or(false),
        "usdg": addr.eq_ignore_ascii_case(USDG),
        "price_usd": num_f64(v.get("exchange_rate")).or_else(|| num_f64(v.get("fiat_value"))),
        "mcap_usd": num_f64(v.get("circulating_market_cap")),
        "vol24_usd": num_f64(v.get("volume_24h")),
        "icon": v.get("icon_url").and_then(|x| x.as_str()).unwrap_or(""),
    }))
}

fn holders_census(v: &Value) -> Value {
    if let Some(n) = v.get("holders_count").and_then(parse_u64_opt) {
        return json!(n);
    }
    match v.get("holders") {
        Some(Value::Number(n)) => n.as_u64().map(|x| json!(x)).unwrap_or(Value::Null),
        Some(Value::String(s)) => s.parse::<u64>().ok().map(|x| json!(x)).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn num_f64(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64().filter(|x| x.is_finite() && *x > 0.0),
        Some(Value::String(s)) => s.parse::<f64>().ok().filter(|x| x.is_finite() && *x > 0.0),
        _ => None,
    }
}

fn map_transfer(v: &Value) -> Option<Value> {
    let token = v.get("token");
    let symbol = token
        .and_then(|t| t.get("symbol"))
        .and_then(|x| x.as_str())
        .unwrap_or("token");
    let dec = token
        .and_then(|t| t.get("decimals"))
        .and_then(parse_u64_opt)
        .unwrap_or(18)
        .min(18) as u8;
    let token_addr = hash_of(
        token
            .and_then(|t| t.get("address_hash"))
            .or_else(|| token.and_then(|t| t.get("address"))),
    );
    let total = v
        .get("total")
        .and_then(|t| t.get("value"))
        .and_then(|x| x.as_str())
        .unwrap_or("0");
    let token_id = v
        .get("total")
        .and_then(|t| t.get("token_id"))
        .and_then(|x| x.as_str())
        .or_else(|| v.get("token_id").and_then(|x| x.as_str()))
        .unwrap_or("");
    let ty = token
        .and_then(|t| t.get("type"))
        .and_then(|x| x.as_str())
        .unwrap_or("ERC-20");
    let nft = ty.to_ascii_uppercase().contains("721")
        || ty.to_ascii_uppercase().contains("1155")
        || !token_id.is_empty();
    let amount = if nft && !token_id.is_empty() {
        format!("#{token_id}")
    } else {
        fmt_dec_wei(total, dec, symbol)
    };
    Some(json!({
        "tx": hash_or(hash_of(v.get("transaction_hash")), hash_of(v.get("tx_hash"))),
        "block": v.get("block_number").and_then(|x| x.as_u64()).unwrap_or(0),
        "from": hash_of(v.get("from")),
        "to": hash_of(v.get("to")),
        "token": token_addr,
        "symbol": symbol,
        "usdg": token_addr.eq_ignore_ascii_case(USDG),
        "amount": amount,
        "nft": nft,
        "kind": if nft {
            if ty.is_empty() { "ERC-721" } else { ty }
        } else {
            "ERC-20"
        },
        "token_id": if token_id.is_empty() { Value::Null } else { json!(token_id) },
        "ts": ts_of(v.get("timestamp")),
        "event": "Transfer",
    }))
}

fn map_holder(v: &Value) -> Option<Value> {
    map_holder_ctx(v, 18, "", "")
}

fn map_holder_ctx(v: &Value, fallback_dec: u8, fallback_sym: &str, fallback_token: &str) -> Option<Value> {
    let addr = hash_or(hash_of(v.get("address")), hash_of(v.get("address_hash")));
    if addr.is_empty() {
        return None;
    }
    let token = v.get("token");
    let token_addr = token
        .map(|t| hash_or(hash_of(t.get("address_hash")), hash_of(t.get("address"))))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback_token.to_ascii_lowercase());
    let usdg = token_addr.eq_ignore_ascii_case(USDG) || fallback_token.eq_ignore_ascii_case(USDG);
    let symbol = token
        .and_then(|t| t.get("symbol"))
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(if usdg { "USDG" } else { fallback_sym });
    let dec = token
        .and_then(|t| t.get("decimals"))
        .and_then(parse_u64_opt)
        .map(|n| n.min(18) as u8)
        .unwrap_or(if usdg { USDG_DECIMALS } else { fallback_dec });
    let val = str_num(v.get("value"));
    let pct = v
        .get("percentage")
        .and_then(|x| x.as_f64())
        .or_else(|| {
            v.get("percentage")
                .and_then(|x| x.as_str())
                .and_then(|s| s.parse::<f64>().ok())
        });
    Some(json!({
        "address": addr,
        "amount": fmt_dec_wei(&val, dec, symbol),
        "value": val,
        "pct": pct.map(|n| json!(n)).unwrap_or(Value::Null),
        "usdg": usdg,
    }))
}

fn map_balance(v: &Value) -> Option<Value> {
    let token = v.get("token")?;
    let addr = hash_or(hash_of(token.get("address_hash")), hash_of(token.get("address")));
    let symbol = token.get("symbol").and_then(|x| x.as_str()).unwrap_or("");
    let dec = parse_u64(token.get("decimals")).min(18) as u8;
    let val = v.get("value").and_then(|x| x.as_str()).unwrap_or("0");
    Some(json!({
        "token": addr,
        "name": token.get("name").and_then(|x| x.as_str()).unwrap_or(""),
        "symbol": symbol,
        "usdg": addr.eq_ignore_ascii_case(USDG),
        "amount": fmt_dec_wei(val, dec, symbol),
    }))
}

fn map_internal(v: &Value) -> Option<Value> {
    let tx = hash_of(v.get("transaction_hash"));
    let ok = v.get("success").and_then(|x| x.as_bool()).unwrap_or(true);
    let err = v
        .get("error")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            map_revert(v.get("revert_reason"))
                .as_str()
                .map(|s| s.to_string())
        })
        .unwrap_or_default();
    Some(json!({
        "tx": tx,
        "hash": tx,
        "from": hash_of(v.get("from")),
        "to": hash_of(v.get("to")),
        "value": fmt_dec_wei(str_num(v.get("value")).as_str(), 18, NATIVE_SYMBOL),
        "type": v.get("type").and_then(|x| x.as_str()).unwrap_or("call"),
        "method": v
            .get("method")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| v.get("type").and_then(|x| x.as_str()).unwrap_or("call")),
        "block": v.get("block_number").and_then(|x| x.as_u64()).unwrap_or(0),
        "index": parse_u64(v.get("index")),
        "gas": parse_u64(v.get("gas_limit").or_else(|| v.get("gas"))),
        "gas_used": parse_u64(v.get("gas_used")),
        "error": if err.is_empty() { Value::Null } else { json!(err) },
        "ts": ts_of(v.get("timestamp")),
        "status": if ok { 1 } else { 0 },
    }))
}

fn map_log(v: &Value) -> Option<Value> {
    let tx = hash_or(
        hash_of(v.get("transaction_hash")),
        hash_of(v.get("tx_hash")),
    );
    let addr = hash_or(hash_of(v.get("address")), hash_of(v.get("address_hash")));
    let topics: Vec<String> = v
        .get("topics")
        .and_then(|x| x.as_array())
        .into_iter()
        .flatten()
        .filter_map(|t| t.as_str().map(|s| s.to_string()))
        .collect();
    let decoded = map_decoded(v.get("decoded"));
    let event = decoded
        .get("name")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let t0 = topics.first().map(|s| s.as_str()).unwrap_or("");
            if t0.eq_ignore_ascii_case(
                "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
            ) {
                "Transfer".into()
            } else if t0.eq_ignore_ascii_case(
                "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925",
            ) {
                "Approval".into()
            } else {
                "log".into()
            }
        });
    let mut out = json!({
        "tx": tx,
        "hash": tx,
        "block": v.get("block_number").and_then(|x| x.as_u64()).unwrap_or(0),
        "index": parse_u64(v.get("index").or_else(|| v.get("log_index"))),
        "address": addr,
        "token": addr,
        "usdg": addr.eq_ignore_ascii_case(USDG),
        "topics": topics,
        "data": v.get("data").and_then(|x| x.as_str()).unwrap_or("0x"),
        "event": event,
        "decoded": decoded,
    });
    flatten_decoded_log(&mut out);
    format_log_amounts(&mut out, v);
    Some(out)
}

fn map_decoded(v: Option<&Value>) -> Value {
    let Some(d) = v.filter(|x| x.is_object()) else {
        return Value::Null;
    };
    let method_call = d.get("method_call").and_then(|x| x.as_str()).unwrap_or("");
    let name = method_call
        .split('(')
        .next()
        .unwrap_or(method_call)
        .trim();
    let selector = d
        .get("method_id")
        .and_then(|x| x.as_str())
        .map(|s| {
            if s.starts_with("0x") || s.starts_with("0X") {
                s.to_ascii_lowercase()
            } else {
                format!("0x{}", s.to_ascii_lowercase())
            }
        })
        .unwrap_or_default();
    let params: Vec<Value> = d
        .get("parameters")
        .and_then(|x| x.as_array())
        .into_iter()
        .flatten()
        .map(|p| {
            let ty = p.get("type").and_then(|x| x.as_str()).unwrap_or("");
            json!({
                "name": p.get("name").and_then(|x| x.as_str()).unwrap_or(""),
                "type": ty,
                "kind": if ty == "address" { "address" } else { ty },
                "value": decoded_value(p.get("value").unwrap_or(&Value::Null)),
            })
        })
        .collect();
    if name.is_empty() && params.is_empty() {
        return Value::Null;
    }
    json!({
        "name": name,
        "method": name,
        "selector": selector,
        "params": params.clone(),
        "args": params,
    })
}

fn decoded_value(v: &Value) -> Value {
    if let Some(s) = v.as_str() {
        return json!(s);
    }
    if let Some(h) = v.get("hash").and_then(|x| x.as_str()) {
        return json!(h);
    }
    v.clone()
}

fn flatten_decoded_log(out: &mut Value) {
    let Some(params) = out
        .get("decoded")
        .and_then(|d| d.get("params"))
        .and_then(|v| v.as_array())
        .cloned()
    else {
        return;
    };
    let Some(obj) = out.as_object_mut() else {
        return;
    };
    for p in params {
        let name = p.get("name").and_then(|x| x.as_str()).unwrap_or("");
        let val = p.get("value").cloned().unwrap_or(Value::Null);
        match name {
            "from" | "owner" => {
                obj.entry("from").or_insert(val);
            }
            "to" => {
                obj.entry("to").or_insert(val);
            }
            "spender" => {
                obj.entry("spender").or_insert(val);
            }
            "amount" | "value" | "wad" | "assets" => {
                obj.entry("amount").or_insert(val);
            }
            _ => {}
        }
    }
}

fn format_log_amounts(out: &mut Value, raw: &Value) {
    let addr = out
        .get("address")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let usdg = out.get("usdg").and_then(|x| x.as_bool()).unwrap_or(false)
        || addr.eq_ignore_ascii_case(USDG);
    let (dec, unit) = if usdg {
        (USDG_DECIMALS, "USDG".to_string())
    } else {
        let dec = out
            .get("decimals")
            .and_then(parse_u64_opt)
            .or_else(|| raw.get("decimals").and_then(parse_u64_opt))
            .or_else(|| {
                raw.get("token")
                    .and_then(|t| t.get("decimals"))
                    .and_then(parse_u64_opt)
            })
            .unwrap_or(0)
            .min(18) as u8;
        if dec == 0 {
            return;
        }
        let unit = out
            .get("symbol")
            .and_then(|x| x.as_str())
            .or_else(|| raw.get("symbol").and_then(|x| x.as_str()))
            .or_else(|| {
                raw.get("token")
                    .and_then(|t| t.get("symbol"))
                    .and_then(|x| x.as_str())
            })
            .unwrap_or("")
            .to_string();
        (dec, unit)
    };
    if let Some(s) = raw_amount_digits(out.get("amount")) {
        if let Some(obj) = out.as_object_mut() {
            obj.insert("amount".into(), json!(fmt_dec_wei(&s, dec, &unit)));
        }
    }
    for key in ["params", "args"] {
        let path = format!("/decoded/{key}");
        let Some(arr) = out.pointer_mut(&path).and_then(|v| v.as_array_mut()) else {
            continue;
        };
        for p in arr {
            let name = p.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if !matches!(name.as_str(), "amount" | "value" | "wad" | "assets") {
                continue;
            }
            let Some(s) = raw_amount_digits(p.get("value")) else {
                continue;
            };
            if let Some(obj) = p.as_object_mut() {
                obj.insert("value".into(), json!(fmt_dec_wei(&s, dec, &unit)));
            }
        }
    }
}

fn raw_amount_digits(v: Option<&Value>) -> Option<String> {
    let s = match v? {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return None,
    };
    if s.contains(' ') || s.starts_with('#') {
        return None;
    }
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

fn l1_fee_of(v: &Value) -> Value {
    let s = pick_nested_num(v, &["l1_fee", "l1Fee"]);
    if s.is_empty() || s == "0" {
        return Value::Null;
    }
    json!(fmt_dec_wei(&s, 18, NATIVE_SYMBOL))
}

fn l1_gas_of(v: &Value) -> Value {
    let s = pick_nested_num(
        v,
        &[
            "l1_gas_used",
            "l1GasUsed",
            "gasUsedForL1",
            "gas_used_for_l1",
        ],
    );
    if s.is_empty() || s == "0" {
        return Value::Null;
    }
    s.parse::<u64>().map(|n| json!(n)).unwrap_or(json!(s))
}

fn l1_gas_price_of(v: &Value) -> Value {
    let s = pick_nested_num(v, &["l1_gas_price", "l1GasPrice"]);
    if s.is_empty() || s == "0" {
        return Value::Null;
    }
    if s.starts_with("0x") || s.starts_with("0X") {
        json!(crate::rpc::fmt_gwei(&json!(s)))
    } else {
        json!(fmt_dec_wei(&s, 9, "GWEI"))
    }
}

fn pick_nested_num(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        let s = str_num(v.get(*k));
        if !s.is_empty() && s != "0" {
            return s;
        }
        for nest in ["arbitrum", "optimism", "scroll"] {
            if let Some(n) = v.get(nest) {
                let s = str_num(n.get(*k));
                if !s.is_empty() && s != "0" {
                    return s;
                }
            }
        }
    }
    String::new()
}

fn map_search(v: &Value) -> Option<Value> {
    let kind = v
        .get("type")
        .or_else(|| v.get("item_type"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let addr = hash_or(hash_of(v.get("address_hash")), hash_of(v.get("address")));
    let tx = hash_or(hash_of(v.get("transaction_hash")), hash_of(v.get("tx_hash")));
    let block_hash = hash_of(v.get("block_hash"));
    let block_number = v
        .get("block_number")
        .and_then(|x| x.as_u64())
        .or_else(|| v.get("height").and_then(|x| x.as_u64()));
    let block = if !block_hash.is_empty() {
        json!(block_hash)
    } else if let Some(n) = block_number {
        json!(n)
    } else {
        Value::Null
    };
    Some(json!({
        "kind": kind,
        "name": v.get("name").and_then(|x| x.as_str()).unwrap_or(""),
        "symbol": v.get("symbol").and_then(|x| x.as_str()).unwrap_or(""),
        "address": addr,
        "tx": tx,
        "block": block,
        "block_number": block_number.map(|n| json!(n)).unwrap_or(Value::Null),
    }))
}

fn map_revert(v: Option<&Value>) -> Value {
    let Some(v) = v else {
        return Value::Null;
    };
    if v.is_null() {
        return Value::Null;
    }
    if let Some(s) = v.as_str() {
        let s = s.trim();
        if s.is_empty() {
            return Value::Null;
        }
        return json!(s);
    }
    if let Some(params) = v.get("parameters").and_then(|x| x.as_array()) {
        for p in params {
            if let Some(s) = p.get("value").and_then(|x| x.as_str()) {
                let s = s.trim();
                if !s.is_empty() && !s.eq_ignore_ascii_case("0x") {
                    return json!(s);
                }
            }
        }
    }
    if let Some(s) = v
        .get("method_call")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return json!(s);
    }
    Value::Null
}

fn fmt_gwei_dec(v: Option<&Value>) -> Value {
    let s = str_num(v);
    if s.is_empty() || s == "0" {
        return Value::Null;
    }
    if s.starts_with("0x") || s.starts_with("0X") {
        let g = crate::rpc::fmt_gwei(&json!(s));
        if g == "—" {
            Value::Null
        } else {
            json!(g)
        }
    } else {
        json!(fmt_dec_wei(&s, 9, "GWEI"))
    }
}

fn hash_of(v: Option<&Value>) -> String {
    let Some(v) = v else {
        return String::new();
    };
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    v.get("hash")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn hash_or(a: String, b: String) -> String {
    if a.is_empty() { b } else { a }
}

fn nested_str(v: &Value, path: &[&str]) -> String {
    let mut cur = v;
    for p in path {
        cur = match cur.get(*p) {
            Some(x) => x,
            None => return String::new(),
        };
    }
    cur.as_str().unwrap_or("").to_string()
}

fn str_num(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => "0".into(),
    }
}

fn num(v: &Value, key: &str) -> u64 {
    parse_u64(v.get(key))
}

fn parse_u64(v: Option<&Value>) -> u64 {
    parse_u64_opt(v.unwrap_or(&Value::Null)).unwrap_or(0)
}

fn parse_u64_opt(v: &Value) -> Option<u64> {
    if let Some(n) = v.as_u64() {
        return Some(n);
    }
    if let Some(n) = v.as_f64() {
        return Some(n as u64);
    }
    v.as_str().and_then(|s| s.parse().ok())
}

fn ts_of(v: Option<&Value>) -> u64 {
    let Some(v) = v else {
        return 0;
    };
    if let Some(n) = v.as_u64() {
        return n;
    }
    if let Some(n) = v.as_i64() {
        return n.max(0) as u64;
    }
    let Some(s) = v.as_str() else {
        return 0;
    };
    if let Ok(n) = s.parse::<u64>() {
        return n;
    }
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.timestamp().max(0) as u64)
        .unwrap_or(0)
}

fn fmt_dec_wei(s: &str, decimals: u8, unit: &str) -> String {
    let s = s.trim();
    if s.contains(' ') {
        return s.to_string();
    }
    if s.is_empty() || s == "0" {
        return fmt_unit("0", unit);
    }
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return fmt_unit("0", unit);
    }
    let digits = digits.trim_start_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let d = decimals as usize;
    let (whole, frac) = if d == 0 {
        (digits.to_string(), String::new())
    } else if digits.len() <= d {
        let mut f = "0".repeat(d - digits.len());
        f.push_str(digits);
        ("0".to_string(), f)
    } else {
        let split = digits.len() - d;
        (digits[..split].to_string(), digits[split..].to_string())
    };
    let w = group_int_str(&whole);
    if frac.chars().all(|c| c == '0') {
        return fmt_unit(&w, unit);
    }
    let mut f = frac;
    while f.ends_with('0') {
        f.pop();
    }
    if f.len() > 6 {
        f.truncate(6);
        while f.ends_with('0') {
            f.pop();
        }
    }
    if f.is_empty() {
        return fmt_unit(&w, unit);
    }
    if unit.is_empty() {
        format!("{w}.{f}")
    } else {
        format!("{w}.{f} {unit}")
    }
}

fn fmt_unit(n: &str, unit: &str) -> String {
    if unit.is_empty() {
        n.to_string()
    } else {
        format!("{n} {unit}")
    }
}

fn group_int_str(digits: &str) -> String {
    let s = digits.trim_start_matches('0');
    let s = if s.is_empty() { "0" } else { s };
    const MAX: usize = 24;
    if s.len() > MAX {
        let tail = &s[s.len() - 12..];
        return format!("…{}", group_commas(tail));
    }
    group_commas(s)
}

fn group_commas(s: &str) -> String {
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

fn urlenc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_tx_from_nested_from() {
        let v = json!({
            "hash": "0xabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabca",
            "from": { "hash": "0x1111111111111111111111111111111111111111" },
            "to": { "hash": "0x2222222222222222222222222222222222222222" },
            "value": "1000000000000000000",
            "method": "transfer",
            "block": 12,
            "status": "ok"
        });
        let t = map_tx(&v).unwrap();
        assert_eq!(t["from"], "0x1111111111111111111111111111111111111111");
        assert!(t["value"].as_str().unwrap().contains("1 ETH"));
    }

    #[test]
    fn live_index_optional() {
        match get("/stats") {
            Ok(v) => {
                assert!(
                    v.get("total_transactions").is_some() || v.get("average_block_time").is_some()
                );
            }
            Err(e) => {
                assert!(!e.to_ascii_lowercase().contains("blockscout"));
                assert!(!e.contains("https://"));
                eprintln!("index optional: {e}");
            }
        }
    }

    #[test]
    fn maps_usdg_token() {
        let v = json!({
            "address_hash": USDG,
            "name": "USDG",
            "symbol": "USDG",
            "decimals": "6",
            "type": "ERC-20",
            "holders_count": "9",
            "total_supply": "1000000"
        });
        let t = map_token(&v).unwrap();
        assert_eq!(t["usdg"], true);
        assert_eq!(t["symbol"], "USDG");
        assert_eq!(t["holders"], 9);
        let missing = json!({
            "address_hash": USDG,
            "name": "USDG",
            "symbol": "USDG",
            "decimals": "6",
            "type": "ERC-20"
        });
        assert!(map_token(&missing).unwrap()["holders"].is_null());
        let alt = json!({
            "address_hash": USDG,
            "name": "USDG",
            "symbol": "USDG",
            "decimals": "6",
            "type": "ERC-20",
            "holders": "218559",
            "exchange_rate": "1.0"
        });
        let t2 = map_token(&alt).unwrap();
        assert_eq!(t2["holders"], 218559);
        assert!((t2["price_usd"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn map_block_has_l1_and_load() {
        let v = json!({
            "height": 88,
            "hash": "0xabc",
            "timestamp": "2026-01-01T00:00:00Z",
            "transactions_count": 4,
            "gas_used": "500000",
            "gas_limit": "1000000",
            "l1_block_number": 42,
            "miner": { "hash": "0x1111111111111111111111111111111111111111" }
        });
        let b = map_block(&v).unwrap();
        assert_eq!(b["number"], 88);
        assert_eq!(b["l1"], 42);
        assert!((b["load"].as_f64().unwrap() - 0.5).abs() < 1e-9);
        assert_eq!(b["gas_used"], 500000);
        assert_eq!(b["gas_limit"], 1000000);
    }

    #[test]
    fn cursor_qs_from_json() {
        let q = cursor_qs(Some(r#"{"block_number":9,"index":2,"items_count":50}"#));
        assert!(q.contains("block_number=9"));
        assert!(q.contains("index=2"));
        assert!(q.contains("items_count=50"));
        assert!(!q.contains("http"));
        assert_eq!(cursor_qs(Some("")), "");
        assert_eq!(with_cursor("/transactions", Some(r#"{"index":1}"#)), "/transactions?index=1");
    }

    #[test]
    fn map_holder_keeps_address() {
        let v = json!({
            "address": { "hash": "0x1111111111111111111111111111111111111111" },
            "value": "1000000",
            "token": { "symbol": "USDG", "decimals": "6", "address_hash": USDG }
        });
        let h = map_holder(&v).unwrap();
        assert_eq!(h["address"], "0x1111111111111111111111111111111111111111");
        assert!(h["amount"].as_str().unwrap().contains("USDG"));
        assert_eq!(h["usdg"], true);
        let bare = json!({
            "address": { "hash": "0x1111111111111111111111111111111111111111" },
            "value": "1000000"
        });
        let h2 = map_holder_ctx(&bare, USDG_DECIMALS, "USDG", USDG).unwrap();
        assert!(h2["amount"].as_str().unwrap().contains("USDG"), "{}", h2["amount"]);
        assert_eq!(h2["usdg"], true);
    }

    #[test]
    fn page_keeps_next() {
        let raw = json!({
            "items": [],
            "next_page_params": { "block_number": 3, "index": 1 }
        });
        let p = page_of(&raw, map_tx);
        assert!(p.items.is_empty());
        assert_eq!(p.next.unwrap()["block_number"], 3);
    }

    #[test]
    fn search_hit_uses_block_number() {
        let v = json!({ "type": "block", "block_number": 42 });
        let s = map_search(&v).unwrap();
        assert_eq!(s["block"], 42);
        assert_eq!(s["block_number"], 42);
    }

    #[test]
    fn groups_large_supply() {
        let s = fmt_dec_wei("1234567000000", 6, "USDG");
        assert!(s.starts_with("1,234,567"), "{s}");
    }

    #[test]
    fn map_internal_ts_and_status() {
        let ok = json!({
            "transaction_hash": "0xabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabca",
            "from": { "hash": "0x1111111111111111111111111111111111111111" },
            "to": { "hash": "0x2222222222222222222222222222222222222222" },
            "value": "1000000000000000000",
            "type": "call",
            "block_number": 9,
            "success": true,
            "timestamp": "2026-01-01T00:00:00Z"
        });
        let t = map_internal(&ok).unwrap();
        assert_eq!(t["status"], 1);
        assert!(t["ts"].as_u64().unwrap() > 0);
        assert_eq!(t["tx"], t["hash"]);
        let bad = json!({
            "transaction_hash": "0xabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabca",
            "from": "0x1111111111111111111111111111111111111111",
            "to": "0x2222222222222222222222222222222222222222",
            "value": "0",
            "success": false,
            "timestamp": 1_704_067_200
        });
        let f = map_internal(&bad).unwrap();
        assert_eq!(f["status"], 0);
        assert_eq!(f["ts"], 1_704_067_200);
        assert!(f.get("success").is_none());
    }

    #[test]
    fn map_tx_carries_l1() {
        let v = json!({
            "hash": "0xabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabca",
            "from": "0x1111111111111111111111111111111111111111",
            "to": "0x2222222222222222222222222222222222222222",
            "value": "0",
            "status": "ok",
            "l1_block_number": 77,
            "l1_fee": "1000000000000000",
            "l1_gas_used": "2100",
            "l1_gas_price": "1000000000"
        });
        let t = map_tx(&v).unwrap();
        assert_eq!(t["l1"], 77);
        assert!(t["l1_fee"].as_str().unwrap().contains("ETH"));
        assert_eq!(t["l1_gas"], 2100);
        assert!(t["l1_gas_price"].as_str().unwrap().contains("GWEI"));
    }

    #[test]
    fn map_log_keeps_topics_and_decoded() {
        let v = json!({
            "transaction_hash": "0xabc",
            "index": 3,
            "block_number": 12,
            "address": { "hash": USDG },
            "data": "0x01",
            "topics": [
                "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                "0x0000000000000000000000001111111111111111111111111111111111111111"
            ],
            "decoded": {
                "method_call": "Transfer(address indexed from, address indexed to, uint256 value)",
                "method_id": "ddf252ad",
                "parameters": [
                    {"name": "from", "type": "address", "value": "0x1111111111111111111111111111111111111111"},
                    {"name": "to", "type": "address", "value": "0x2222222222222222222222222222222222222222"},
                    {"name": "value", "type": "uint256", "value": "1000000"}
                ]
            }
        });
        let l = map_log(&v).unwrap();
        assert_eq!(l["event"], "Transfer");
        assert_eq!(l["index"], 3);
        assert!(l["topics"].as_array().unwrap().len() >= 2);
        assert_eq!(l["data"], "0x01");
        assert_eq!(l["decoded"]["params"][0]["name"], "from");
        assert_eq!(l["from"], "0x1111111111111111111111111111111111111111");
        assert_eq!(l["to"], "0x2222222222222222222222222222222222222222");
        assert_eq!(l["amount"], "1 USDG");
        assert!(
            l["decoded"]["params"][2]["value"]
                .as_str()
                .unwrap()
                .contains("USDG"),
            "{}",
            l["decoded"]["params"][2]["value"]
        );
        assert_eq!(l["usdg"], true);
    }

    #[test]
    fn fmt_dec_wei_uint256_does_not_zero() {
        let s = format!("1{}", "0".repeat(70));
        let out = fmt_dec_wei(&s, 18, "TOKEN");
        assert!(!out.starts_with('0'), "{out}");
        assert!(out.contains('…') || out.contains("TOKEN"), "{out}");
        assert!(out.contains("TOKEN"), "{out}");
        assert_ne!(out, "0 TOKEN");
    }

    #[test]
    fn map_tx_keeps_nonce_gas_and_revert() {
        let v = json!({
            "hash": "0xabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabca",
            "from": "0x1111111111111111111111111111111111111111",
            "to": "0x2222222222222222222222222222222222222222",
            "value": "0",
            "status": "error",
            "method": "0xa9059cbb",
            "nonce": 7,
            "position": 3,
            "gas_limit": "21000",
            "gas_used": "21000",
            "gas_price": "1000000000",
            "revert_reason": { "parameters": [{ "value": "stale" }] },
            "decoded_input": { "method_call": "transfer(address,uint256)", "method_id": "a9059cbb" }
        });
        let t = map_tx(&v).unwrap();
        assert_eq!(t["nonce"], 7);
        assert_eq!(t["index"], 3);
        assert_eq!(t["gas"], 21000);
        assert_eq!(t["gas_used"], 21000);
        assert!(t["gas_price"].as_str().unwrap().contains("GWEI"));
        assert_eq!(t["revert"], "stale");
        assert_eq!(t["method"], "transfer");
        assert_eq!(t["status"], 0);
        assert_eq!(t["decoded"]["name"], "transfer");
    }

    #[test]
    fn map_revert_string_and_params() {
        assert_eq!(map_revert(Some(&json!("boom"))), json!("boom"));
        assert_eq!(
            map_revert(Some(&json!({"parameters":[{"value":"nope"}]}))),
            json!("nope")
        );
        assert!(map_revert(Some(&json!(""))).is_null());
        assert!(map_revert(None).is_null());
    }

    #[test]
    fn map_transfer_nft_token_id() {
        let v = json!({
            "transaction_hash": "0xabc",
            "from": "0x1111111111111111111111111111111111111111",
            "to": "0x2222222222222222222222222222222222222222",
            "token": {
                "address_hash": "0x0000000000000000000000000000000000000003",
                "symbol": "PUNK",
                "decimals": "0",
                "type": "ERC-721"
            },
            "total": { "value": "1", "token_id": "7" }
        });
        let t = map_transfer(&v).unwrap();
        assert_eq!(t["nft"], true);
        assert_eq!(t["amount"], "#7");
        assert_eq!(t["token_id"], "7");
        assert_eq!(t["kind"], "ERC-721");
    }

    #[test]
    fn map_holder_percentage() {
        let v = json!({
            "address": { "hash": "0x1111111111111111111111111111111111111111" },
            "value": "1000000",
            "percentage": "12.5",
            "token": { "symbol": "USDG", "decimals": "6", "address_hash": USDG }
        });
        let h = map_holder(&v).unwrap();
        assert!((h["pct"].as_f64().unwrap() - 12.5).abs() < 1e-9);
    }
}
