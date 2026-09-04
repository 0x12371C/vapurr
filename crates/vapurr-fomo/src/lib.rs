//! Native social trading desk.
//!
//! Rebuilds the fomo.family *product* inside vapurr (trending board, social
//! pulse, Ketflix). The tape is passed through from fomo.family's public feed
//! (fomoapi.io keyless alerts + optional FOMO_API_KEY trending board). Tokens
//! are priority-indexed: Robinhood Chain (4663) first, then FOMO rank / size.
//! Token hrefs stay on https://fomo.family/token/…. Trades still settle later
//! through vapurr-wallet. This is not a scrape of their HTML and not their brand.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use vapurr_rhc::{CHAIN_ID, CHAIN_NAME, EXPLORER, RPC_HTTP};

mod trenches;
pub use trenches::api_json as trenches_json;

const FOMO_API: &str = "https://api.fomoapi.io";
const FAMILY: &str = "https://fomo.family";
const CACHE_MS: u128 = 8_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub symbol: String,
    pub name: String,
    pub chain: String,
    pub address: String,
    pub price_usd: String,
    pub mcap: String,
    pub vol24: String,
    pub change24: String,
    pub url: String,
    #[serde(default)]
    pub rank: u32,
    #[serde(default)]
    pub hits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pulse {
    pub who: String,
    pub verb: String,
    pub token: String,
    pub usd: String,
    pub chain: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Show {
    pub name: String,
    pub ticker: String,
    pub cells: u32,
    pub note: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FomoDesk {
    pub ok: bool,
    pub source: String,
    pub chain: String,
    pub rpc: String,
    pub explorer: String,
    pub family: String,
    pub trending: Vec<Token>,
    pub feed: Vec<Pulse>,
    pub ketflix: Show,
}

struct Cache {
    desk: FomoDesk,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);
static LOOP: AtomicBool = AtomicBool::new(false);

pub fn warm() {
    kick();
    trenches::warm();
}

fn kick() {
    if LOOP.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("fomo-desk".into())
        .spawn(|| loop {
            let d = fetch_desk();
            if let Ok(mut guard) = CACHE.lock() {
                *guard = Some(Cache { desk: d });
            }
            std::thread::sleep(Duration::from_millis(CACHE_MS as u64));
        });
}

pub fn desk() -> FomoDesk {
    kick();
    if let Ok(guard) = CACHE.lock() {
        if let Some(c) = guard.as_ref() {
            return c.desk.clone();
        }
    }
    empty_desk()
}

fn fetch_desk() -> FomoDesk {
    let mut d = empty_desk();
    let http = match client() {
        Some(c) => c,
        None => return d,
    };
    let liq = rhc_book();
    let mut alerts = fetch_json(&http, &format!("{FOMO_API}/v2/alerts?limit=100&chain=robinhood"))
        .map(|v| alert_rows(&v))
        .unwrap_or_default();
    let global = fetch_json(&http, &format!("{FOMO_API}/v2/alerts?limit=50"))
        .map(|v| alert_rows(&v))
        .unwrap_or_default();
    merge_alerts(&mut alerts, global);

    let keyed = fomo_key().is_some();
    let board = if keyed {
        fetch_json(
            &http,
            &format!("{FOMO_API}/v2/leaderboard/tokens/trending?limit=40"),
        )
        .map(|v| parse_trending_board(&v))
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut trending = if !board.is_empty() {
        d.source = "fomo.family".into();
        board
    } else {
        index_from_alerts(&alerts, &liq)
    };
    overlay_liq(&mut trending, &liq);
    if trending.is_empty() {
        trending = liq_tokens(&liq);
        if !trending.is_empty() {
            d.source = "rhc-liq".into();
        }
    } else {
        d.source = "fomo.family".into();
    }
    if trending.is_empty() {
        trending = rail_tokens();
    }
    priority_index(&mut trending);
    let mut feed = parse_alerts_rows(&alerts);
    priority_feed(&mut feed);
    feed.truncate(32);
    trending.truncate(28);

    d.ok = !trending.is_empty() && trending.iter().any(|t| t.hits > 0 || t.rank > 0 || t.chain == "robinhood");
    if d.source == "idle" && d.ok {
        d.source = "fomo.family".into();
    }
    d.trending = trending;
    d.feed = feed;
    d.ketflix = ketflix_status();
    d
}

pub fn desk_json() -> String {
    serde_json::to_string(&desk()).unwrap_or_else(|_| "{}".into())
}

fn empty_desk() -> FomoDesk {
    FomoDesk {
        ok: false,
        source: "idle".into(),
        chain: CHAIN_NAME.into(),
        rpc: RPC_HTTP.into(),
        explorer: EXPLORER.into(),
        family: FAMILY.into(),
        trending: rail_tokens(),
        feed: vec![],
        ketflix: ketflix_status(),
    }
}

fn client() -> Option<reqwest::blocking::Client> {
    let mut b = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("vapurr/0.1");
    if let Some(k) = fomo_key() {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("Bearer {k}")) {
            headers.insert(reqwest::header::AUTHORIZATION, v);
        }
        b = b.default_headers(headers);
    }
    b.build().ok()
}

fn fomo_key() -> Option<String> {
    std::env::var("FOMO_API_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn fetch_json(http: &reqwest::blocking::Client, url: &str) -> Option<Value> {
    http.get(url).send().ok()?.json().ok()
}

fn alert_rows(v: &Value) -> Vec<Value> {
    v.get("alerts")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
}

fn merge_alerts(dst: &mut Vec<Value>, extra: Vec<Value>) {
    let mut seen = std::collections::HashSet::new();
    for a in dst.iter() {
        if let Some(id) = a.get("id").and_then(|x| x.as_str()) {
            seen.insert(id.to_string());
        }
    }
    for a in extra {
        let id = a
            .get("id")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() || seen.insert(id) {
            dst.push(a);
        }
    }
}

struct LiqTok {
    symbol: String,
    name: String,
    price: f64,
    mcap: f64,
    vol: f64,
}

fn rhc_book() -> HashMap<String, LiqTok> {
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
        if !addr.starts_with("0x") {
            continue;
        }
        let symbol = t
            .get("symbol")
            .and_then(|x| x.as_str())
            .unwrap_or("—")
            .to_string();
        out.insert(
            addr,
            LiqTok {
                name: symbol.clone(),
                symbol,
                price: t.get("price_usd").and_then(|x| x.as_f64()).unwrap_or(0.0),
                mcap: t.get("mcap_usd").and_then(|x| x.as_f64()).unwrap_or(0.0),
                vol: t.get("vol24_usd").and_then(|x| x.as_f64()).unwrap_or(0.0),
            },
        );
    }
    out
}

fn liq_tokens(liq: &HashMap<String, LiqTok>) -> Vec<Token> {
    let mut rows: Vec<_> = liq
        .iter()
        .filter(|(addr, _)| {
            let a = addr.as_str();
            a != vapurr_rhc::USDG.to_ascii_lowercase() && a != vapurr_rhc::WETH.to_ascii_lowercase()
        })
        .map(|(addr, t)| Token {
            symbol: t.symbol.clone(),
            name: t.name.clone(),
            chain: "robinhood".into(),
            address: addr.clone(),
            price_usd: if t.price > 0.0 {
                fmt_compact(t.price)
            } else {
                "—".into()
            },
            mcap: if t.mcap > 0.0 {
                fmt_compact(t.mcap)
            } else {
                "—".into()
            },
            vol24: if t.vol > 0.0 {
                fmt_compact(t.vol)
            } else {
                "—".into()
            },
            change24: "—".into(),
            url: family_href(addr),
            rank: 0,
            hits: 0,
        })
        .collect();
    rows.sort_by(|a, b| {
        let av = liq
            .get(&a.address)
            .map(|t| t.vol)
            .unwrap_or(0.0);
        let bv = liq
            .get(&b.address)
            .map(|t| t.vol)
            .unwrap_or(0.0);
        bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

fn overlay_liq(tokens: &mut [Token], liq: &HashMap<String, LiqTok>) {
    for t in tokens {
        if t.chain != "robinhood" {
            continue;
        }
        let Some(l) = liq.get(&t.address.to_ascii_lowercase()) else {
            continue;
        };
        if t.price_usd == "—" && l.price > 0.0 {
            t.price_usd = fmt_compact(l.price);
        }
        if t.mcap == "—" && l.mcap > 0.0 {
            t.mcap = fmt_compact(l.mcap);
        }
        if t.vol24 == "—" && l.vol > 0.0 {
            t.vol24 = fmt_compact(l.vol);
        }
        if t.name == t.symbol && !l.symbol.is_empty() {
            t.name = l.name.clone();
        }
    }
}

#[derive(Default)]
struct Acc {
    symbol: String,
    name: String,
    chain: String,
    address: String,
    usd: f64,
    hits: u32,
    last_ts: u64,
}

fn index_from_alerts(alerts: &[Value], liq: &HashMap<String, LiqTok>) -> Vec<Token> {
    let mut map: HashMap<String, Acc> = HashMap::new();
    for a in alerts {
        let addr = a
            .get("tokenAddress")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if addr.is_empty() {
            continue;
        }
        let chain = norm_chain(
            a.get("chain").and_then(|x| x.as_str()).unwrap_or(""),
            a.get("chainId").and_then(|x| x.as_u64()),
        );
        let key = format!("{}:{}", chain, addr.to_ascii_lowercase());
        let ent = map.entry(key).or_insert_with(|| Acc {
            symbol: a
                .get("token")
                .and_then(|x| x.as_str())
                .unwrap_or("—")
                .to_string(),
            name: a
                .get("token")
                .and_then(|x| x.as_str())
                .unwrap_or("—")
                .to_string(),
            chain: chain.clone(),
            address: if chain == "robinhood" {
                addr.to_ascii_lowercase()
            } else {
                addr.clone()
            },
            ..Acc::default()
        });
        ent.hits += 1;
        ent.usd += num(a.get("usdValue"));
        let ts = a.get("ts").and_then(|x| x.as_u64()).unwrap_or(0);
        if ts > ent.last_ts {
            ent.last_ts = ts;
        }
        if let Some(l) = liq.get(&ent.address) {
            if !l.symbol.is_empty() {
                ent.symbol = l.symbol.clone();
                ent.name = l.name.clone();
            }
        }
    }
    let mut rows: Vec<Token> = map
        .into_values()
        .map(|e| {
            let l = liq.get(&e.address);
            Token {
                symbol: e.symbol,
                name: e.name,
                chain: e.chain,
                address: e.address.clone(),
                price_usd: l
                    .filter(|t| t.price > 0.0)
                    .map(|t| fmt_compact(t.price))
                    .unwrap_or_else(|| "—".into()),
                mcap: l
                    .filter(|t| t.mcap > 0.0)
                    .map(|t| fmt_compact(t.mcap))
                    .unwrap_or_else(|| "—".into()),
                vol24: if e.usd > 0.0 {
                    fmt_compact(e.usd)
                } else {
                    l.filter(|t| t.vol > 0.0)
                        .map(|t| fmt_compact(t.vol))
                        .unwrap_or_else(|| "—".into())
                },
                change24: format!("{} live", e.hits),
                url: family_href(&e.address),
                rank: 0,
                hits: e.hits,
            }
        })
        .collect();
    priority_index(&mut rows);
    rows
}

pub fn parse_trending_board(v: &Value) -> Vec<Token> {
    let arr = v
        .get("tokens")
        .or_else(|| v.get("items"))
        .or_else(|| v.get("data"))
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_else(|| {
            if let Value::Array(a) = v {
                a.clone()
            } else {
                vec![]
            }
        });
    let mut rows: Vec<Token> = arr
        .into_iter()
        .filter_map(|row| {
            let tok = row.get("token").cloned().unwrap_or(row.clone());
            let addr = tok
                .get("address")
                .or_else(|| row.get("address"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if addr.is_empty() {
                return None;
            }
            let chain = norm_chain(
                row.get("network")
                    .or_else(|| row.get("chain"))
                    .and_then(|x| x.as_str())
                    .unwrap_or(""),
                row.get("networkId")
                    .or_else(|| row.get("chainId"))
                    .and_then(|x| x.as_u64()),
            );
            let symbol = tok
                .get("symbol")
                .or_else(|| row.get("symbol"))
                .and_then(|x| x.as_str())
                .unwrap_or("—")
                .to_string();
            let name = tok
                .get("name")
                .or_else(|| row.get("name"))
                .and_then(|x| x.as_str())
                .unwrap_or(&symbol)
                .chars()
                .take(48)
                .collect();
            let rank = row
                .get("rank")
                .and_then(|x| x.as_u64())
                .unwrap_or(0) as u32;
            Some(Token {
                symbol,
                name,
                chain,
                address: addr.clone(),
                price_usd: fmt_num(
                    row.get("priceUsd")
                        .or_else(|| tok.get("priceUsd")),
                ),
                mcap: fmt_num(
                    row.get("marketCapUsd")
                        .or_else(|| tok.get("marketCapUsd")),
                ),
                vol24: fmt_num(
                    row.get("volume24hUsd")
                        .or_else(|| row.get("fomoBuyers")),
                ),
                change24: fmt_chg(row.get("change24h")),
                url: family_href(&addr),
                rank,
                hits: row
                    .get("fomoBuyers")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0) as u32,
            })
        })
        .collect();
    priority_index(&mut rows);
    rows
}

pub fn parse_alerts_rows(alerts: &[Value]) -> Vec<Pulse> {
    let mut rows: Vec<(u64, u8, Pulse)> = alerts
        .iter()
        .filter_map(|a| {
            let token = a.get("token").and_then(|x| x.as_str())?.to_string();
            if token.is_empty() {
                return None;
            }
            let addr = a
                .get("tokenAddress")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let chain = norm_chain(
                a.get("chain").and_then(|x| x.as_str()).unwrap_or(""),
                a.get("chainId").and_then(|x| x.as_u64()),
            );
            let who = a
                .get("trader")
                .and_then(|x| x.as_str())
                .unwrap_or("anon")
                .to_string();
            let verb = a
                .get("type")
                .and_then(|x| x.as_str())
                .unwrap_or("live")
                .to_string();
            let usd = match a.get("usdValue") {
                Some(Value::Number(n)) => n
                    .as_f64()
                    .map(fmt_compact)
                    .unwrap_or_else(|| "—".into()),
                Some(Value::String(s)) => s
                    .parse::<f64>()
                    .map(fmt_compact)
                    .unwrap_or_else(|_| s.clone()),
                _ => "—".into(),
            };
            let ts = a.get("ts").and_then(|x| x.as_u64()).unwrap_or(0);
            Some((
                ts,
                chain_pri(&chain),
                Pulse {
                    who,
                    verb,
                    token,
                    usd,
                    chain,
                    href: family_href(&addr),
                },
            ))
        })
        .collect();
    rows.sort_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)));
    rows.into_iter().map(|(_, _, p)| p).collect()
}

pub fn family_href(address: &str) -> String {
    if address.is_empty() {
        FAMILY.into()
    } else {
        format!("{FAMILY}/token/{address}")
    }
}

pub fn norm_chain(s: &str, chain_id: Option<u64>) -> String {
    if chain_id == Some(CHAIN_ID) {
        return "robinhood".into();
    }
    match s.trim().to_ascii_lowercase().as_str() {
        "rh" | "hood" | "robinhood chain" | "robinhoodchain" => "robinhood".into(),
        "eth" | "ethereum" => "eth".into(),
        other if other.is_empty() => "unknown".into(),
        other => other.into(),
    }
}

fn chain_pri(chain: &str) -> u8 {
    match chain {
        "robinhood" => 0,
        "base" | "bsc" | "eth" | "ethereum" => 1,
        _ => 2,
    }
}

pub fn priority_index(tokens: &mut [Token]) {
    tokens.sort_by(|a, b| {
        chain_pri(&a.chain)
            .cmp(&chain_pri(&b.chain))
            .then_with(|| {
                let ar = if a.rank == 0 { u32::MAX } else { a.rank };
                let br = if b.rank == 0 { u32::MAX } else { b.rank };
                ar.cmp(&br)
            })
            .then(b.hits.cmp(&a.hits))
    });
    for (i, t) in tokens.iter_mut().enumerate() {
        t.rank = (i as u32) + 1;
        if t.url.is_empty() {
            t.url = family_href(&t.address);
        }
    }
}

fn priority_feed(feed: &mut [Pulse]) {
    feed.sort_by(|a, b| chain_pri(&a.chain).cmp(&chain_pri(&b.chain)));
}

fn rail_tokens() -> Vec<Token> {
    vec![
        Token {
            symbol: "USDG".into(),
            name: "USDG".into(),
            chain: "robinhood".into(),
            address: vapurr_rhc::USDG.into(),
            price_usd: "1.00".into(),
            mcap: "—".into(),
            vol24: "—".into(),
            change24: "—".into(),
            url: family_href(vapurr_rhc::USDG),
            rank: 1,
            hits: 0,
        },
        Token {
            symbol: "WETH".into(),
            name: "Wrapped ETH".into(),
            chain: "robinhood".into(),
            address: vapurr_rhc::WETH.into(),
            price_usd: "—".into(),
            mcap: "—".into(),
            vol24: "—".into(),
            change24: "—".into(),
            url: family_href(vapurr_rhc::WETH),
            rank: 2,
            hits: 0,
        },
    ]
}

pub fn parse_pairs(v: &Value) -> Vec<Token> {
    let pairs = match v.get("pairs").and_then(|x| x.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    pairs
        .iter()
        .filter_map(|p| {
            let base = p.get("baseToken")?;
            let addr = base.get("address")?.as_str()?.to_string();
            if addr.is_empty() {
                return None;
            }
            let chain = norm_chain(
                p.get("chainId").and_then(|x| x.as_str()).unwrap_or("unknown"),
                None,
            );
            let symbol = base
                .get("symbol")
                .and_then(|x| x.as_str())
                .unwrap_or("—")
                .to_string();
            let name = base
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or(&symbol)
                .chars()
                .take(48)
                .collect();
            Some(Token {
                symbol,
                name,
                chain,
                address: addr.clone(),
                price_usd: fmt_num(p.get("priceUsd")),
                mcap: fmt_num(
                    p.get("marketCap")
                        .or_else(|| p.get("fdv"))
                        .or_else(|| p.get("liquidity").and_then(|l| l.get("usd"))),
                ),
                vol24: fmt_num(p.get("volume").and_then(|v| v.get("h24"))),
                change24: fmt_chg(p.get("priceChange").and_then(|v| v.get("h24"))),
                url: family_href(&addr),
                rank: 0,
                hits: 0,
            })
        })
        .take(40)
        .collect()
}

fn fmt_num(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n
            .as_f64()
            .map(fmt_compact)
            .unwrap_or_else(|| n.to_string()),
        Some(Value::String(s)) => s
            .parse::<f64>()
            .map(fmt_compact)
            .unwrap_or_else(|_| s.clone()),
        _ => "—".into(),
    }
}

fn fmt_chg(v: Option<&Value>) -> String {
    let n = match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    };
    match n {
        Some(x) if x.is_finite() => format!("{x:+.1}%"),
        _ => "—".into(),
    }
}

fn fmt_compact(n: f64) -> String {
    if !n.is_finite() {
        return "—".into();
    }
    let a = n.abs();
    if a >= 1_000_000_000.0 {
        format!("{:.1}B", n / 1_000_000_000.0)
    } else if a >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if a >= 1_000.0 {
        format!("{:.1}K", n / 1_000.0)
    } else if a >= 1.0 {
        format!("{n:.2}")
    } else {
        format!("{n:.4}")
    }
}

fn num(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

pub fn parse_boosts(v: &Value) -> Vec<Token> {
    let arr = match v {
        Value::Array(a) => a.clone(),
        Value::Object(o) => o
            .get("boosts")
            .or_else(|| o.get("tokens"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => vec![],
    };
    arr.into_iter()
        .filter_map(|row| {
            let addr = row
                .get("tokenAddress")
                .or_else(|| row.get("address"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if addr.is_empty() {
                return None;
            }
            let chain = row
                .get("chainId")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string();
            let desc = row
                .get("description")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let symbol = row
                .get("symbol")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| short_sym(&desc, &addr));
            Some(Token {
                symbol,
                name: desc.chars().take(48).collect(),
                chain,
                address: addr.clone(),
                price_usd: "—".into(),
                mcap: fmt_amt(row.get("totalAmount").or_else(|| row.get("amount"))),
                vol24: "—".into(),
                change24: "—".into(),
                url: family_href(&addr),
                rank: 0,
                hits: 0,
            })
        })
        .take(40)
        .collect()
}

fn short_sym(desc: &str, addr: &str) -> String {
    let w = desc
        .split_whitespace()
        .find(|t| t.chars().all(|c| c.is_ascii_alphanumeric()) && t.len() >= 2 && t.len() <= 8)
        .unwrap_or("");
    if !w.is_empty() {
        return w.to_ascii_uppercase();
    }
    addr.chars()
        .rev()
        .take(4)
        .collect::<String>()
        .to_ascii_uppercase()
}

fn fmt_amt(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => "—".into(),
    }
}

pub fn ketflix_status() -> Show {
    let dir = ketflix_dir();
    let cells = count_cells(&dir);
    Show {
        name: "Ketflix".into(),
        ticker: "KFLX".into(),
        cells,
        note: if cells == 0 {
            "local show paused — 5s T2V cells, English muxed, Robinhood Chain votes not deployed".into()
        } else {
            format!("{cells} local cells on disk. Not live TV.")
        },
        path: dir.display().to_string(),
    }
}

fn ketflix_dir() -> PathBuf {
    if let Ok(p) = std::env::var("VAPURR_KETFLIX_DIR") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("vapurr").join("ketflix")
}

fn count_cells(dir: &PathBuf) -> u32 {
    let Ok(rd) = fs::read_dir(dir) else {
        return 0;
    };
    rd.filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.starts_with("tv_") && n.ends_with(".mp4") && !n.contains("raw")
        })
        .count() as u32
}

pub fn token_href(chain: &str, address: &str) -> String {
    let _ = chain;
    family_href(address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_boost_array() {
        let v = json!([
            {
                "tokenAddress": "0xabcabcabcabcabcabcabcabcabcabcabcabcabca",
                "chainId": "base",
                "description": "PEPE the frog",
                "totalAmount": 1200,
                "url": "https://dexscreener.com/base/0xabc"
            }
        ]);
        let t = parse_boosts(&v);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].chain, "base");
        assert!(t[0].url.contains("fomo.family/token/"));
        assert!(t[0].symbol.contains("PEPE") || t[0].symbol.len() >= 3);
    }

    #[test]
    fn parses_pairs_passes_family() {
        let v = json!({
            "pairs": [{
                "chainId": "robinhood",
                "url": "https://dexscreener.com/robinhood/xyz",
                "baseToken": { "address": "0x5fc5360d0400a0fd4f2af552add042d716f1d168", "name": "USDG", "symbol": "USDG" },
                "priceUsd": "1.00",
                "volume": { "h24": 12000 },
                "priceChange": { "h24": 0.2 },
                "marketCap": 1000000
            }]
        });
        let t = parse_pairs(&v);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].symbol, "USDG");
        assert_eq!(t[0].chain, "robinhood");
        assert_eq!(
            t[0].url,
            "https://fomo.family/token/0x5fc5360d0400a0fd4f2af552add042d716f1d168"
        );
    }

    #[test]
    fn alerts_priority_indexes_robinhood_first() {
        let alerts = vec![
            json!({
                "id": "s1",
                "type": "buy",
                "trader": "sol_whale",
                "token": "BONK",
                "tokenAddress": "DezSomethingSolanaMint111111111111111",
                "chainId": 1399811149,
                "chain": "solana",
                "usdValue": 90000,
                "ts": 200
            }),
            json!({
                "id": "r1",
                "type": "buy",
                "trader": "BraveValidCamel",
                "token": "BONER",
                "tokenAddress": "0x98096d17e191b3da1d5f99a6d7b3584351b11e18",
                "chainId": 4663,
                "chain": "robinhood",
                "usdValue": 7000,
                "ts": 100
            }),
            json!({
                "id": "r2",
                "type": "sell",
                "trader": "abdhzim",
                "token": "JINQIAN",
                "tokenAddress": "0xe81880c1c5054245e036359f5c7be31606e79f56",
                "chainId": 4663,
                "chain": "robinhood",
                "usdValue": 1200,
                "ts": 300
            }),
        ];
        let empty = HashMap::new();
        let t = index_from_alerts(&alerts, &empty);
        assert!(t.len() >= 2);
        assert_eq!(t[0].chain, "robinhood");
        assert_eq!(t[0].rank, 1);
        assert!(t[0].url.starts_with("https://fomo.family/token/"));
        assert!(t.iter().all(|x| x.chain != "solana" || x.rank > t[0].rank));
        let feed = parse_alerts_rows(&alerts);
        assert_eq!(feed[0].chain, "robinhood");
        assert_eq!(feed[0].href.contains("fomo.family"), true);
        assert_eq!(feed.last().unwrap().chain, "solana");
    }

    #[test]
    fn ketflix_has_name() {
        let s = ketflix_status();
        assert_eq!(s.ticker, "KFLX");
        assert_eq!(s.name, "Ketflix");
    }

    #[test]
    fn desk_json_is_object() {
        let v: Value = serde_json::from_str(&desk_json()).unwrap();
        assert!(v.get("ketflix").is_some());
        assert!(v.get("trending").is_some());
        assert_eq!(v.get("family").and_then(|x| x.as_str()), Some(FAMILY));
    }
}
