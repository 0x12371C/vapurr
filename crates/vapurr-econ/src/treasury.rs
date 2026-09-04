//! $VAPURR / $PUSD treasury. Log-optimal book of Robinhood Chain assets.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use vapurr_rhc::{self as rhc, USDG, WETH};

use crate::kelly::{arith_dump, log_optimal, mean_arith, mean_log};

const RING: usize = 48;
const NAMES: usize = 6;
const X_LO: f64 = 0.25;
const X_HI: f64 = 4.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Asset {
    id: String,
    address: String,
    symbol: String,
    px: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Book {
    assets: Vec<Asset>,
    xs: Vec<Vec<f64>>,
    b: Vec<f64>,
}

fn path() -> std::path::PathBuf {
    vapurr_wallet::data_dir().join("treasury.json")
}

fn load() -> Book {
    std::fs::read(path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save(book: &Book) {
    let _ = std::fs::create_dir_all(vapurr_wallet::data_dir());
    if let Ok(bytes) = serde_json::to_vec_pretty(book) {
        let _ = std::fs::write(path(), bytes);
    }
}

fn universe(liq: &Value) -> Vec<Asset> {
    let mut out = vec![Asset {
        id: "usdg".into(),
        address: USDG.to_ascii_lowercase(),
        symbol: "USDG".into(),
        px: 1.0,
    }];
    let mut rows: Vec<&Value> = liq
        .get("tokens")
        .and_then(|x| x.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        let av = a.get("tvl_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let bv = b.get("tvl_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
    });
    let usdg = USDG.to_ascii_lowercase();
    let weth = WETH.to_ascii_lowercase();
    let mut have_weth = false;
    for t in rows {
        if out.len() >= NAMES {
            break;
        }
        let addr = t
            .get("address")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if addr.is_empty() || addr == usdg {
            continue;
        }
        let px = t.get("price_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
        if px <= 0.0 || !px.is_finite() {
            continue;
        }
        let kind = t.get("kind").and_then(|x| x.as_str()).unwrap_or("");
        if kind == "stable" {
            continue;
        }
        let symbol = t
            .get("symbol")
            .and_then(|x| x.as_str())
            .unwrap_or("?")
            .to_string();
        if addr == weth {
            have_weth = true;
        }
        out.push(Asset {
            id: symbol.to_ascii_lowercase(),
            address: addr,
            symbol,
            px,
        });
    }
    if !have_weth && out.len() < NAMES {
        if let Some(t) = liq
            .get("tokens")
            .and_then(|x| x.as_array())
            .and_then(|a| a.iter().find(|t| {
                t.get("address")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .eq_ignore_ascii_case(WETH)
            }))
        {
            let px = t.get("price_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
            if px > 0.0 {
                out.insert(
                    1.min(out.len()),
                    Asset {
                        id: "weth".into(),
                        address: weth,
                        symbol: "WETH".into(),
                        px,
                    },
                );
                out.truncate(NAMES);
            }
        }
    }
    out
}

fn ingest(book: &mut Book, next: Vec<Asset>) {
    if next.is_empty() {
        return;
    }
    if book.assets.len() != next.len()
        || book
            .assets
            .iter()
            .zip(next.iter())
            .any(|(a, b)| a.address != b.address)
    {
        book.assets = next;
        book.xs.clear();
        book.b = vec![1.0 / book.assets.len() as f64; book.assets.len()];
        return;
    }
    let n = next.len();
    let mut x = vec![1.0; n];
    let mut moved = false;
    for i in 0..n {
        let prev = book.assets[i].px;
        let now = next[i].px;
        if prev > 0.0 && now > 0.0 && (now - prev).abs() / prev > 1e-6 {
            let r = (now / prev).clamp(X_LO, X_HI);
            x[i] = r;
            moved = true;
        }
        book.assets[i].px = now;
    }
    x[0] = 1.0;
    if !moved {
        return;
    }
    book.xs.push(x);
    if book.xs.len() > RING {
        book.xs.remove(0);
    }
}

fn solve(book: &mut Book) {
    let n = book.assets.len();
    if n == 0 {
        book.b.clear();
        return;
    }
    book.b = log_optimal(&book.xs, 500);
    if book.b.len() != n {
        book.b = vec![1.0 / n as f64; n];
    }
}

/// Live book from the Scan cache. Do not warm the mainnet crawler here —
/// pulse and the econ thread would 429 Robinhood RPC and stall testnet txs.
pub fn snap() -> Value {
    let mut book = load();
    if let Some(liq) = rhc::liq::cached_ok() {
        ingest(&mut book, universe(&liq));
        solve(&mut book);
        save(&book);
    } else if book.assets.is_empty() {
        book.assets = vec![Asset {
            id: "usdg".into(),
            address: USDG.to_ascii_lowercase(),
            symbol: "USDG".into(),
            px: 1.0,
        }];
        book.b = vec![1.0];
    }
    let n = book.assets.len();
    let dump = arith_dump(&book.xs);
    let dump_i = dump
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let rows: Vec<Value> = book
        .assets
        .iter()
        .enumerate()
        .map(|(i, a)| {
            json!({
                "id": a.id,
                "symbol": a.symbol,
                "address": a.address,
                "px": a.px,
                "b": book.b.get(i).copied().unwrap_or(0.0),
                "pct": ((book.b.get(i).copied().unwrap_or(0.0) * 1000.0).round() / 10.0),
            })
        })
        .collect();
    json!({
        "ok": true,
        "formula": "W* = max E[log bᵀX]",
        "n": n,
        "windows": book.xs.len(),
        "ready": book.xs.len() >= 2,
        "growth": mean_log(&book.b, &book.xs),
        "arith": mean_arith(&book.b, &book.xs) - 1.0,
        "dump": book.assets.get(dump_i).map(|a| a.symbol.clone()).unwrap_or_default(),
        "assets": rows,
    })
}
