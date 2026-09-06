//! Ketcharts tape. Pairs from the live RHC crawl. Trades from Swap logs.

use super::snapshot::{cached_ok, idle};
use super::warm;
use super::{
    abi_word_human, block_ts, get_logs, token_kind, SWAP_V2, SWAP_V3,
};

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::rpc::{hex_u64, Rpc};
use crate::{CHAIN_ID, CHAIN_NAME};

static TRADES: LazyLock<Mutex<HashMap<String, (Instant, Value)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static FETCHING: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn tape_json() -> String {
    warm();
    let snap = cached_ok().unwrap_or_else(|| idle("loading"));
    let ok = snap.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
    let pairs: Vec<Value> = snap
        .get("pools")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(map_pair)
        .collect();
    json!({
        "ok": ok,
        "loading": !ok,
        "source": "rhc-rpc",
        "chain": CHAIN_NAME,
        "chain_id": CHAIN_ID,
        "stats": snap.get("stats").cloned().unwrap_or(json!({})),
        "pairs": pairs,
    })
    .to_string()
}

pub fn trades_json(pool: &str) -> String {
    warm();
    let pool = pool.trim().to_ascii_lowercase();
    if pool.len() != 42 || !pool.starts_with("0x") {
        return json!({ "ok": false, "error": "bad pool", "trades": [] }).to_string();
    }
    if let Ok(g) = TRADES.lock() {
        if let Some((at, v)) = g.get(&pool) {
            if at.elapsed() < Duration::from_secs(12) {
                return v.to_string();
            }
        }
    }
    kick_trades(pool.clone());
    if let Ok(g) = TRADES.lock() {
        if let Some((_, v)) = g.get(&pool) {
            return v.to_string();
        }
    }
    json!({
        "ok": true,
        "loading": true,
        "source": "rhc-rpc",
        "pool": pool,
        "trades": [],
    })
    .to_string()
}

fn kick_trades(pool: String) {
    {
        let Ok(mut g) = FETCHING.lock() else {
            return;
        };
        if !g.insert(pool.clone()) {
            return;
        }
    }
    let _ = std::thread::Builder::new()
        .name("rhc-trades".into())
        .spawn(move || {
            let v = fetch_trades(&pool);
            if let Ok(mut g) = TRADES.lock() {
                g.insert(pool.clone(), (Instant::now(), v));
                if g.len() > 48 {
                    if let Some(k) = g.keys().next().cloned() {
                        g.remove(&k);
                    }
                }
            }
            if let Ok(mut g) = FETCHING.lock() {
                g.remove(&pool);
            }
        });
}

fn pool_meta(pool: &str) -> Option<Value> {
    let snap = cached_ok()?;
    snap.get("pools")?
        .as_array()?
        .iter()
        .find(|p| {
            p.get("address")
                .and_then(|x| x.as_str())
                .map(|s| s.eq_ignore_ascii_case(pool))
                .unwrap_or(false)
        })
        .cloned()
}

fn fetch_trades(pool: &str) -> Value {
    let house = super::house::is_house_pool(pool);
    let rpc = Rpc::at_timeout(
        if house {
            crate::TESTNET_RPC_HTTP
        } else {
            crate::RPC_HTTP
        },
        10,
    );
    let head = match rpc.call("eth_blockNumber", json!([])) {
        Ok(v) => hex_u64(&v),
        Err(_) => {
            return json!({ "ok": false, "error": "rpc", "pool": pool, "trades": [] });
        }
    };
    if head == 0 {
        return json!({ "ok": false, "error": "head", "pool": pool, "trades": [] });
    }
    let meta = pool_meta(pool);
    let v3 = meta
        .as_ref()
        .and_then(|p| p.get("dex").and_then(|x| x.as_str()))
        .unwrap_or("")
        .contains("v3");
    let d0 = meta
        .as_ref()
        .and_then(|p| p.pointer("/base/decimals"))
        .and_then(|x| x.as_u64())
        .unwrap_or(18) as u8;
    let d1 = meta
        .as_ref()
        .and_then(|p| p.pointer("/quote/decimals"))
        .and_then(|x| x.as_u64())
        .unwrap_or(18) as u8;
    let px0 = meta
        .as_ref()
        .and_then(|p| p.pointer("/base/price_usd"))
        .and_then(|x| x.as_f64())
        .unwrap_or(0.0);
    let px1 = meta
        .as_ref()
        .and_then(|p| p.pointer("/quote/price_usd"))
        .and_then(|x| x.as_f64())
        .unwrap_or(0.0);
    let from = head.saturating_sub(6_000).max(1);
    if house {
        // House Uni v4: Swap logs live on HouseSwap, priced from amountIn/Out (pool), never Lithe feed.
        let trades = super::house::fetch_house_trades(&rpc, from, head);
        return json!({
            "ok": true,
            "loading": false,
            "source": "rhc-rpc",
            "pool": pool,
            "head": head,
            "trades": trades,
        });
    }
    let topic = if v3 { SWAP_V3 } else { SWAP_V2 };
    let logs = match get_logs(&rpc, pool, topic, from, head) {
        Ok(v) => v,
        Err(_) if v3 => get_logs(&rpc, pool, SWAP_V2, from, head).unwrap_or_default(),
        Err(_) => get_logs(&rpc, pool, SWAP_V3, from, head).unwrap_or_default(),
    };
    let ts_hi = block_ts(&rpc, head);
    let sample = head.saturating_sub(from).max(1);
    let dt = if ts_hi > 0 {
        2.0_f64.max(8_000.0 / sample as f64).min(4.0)
    } else {
        2.0
    };
    let mut trades: Vec<Value> = logs
        .iter()
        .filter_map(|log| map_trade(log, d0, d1, px0, px1, v3, ts_hi, head, dt))
        .collect();
    trades.sort_by(|a, b| {
        let at = a.get("block").and_then(|x| x.as_u64()).unwrap_or(0);
        let bt = b.get("block").and_then(|x| x.as_u64()).unwrap_or(0);
        bt.cmp(&at)
    });
    trades.truncate(80);
    json!({
        "ok": true,
        "loading": false,
        "source": "rhc-rpc",
        "pool": pool,
        "head": head,
        "trades": trades,
    })
}

fn map_trade(
    log: &Value,
    d0: u8,
    d1: u8,
    px0: f64,
    px1: f64,
    v3: bool,
    ts_hi: u64,
    head: u64,
    dt: f64,
) -> Option<Value> {
    let bn = hex_u64(log.get("blockNumber").unwrap_or(&Value::Null));
    let data = log.get("data").and_then(|x| x.as_str()).unwrap_or("0x");
    let tx = log
        .get("transactionHash")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if tx.is_empty() {
        return None;
    }
    let (vol, buy, a0, a1) = if v3 {
        let x0 = abi_word_human(data, 0, d0);
        let x1 = abi_word_human(data, 1, d1);
        let v = if px0 > 0.0 {
            x0.abs() * px0
        } else {
            x1.abs() * px1
        };
        (v, x0 < 0.0, x0.abs(), x1.abs())
    } else {
        let i0 = abi_word_human(data, 0, d0).abs();
        let i1 = abi_word_human(data, 1, d1).abs();
        let o0 = abi_word_human(data, 2, d0).abs();
        let o1 = abi_word_human(data, 3, d1).abs();
        (i0 * px0 + i1 * px1, o0 > 0.0, i0.max(o0), i1.max(o1))
    };
    if !(vol.is_finite() && vol > 0.0) {
        return None;
    }
    let px = if a0 > 0.0 && px0 > 0.0 {
        px0
    } else {
        px1
    };
    let behind = head.saturating_sub(bn) as f64;
    let ts = if ts_hi > 0 {
        ts_hi.saturating_sub((behind * dt) as u64)
    } else {
        0
    };
    Some(json!({
        "tx": tx,
        "block": bn,
        "time": ts.saturating_mul(1000),
        "buy": buy,
        "px": px,
        "vol": vol,
        "base_amt": a0,
        "quote_amt": a1,
    }))
}

pub(crate) fn map_pair(p: &Value) -> Option<Value> {
    let pool = p.get("address").and_then(|x| x.as_str())?.to_ascii_lowercase();
    if pool.len() != 42 {
        return None;
    }
    let base = p.get("base")?;
    let quote = p.get("quote")?;
    let sym = base
        .get("symbol")
        .and_then(|x| x.as_str())
        .unwrap_or("???")
        .to_string();
    let qsym = quote
        .get("symbol")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let token = base
        .get("address")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let px = base.get("price_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
    let chg = p
        .get("change24")
        .and_then(|x| x.as_f64())
        .or_else(|| {
            p.get("change24")
                .and_then(|x| x.as_str())
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(0.0);
    let pool_mid = p.get("pool_mid").and_then(|x| x.as_f64());
    let mid_ok = p.get("mid_ok").and_then(|x| x.as_bool()).unwrap_or(false);
    // Prefer real pool mid when ok; never let a zero mid clobber a reserve/slot px.
    let px = if mid_ok {
        pool_mid.filter(|m| *m > 0.0).unwrap_or(px)
    } else {
        pool_mid.filter(|m| *m > 0.0).unwrap_or(px)
    };
    Some(json!({
        "pool": pool,
        "token": token,
        "quote": quote.get("address").and_then(|x| x.as_str()).unwrap_or("").to_ascii_lowercase(),
        "sym": sym,
        "quote_sym": qsym,
        "name": p.get("name").and_then(|x| x.as_str()).unwrap_or(""),
        "dex": p.get("dex").and_then(|x| x.as_str()).unwrap_or(""),
        "fee": p.get("fee").and_then(|x| x.as_str()).unwrap_or(""),
        "px": px,
        "pool_mid": pool_mid,
        "mid_ok": mid_ok,
        "chg": chg,
        "vol": p.get("vol24_usd").and_then(|x| x.as_f64()).unwrap_or(0.0),
        "vol1": p.get("vol1_usd").and_then(|x| x.as_f64()).unwrap_or(0.0),
        "vol6": p.get("vol6_usd").and_then(|x| x.as_f64()).unwrap_or(0.0),
        "liq": p.get("reserve_usd").and_then(|x| x.as_f64()).unwrap_or(0.0),
        "mcap": p.get("mcap_usd").and_then(|x| x.as_f64()).unwrap_or(0.0),
        "buys": p.get("buys24").and_then(|x| x.as_u64()).unwrap_or(0),
        "sells": p.get("sells24").and_then(|x| x.as_u64()).unwrap_or(0),
        "txns": p.get("txns24").and_then(|x| x.as_u64()).unwrap_or(0),
        "kind": token_kind(&token, &sym),
        "source": "rhc",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{USDG, WETH};

    #[test]
    fn maps_usdg_weth_pool() {
        let p = json!({
            "address": "0x52e65b17fb6e5ba00ed806f37afcd2daa50271ca",
            "name": "USDG / WETH 0.01%",
            "dex": "uniswap v3",
            "fee": "0.01%",
            "base": { "address": USDG, "symbol": "USDG", "price_usd": 1.0, "decimals": 6 },
            "quote": { "address": WETH, "symbol": "WETH", "price_usd": 2400.0, "decimals": 18 },
            "reserve_usd": 1000.5,
            "vol1_usd": 10.0,
            "vol6_usd": 40.0,
            "vol24_usd": 90.0,
            "mcap_usd": 0.0,
            "change24": "1.5",
            "buys24": 3,
            "sells24": 2,
            "txns24": 5
        });
        let v = map_pair(&p).unwrap();
        assert_eq!(v["sym"], "USDG");
        assert_eq!(v["quote_sym"], "WETH");
        assert_eq!(v["source"], "rhc");
        assert_eq!(v["liq"], 1000.5);
        assert_eq!(v["vol"], 90.0);
        assert_eq!(v["txns"], 5);
        assert_eq!(v["chg"], 1.5);
        assert_eq!(v["kind"], "stable");
    }

    #[test]
    fn tape_json_is_object() {
        let v: Value = serde_json::from_str(&tape_json()).unwrap();
        assert!(v.get("pairs").and_then(|x| x.as_array()).is_some());
        assert!(v.get("source").and_then(|x| x.as_str()) == Some("rhc-rpc"));
    }
}
