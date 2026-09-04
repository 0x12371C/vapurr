//! First-party Robinhood Chain trenches tape.
//!
//! Ingests robinhoodtrenches.com (`/api/tape|closed|traders|tokens|flow|radar`)
//! plus the keyless fomoapi.io pulse and the on-chain `/liq` book. Served at
//! `/trenches/api`. This is vapurr's JSON, not their HTML.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::desk;

const TRENCH: &str = "https://robinhoodtrenches.com";
const SCAN: &str = "https://robinhoodchain.blockscout.com";
const DEX: &str = "https://dexscreener.com/robinhood";
const FAMILY: &str = "https://fomo.family";
const CACHE: Duration = Duration::from_secs(8);
const TAPE_N: usize = 80;
const CLOSED_N: usize = 40;
const TRADERS_N: usize = 40;
const TOKENS_N: usize = 30;
const FLOW_N: usize = 20;
const RADAR_N: usize = 20;

#[derive(Clone)]
struct Book {
    at: Instant,
    window: String,
    stocks: bool,
    status: Value,
    overview: Value,
    tape: Vec<Value>,
    closed: Vec<Value>,
    traders: Vec<Value>,
    tokens: Vec<Value>,
    flow: Vec<Value>,
    radar: Vec<Value>,
}

static BOOKS: LazyLock<Mutex<HashMap<String, Book>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static LOOP: AtomicBool = AtomicBool::new(false);

pub fn warm() {
    kick();
}

fn kick() {
    if LOOP.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("trenches".into())
        .spawn(|| loop {
            let _ = load("24h", false);
            std::thread::sleep(CACHE);
        });
}

pub fn api_json(kind: &str, query: &str) -> String {
    let kind = kind.trim().trim_end_matches('/');
    if !matches!(
        kind,
        "" | "desk"
            | "snapshot"
            | "tape"
            | "closed"
            | "traders"
            | "tokens"
            | "flow"
            | "radar"
            | "status"
            | "overview"
    ) && !kind.starts_with("trader/")
    {
        return json!({ "ok": false, "error": "unknown" }).to_string();
    }
    kick();
    let window = window_of(query);
    let stocks = flag(query, "stocks");
    if let Some(handle) = kind.strip_prefix("trader/") {
        return trader_json(handle, &window, stocks).to_string();
    }
    let book = load(&window, stocks);
    let body = match kind {
        "" | "desk" | "snapshot" => snapshot(&book),
        "tape" => json!({
            "ok": book.status.get("ok").and_then(|x| x.as_bool()).unwrap_or(false),
            "source": "vapurr",
            "window": book.window,
            "stocks": book.stocks,
            "fills": take(&book.tape, limit(query, TAPE_N)),
        }),
        "closed" => json!({
            "ok": true,
            "source": "vapurr",
            "window": book.window,
            "trades": take(&book.closed, limit(query, CLOSED_N)),
        }),
        "traders" => json!({
            "ok": true,
            "source": "vapurr",
            "window": book.window,
            "traders": take(&book.traders, limit(query, TRADERS_N)),
        }),
        "tokens" => json!({
            "ok": true,
            "source": "vapurr",
            "window": book.window,
            "tokens": take(&book.tokens, limit(query, TOKENS_N)),
        }),
        "flow" => json!({
            "ok": true,
            "source": "vapurr",
            "window": book.window,
            "flow": take(&book.flow, limit(query, FLOW_N)),
        }),
        "radar" => json!({
            "ok": true,
            "source": "vapurr",
            "window": book.window,
            "pools": take(&book.radar, limit(query, RADAR_N)),
        }),
        "status" => json!({
            "ok": book.status.get("ok").and_then(|x| x.as_bool()).unwrap_or(false),
            "source": "vapurr",
            "status": book.status,
        }),
        "overview" => json!({
            "ok": true,
            "source": "vapurr",
            "window": book.window,
            "overview": book.overview,
        }),
        _ => json!({ "ok": false, "error": "unknown" }),
    };
    body.to_string()
}

fn snapshot(book: &Book) -> Value {
    let d = desk();
    let liq = vapurr_rhc::liq::snapshot();
    let liq_ok = liq.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
    let pairs = liq
        .get("pools")
        .and_then(|x| x.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    json!({
        "ok": book.status.get("ok").and_then(|x| x.as_bool()).unwrap_or(false),
        "source": "vapurr",
        "ingest": ["robinhoodtrenches", "fomoapi", "rhc-liq"],
        "chain": "robinhood",
        "chain_id": vapurr_rhc::CHAIN_ID,
        "window": book.window,
        "stocks": book.stocks,
        "status": book.status,
        "overview": book.overview,
        "tape": take(&book.tape, TAPE_N),
        "closed": take(&book.closed, CLOSED_N),
        "traders": take(&book.traders, TRADERS_N),
        "tokens": take(&book.tokens, TOKENS_N),
        "flow": take(&book.flow, FLOW_N),
        "radar": take(&book.radar, RADAR_N),
        "pulse": d.feed.into_iter().take(16).collect::<Vec<_>>(),
        "fomo": {
            "ok": d.ok,
            "source": d.source,
            "trending": d.trending.len(),
        },
        "liq": { "ok": liq_ok, "pairs": pairs },
    })
}

fn load(window: &str, stocks: bool) -> Book {
    let key = cache_key(window, stocks);
    if let Ok(g) = BOOKS.lock() {
        if let Some(b) = g.get(&key) {
            if b.at.elapsed() < CACHE {
                return b.clone();
            }
        }
    }
    let book = fetch_book(window, stocks);
    if let Ok(mut g) = BOOKS.lock() {
        g.insert(key, book.clone());
        if g.len() > 8 {
            g.retain(|_, b| b.at.elapsed() < Duration::from_secs(30));
        }
    }
    book
}

fn fetch_book(window: &str, stocks: bool) -> Book {
    let Some(http) = client() else {
        return empty_book(window, stocks);
    };
    let w = window.to_string();
    let st = stocks;
    let (status, overview, tape, closed, traders, tokens, flow, radar) = std::thread::scope(|s| {
        let status = s.spawn(|| pull(&http, "/status", &w, st));
        let overview = s.spawn(|| pull(&http, "/overview", &w, st));
        let tape = s.spawn(|| pull(&http, "/tape", &w, st));
        let closed = s.spawn(|| pull(&http, "/closed", &w, st));
        let traders = s.spawn(|| pull(&http, "/traders", &w, st));
        let tokens = s.spawn(|| pull(&http, "/tokens", &w, st));
        let flow = s.spawn(|| pull(&http, "/flow", &w, st));
        let radar = s.spawn(|| pull(&http, "/radar", &w, st));
        (
            status.join().ok().flatten(),
            overview.join().ok().flatten(),
            tape.join().ok().flatten(),
            closed.join().ok().flatten(),
            traders.join().ok().flatten(),
            tokens.join().ok().flatten(),
            flow.join().ok().flatten(),
            radar.join().ok().flatten(),
        )
    });
    let liq = rhc_px();
    Book {
        at: Instant::now(),
        window: window.into(),
        stocks,
        status: status
            .as_ref()
            .map(map_status)
            .unwrap_or_else(|| json!({ "ok": false })),
        overview: overview.as_ref().map(map_overview).unwrap_or(Value::Null),
        tape: rows(&tape).into_iter().filter_map(|v| map_fill(&v)).collect(),
        closed: rows(&closed)
            .into_iter()
            .filter_map(|v| map_closed(&v))
            .collect(),
        traders: rows(&traders)
            .into_iter()
            .filter_map(|v| map_trader(&v))
            .collect(),
        tokens: rows(&tokens)
            .into_iter()
            .filter_map(|v| map_token(&v, &liq))
            .collect(),
        flow: rows(&flow).into_iter().filter_map(|v| map_flow(&v)).collect(),
        radar: rows(&radar)
            .into_iter()
            .filter_map(|v| map_radar(&v))
            .collect(),
    }
}

fn empty_book(window: &str, stocks: bool) -> Book {
    Book {
        at: Instant::now(),
        window: window.into(),
        stocks,
        status: json!({ "ok": false }),
        overview: Value::Null,
        tape: vec![],
        closed: vec![],
        traders: vec![],
        tokens: vec![],
        flow: vec![],
        radar: vec![],
    }
}

fn trader_json(handle: &str, window: &str, stocks: bool) -> Value {
    let handle = handle.trim();
    if handle.is_empty() || handle.len() > 64 {
        return json!({ "ok": false, "error": "handle" });
    }
    let Some(http) = client() else {
        return json!({ "ok": false, "error": "http" });
    };
    let path = format!("/trader/{}", urlencoding_lite(handle));
    match pull(&http, &path, window, stocks) {
        Some(v) => json!({
            "ok": true,
            "source": "vapurr",
            "window": window,
            "trader": v,
        }),
        None => json!({ "ok": false, "error": "no trader" }),
    }
}

fn client() -> Option<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("vapurr/1.1")
        .build()
        .ok()
}

fn pull(http: &reqwest::blocking::Client, path: &str, window: &str, stocks: bool) -> Option<Value> {
    let url = format!(
        "{TRENCH}/api{path}?window={window}&stocks={}",
        if stocks { "true" } else { "false" }
    );
    http.get(url).send().ok()?.json().ok()
}

fn rows(v: &Option<Value>) -> Vec<Value> {
    match v {
        Some(Value::Array(a)) => a.clone(),
        Some(Value::Object(o)) => o
            .get("fills")
            .or_else(|| o.get("trades"))
            .or_else(|| o.get("traders"))
            .or_else(|| o.get("tokens"))
            .or_else(|| o.get("flow"))
            .or_else(|| o.get("pools"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => vec![],
    }
}

fn map_status(v: &Value) -> Value {
    json!({
        "ok": v.get("ok").and_then(|x| x.as_bool()).unwrap_or(true),
        "chain": v.get("chain").and_then(|x| x.as_str()).unwrap_or("robinhood"),
        "chain_id": v.get("chain_id").and_then(|x| x.as_u64()).unwrap_or(vapurr_rhc::CHAIN_ID),
        "wallets": v.get("wallets").and_then(|x| x.as_u64()).unwrap_or(0),
        "fills": v.get("trades").and_then(|x| x.as_u64()).unwrap_or(0),
        "block": v.get("last_block").and_then(|x| x.as_u64()).unwrap_or(0),
        "lag": v.get("lag_seconds").cloned().unwrap_or(Value::Null),
        "viewers": v.get("viewers").and_then(|x| x.as_u64()).unwrap_or(0),
        "feed": v.get("source").and_then(|x| x.as_str()).unwrap_or("websocket"),
        "ws": format!("{TRENCH}/ws"),
    })
}

fn map_overview(v: &Value) -> Value {
    json!({
        "window": v.get("window").cloned().unwrap_or(json!("24h")),
        "fills": v.get("fills"),
        "buys": v.get("buys"),
        "sells": v.get("sells"),
        "volume": v.get("volume"),
        "traders": v.get("active_traders"),
        "tokens": v.get("tokens"),
        "realized": v.get("realized_pnl"),
        "unrealized": v.get("unrealized_pnl"),
        "net": v.get("net_pnl"),
        "win_rate": v.get("win_rate"),
        "closed": v.get("closed_trades"),
        "open_bags": v.get("open_bags"),
        "last_5m": v.get("last_5m"),
        "biggest_win": v.get("biggest_win"),
        "biggest_buy": v.get("biggest_buy"),
    })
}

fn map_fill(v: &Value) -> Option<Value> {
    let token = s(v, "token")?;
    let handle = s(v, "handle").unwrap_or_else(|| "anon".into());
    let tx = s(v, "tx").unwrap_or_default();
    let pair = s(v, "pair_url").unwrap_or_else(|| format!("{DEX}/{token}"));
    Some(json!({
        "id": v.get("id"),
        "ts": v.get("ts"),
        "side": s(v, "side").unwrap_or_else(|| "buy".into()),
        "usd": v.get("usd"),
        "amount": v.get("amount"),
        "price": v.get("price"),
        "symbol": s(v, "symbol").unwrap_or_else(|| "—".into()),
        "token": token,
        "handle": handle,
        "wallet": s(v, "wallet"),
        "followers": v.get("followers"),
        "tx": tx,
        "block": v.get("block"),
        "stock": v.get("is_stock").and_then(|x| x.as_u64()).unwrap_or(0) == 1
            || v.get("is_stock").and_then(|x| x.as_bool()).unwrap_or(false),
        "flags": v.get("flags").cloned().unwrap_or(json!([])),
        "href": {
            "fomo": format!("{FAMILY}/profile/{handle}"),
            "dex": pair,
            "tx": if tx.is_empty() { Value::Null } else { json!(format!("{SCAN}/tx/{tx}")) },
            "token": format!("{SCAN}/token/{token}"),
        }
    }))
}

fn map_closed(v: &Value) -> Option<Value> {
    let token = s(v, "token")?;
    let handle = s(v, "handle").unwrap_or_else(|| "anon".into());
    Some(json!({
        "handle": handle,
        "wallet": s(v, "wallet"),
        "token": token,
        "symbol": s(v, "symbol").unwrap_or_else(|| "—".into()),
        "stock": flag_stock(v),
        "opened": v.get("opened_ts"),
        "closed": v.get("closed_ts"),
        "paid": v.get("cost_sold"),
        "got": v.get("proceeds_usd"),
        "pnl": v.get("pnl_usd"),
        "pct": v.get("pnl_pct"),
        "held": v.get("hold_seconds"),
        "buys": v.get("buys"),
        "sells": v.get("sells"),
        "followers": v.get("followers"),
        "href": {
            "fomo": format!("{FAMILY}/profile/{handle}"),
            "dex": format!("{DEX}/{token}"),
        }
    }))
}

fn map_trader(v: &Value) -> Option<Value> {
    let handle = s(v, "handle")?;
    let wallet = s(v, "address").or_else(|| s(v, "wallet")).unwrap_or_default();
    Some(json!({
        "handle": handle,
        "name": s(v, "display_name").unwrap_or_else(|| handle.clone()),
        "wallet": wallet,
        "followers": v.get("followers"),
        "fills": v.get("fills"),
        "buys": v.get("buys"),
        "sells": v.get("sells"),
        "volume": v.get("volume"),
        "realized": v.get("realized_pnl"),
        "unrealized": v.get("unrealized_pnl"),
        "net": v.get("net_pnl"),
        "win_rate": v.get("win_rate"),
        "closed": v.get("closed_trades"),
        "wins": v.get("wins"),
        "best": v.get("best_trade"),
        "worst": v.get("worst_trade"),
        "open_bags": v.get("open_bags"),
        "active": v.get("active"),
        "href": {
            "fomo": format!("{FAMILY}/profile/{handle}"),
            "scan": if wallet.is_empty() { Value::Null } else { json!(format!("{SCAN}/address/{wallet}")) },
        }
    }))
}

fn map_token(v: &Value, liq: &HashMap<String, f64>) -> Option<Value> {
    let token = s(v, "token")?;
    let px = liq.get(&token.to_ascii_lowercase()).copied();
    Some(json!({
        "token": token,
        "symbol": s(v, "symbol").unwrap_or_else(|| "—".into()),
        "name": s(v, "name").unwrap_or_default(),
        "stock": flag_stock(v),
        "buyers": v.get("buyers"),
        "holders": v.get("holders"),
        "usd_in": v.get("usd_in"),
        "usd_out": v.get("usd_out"),
        "net": v.get("net_usd"),
        "mark": v.get("mark").cloned().or_else(|| px.map(|p| json!(p))),
        "liq": v.get("liquidity"),
        "change24": v.get("change24"),
        "since_first": v.get("since_first_buy_pct"),
        "first_buyer": v.get("first_buyer"),
        "pair": v.get("pair_url"),
        "href": {
            "dex": v.get("pair_url").and_then(|x| x.as_str()).map(|s| s.to_string())
                .unwrap_or_else(|| format!("{DEX}/{token}")),
            "token": format!("{SCAN}/token/{token}"),
            "fomo": format!("{FAMILY}/token/{token}"),
        }
    }))
}

fn map_flow(v: &Value) -> Option<Value> {
    let token = s(v, "token")?;
    Some(json!({
        "token": token,
        "symbol": s(v, "symbol").unwrap_or_else(|| "—".into()),
        "lead": v.get("lead"),
        "followers": v.get("followers"),
        "href": { "dex": format!("{DEX}/{token}") },
    }))
}

fn map_radar(v: &Value) -> Option<Value> {
    let token = s(v, "token")?;
    Some(json!({
        "token": token,
        "symbol": s(v, "symbol").unwrap_or_else(|| "—".into()),
        "name": s(v, "name").unwrap_or_default(),
        "stock": flag_stock(v),
        "fresh": v.get("fresh").and_then(|x| x.as_bool()).unwrap_or(true),
        "buyers": v.get("buyers"),
        "usd_in": v.get("usd_in"),
        "first_ts": v.get("first_ts"),
        "first_buyer": v.get("first_buyer"),
        "href": {
            "dex": format!("{DEX}/{token}"),
            "token": format!("{SCAN}/token/{token}"),
        }
    }))
}

fn rhc_px() -> HashMap<String, f64> {
    let snap = vapurr_rhc::liq::snapshot();
    let mut out = HashMap::new();
    let Some(arr) = snap.get("tokens").and_then(|x| x.as_array()) else {
        return out;
    };
    for t in arr {
        let addr = t
            .get("address")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if let Some(px) = t.get("price_usd").and_then(|x| x.as_f64()) {
            if px > 0.0 {
                out.insert(addr, px);
            }
        }
    }
    out
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k)
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn flag_stock(v: &Value) -> bool {
    v.get("is_stock").and_then(|x| x.as_u64()).unwrap_or(0) == 1
        || v.get("is_stock").and_then(|x| x.as_bool()).unwrap_or(false)
}

fn take(rows: &[Value], n: usize) -> Vec<Value> {
    rows.iter().take(n).cloned().collect()
}

fn cache_key(window: &str, stocks: bool) -> String {
    format!("{window}|{stocks}")
}

fn window_of(query: &str) -> String {
    match param(query, "window").as_deref() {
        Some("1h") | Some("1") => "1h".into(),
        Some("7d") | Some("3") => "7d".into(),
        Some("30d") | Some("4") => "30d".into(),
        Some("all") | Some("5") => "all".into(),
        _ => "24h".into(),
    }
}

fn flag(query: &str, k: &str) -> bool {
    matches!(
        param(query, k).as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

fn limit(query: &str, cap: usize) -> usize {
    param(query, "limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(cap)
        .clamp(1, cap)
}

fn param(query: &str, key: &str) -> Option<String> {
    for part in query.split('&') {
        let (k, v) = part.split_once('=').unwrap_or((part, ""));
        if k == key && !v.is_empty() {
            return Some(v.to_string());
        }
    }
    None
}

fn urlencoding_lite(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_are_pinned() {
        assert_eq!(window_of("window=1h"), "1h");
        assert_eq!(window_of("window=7d&stocks=true"), "7d");
        assert_eq!(window_of(""), "24h");
        assert_eq!(window_of("window=nope"), "24h");
    }

    #[test]
    fn maps_a_live_fill() {
        let v = json!({
            "id": 1,
            "ts": 1788543944,
            "tx": "0xabc",
            "side": "buy",
            "usd": 9977.56,
            "amount": 192.6,
            "price": 0.051,
            "handle": "stigstigstig_",
            "wallet": "0x1111111111111111111111111111111111111111",
            "followers": 23400,
            "token": "0x385f4f8ae47651ce5f58f5265395a669f8281e18",
            "symbol": "MEME",
            "is_stock": 0,
            "block": 54437904,
            "flags": []
        });
        let f = map_fill(&v).unwrap();
        assert_eq!(f["side"], "buy");
        assert_eq!(f["symbol"], "MEME");
        assert_eq!(f["stock"], false);
        assert!(f["href"]["fomo"].as_str().unwrap().contains("stigstigstig_"));
        assert!(f["href"]["tx"].as_str().unwrap().contains("0xabc"));
    }

    #[test]
    fn maps_a_trader() {
        let v = json!({
            "address": "0x0a6ebed0155edb4b21d92ad02897a626cd90119e",
            "handle": "unipcs",
            "display_name": "Unipcs",
            "followers": 463713,
            "volume": 34583.4,
            "fills": 198,
            "realized_pnl": 220.9,
            "unrealized_pnl": -2481.6,
            "net_pnl": -2260.7,
            "win_rate": 0.66,
            "closed_trades": 3,
            "active": true
        });
        let t = map_trader(&v).unwrap();
        assert_eq!(t["handle"], "unipcs");
        assert_eq!(t["wallet"], "0x0a6ebed0155edb4b21d92ad02897a626cd90119e");
        assert_eq!(t["fills"], 198);
    }

    #[test]
    fn unknown_kind_is_not_ok() {
        let v: Value = serde_json::from_str(&api_json("nope", "")).unwrap();
        assert_eq!(v["ok"], false);
    }

    #[test]
    fn live_trenches_optional() {
        let Some(http) = client() else {
            return;
        };
        let Some(v) = pull(&http, "/status", "24h", false) else {
            return;
        };
        assert_eq!(v["chain"], "robinhood");
        assert!(v.get("wallets").and_then(|x| x.as_u64()).unwrap_or(0) > 0);
        let s = api_json("status", "window=24h");
        let out: Value = serde_json::from_str(&s).unwrap();
        eprintln!("trenches status {s}");
        if out["ok"] == true {
            assert_eq!(out["source"], "vapurr");
            assert!(out["status"]["wallets"].as_u64().unwrap_or(0) > 0);
        }
    }
}
