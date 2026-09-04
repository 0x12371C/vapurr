use super::*;


pub(crate) fn kick_head() {
    crate::index::kick();
    if HEAD_LOOP.swap(true, Ordering::SeqCst) {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("scan-head".into())
        .spawn(|| loop {
            if catch_unwind(AssertUnwindSafe(|| {
                let _ = head();
            }))
            .is_err()
            {
                HEAD_LOOP.store(false, Ordering::SeqCst);
                return;
            }
            std::thread::sleep(Duration::from_secs(4));
        });
    if spawned.is_err() {
        HEAD_LOOP.store(false, Ordering::SeqCst);
    }
}

pub(crate) fn index_page(key: &str, r: Result<crate::index::Page, String>) -> Value {
    let mut out = serde_json::Map::new();
    out.insert("ok".into(), json!(true));
    out.insert("now".into(), json!(now_secs()));
    match r {
        Ok(page) => {
            out.insert("index".into(), json!(true));
            out.insert("source".into(), json!("index"));
            out.insert(key.into(), json!(page.items));
            out.insert("next".into(), page.next.unwrap_or(Value::Null));
        }
        Err(_) => {
            out.insert("index".into(), json!(false));
            out.insert("source".into(), json!("rpc"));
            out.insert(key.into(), json!([]));
            out.insert("next".into(), Value::Null);
            out.insert("error".into(), json!("index wait"));
        }
    }
    Value::Object(out)
}


pub(crate) fn list_blocks(cursor: Option<&str>) -> Result<Value, String> {
    crate::index::kick();
    let paged = cursor.map(str::trim).filter(|s| !s.is_empty()).is_some();
    if paged {
        return Ok(index_page("blocks", crate::index::latest_blocks(cursor)));
    }
    if let Some(page) = crate::index::latest_blocks_if_ready() {
        return Ok(index_page("blocks", Ok(page)));
    }
    let mut v = index_page("blocks", Err("index wait".into()));
    if let Some(obj) = v.as_object_mut() {
        obj.insert("blocks".into(), json!(rpc_window_blocks()));
    }
    Ok(v)
}


pub(crate) fn last_head() -> Option<Value> {
    if let Ok(g) = HEAD.lock() {
        return g.as_ref().map(|(_, v)| v.clone());
    }
    None
}


pub(crate) fn remember_head(v: Value) {
    if let Ok(mut g) = HEAD.lock() {
        *g = Some((Instant::now(), v));
    }
}


pub(crate) fn rpc_window_blocks() -> Vec<Value> {
    if let Some(h) = last_head() {
        if let Some(arr) = h.get("blocks").and_then(|v| v.as_array()) {
            if !arr.is_empty() {
                return arr.clone();
            }
        }
    }
    Vec::new()
}


pub(crate) fn rpc_window_txs() -> Vec<Value> {
    if let Some(h) = last_head() {
        if let Some(arr) = h.get("transactions").and_then(|v| v.as_array()) {
            if !arr.is_empty() {
                return arr.clone();
            }
        }
    }
    Vec::new()
}


/// Never RPC. Custom-protocol thread must return immediately.
pub(crate) fn head_snapshot() -> Value {
    last_head().unwrap_or_else(|| {
        json!({
            "ok": false,
            "loading": true,
            "chain": CHAIN_NAME,
            "chain_id": CHAIN_ID,
            "rpc": RPC_HTTP,
            "now": now_secs(),
        })
    })
}


pub(crate) fn head() -> Result<Value, String> {
    if let Ok(g) = HEAD.lock() {
        if let Some((at, v)) = g.as_ref() {
            let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
            let ttl = if ok { HEAD_TTL } else { ERR_TTL };
            if at.elapsed() < ttl {
                return Ok(v.clone());
            }
        }
    }
    match fetch_head() {
        Ok(out) => {
            remember_head(out.clone());
            Ok(out)
        }
        Err(e) => {
            let mut body = json!({
                "ok": false,
                "error": e,
                "chain": CHAIN_NAME,
                "chain_id": CHAIN_ID,
                "rpc": RPC_HTTP,
                "now": now_secs(),
            });
            if let Some(last) = last_head() {
                if last.get("block").is_some() {
                    if let Some(obj) = body.as_object_mut() {
                        obj.insert("stale".into(), json!(true));
                        for key in [
                            "block",
                            "hash",
                            "ts",
                            "gwei",
                            "blocks",
                            "transactions",
                            "load",
                            "gas_used",
                            "gas_limit",
                            "miner",
                            "l1",
                            "spark",
                            "index",
                            "txs",
                            "base_fee",
                            "gas",
                        ] {
                            if let Some(v) = last.get(key) {
                                obj.insert(key.into(), v.clone());
                            }
                        }
                    }
                }
            }
            remember_head(body.clone());
            Ok(body)
        }
    }
}


pub(crate) fn fetch_head() -> Result<Value, String> {
    let n = hex_u64(&call("eth_blockNumber", json!([]))?);
    let gp = call("eth_gasPrice", json!([]))?;
    // Hashes only. Full tx objects on the latest Orbit block freeze the chrome.
    let latest = call("eth_getBlockByNumber", json!([hex_n(n), false]))?;
    let mut reqs = Vec::new();
    for i in 1..OVERVIEW_BLOCKS {
        if n >= i {
            reqs.push(json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "eth_getBlockByNumber",
                "params": [hex_n(n - i), false]
            }));
        }
    }
    let prev = batch(reqs)?;
    let mut blocks = vec![summarize_block(&latest, true)];
    for p in prev {
        if !p.is_null() {
            blocks.push(summarize_block(&p, false));
        }
    }
    let txs = txs_from_block(&latest, n, Some(OVERVIEW_TXS));
    let gas_used = hex_u64(latest.get("gasUsed").unwrap_or(&Value::Null));
    let gas_limit = hex_u64(latest.get("gasLimit").unwrap_or(&Value::Null)).max(1);
    let gp_n = hex_u128_val(&gp) as f64 / 1e9;
    let base_n = hex_u128_val(latest.get("baseFeePerGas").unwrap_or(&Value::Null)) as f64 / 1e9;
    let out = json!({
        "ok": true,
        "chain": CHAIN_NAME,
        "chain_id": CHAIN_ID,
        "rpc": RPC_HTTP,
        "block": n,
        "hash": str_of(&latest, "hash"),
        "gwei": fmt_gwei(&gp),
        "gwei_n": gp_n,
        "base_fee": fmt_gwei(latest.get("baseFeePerGas").unwrap_or(&Value::Null)),
        "base_fee_n": base_n,
        "gas_used": gas_used,
        "gas_limit": gas_limit,
        "load": (gas_used as f64 / gas_limit as f64).clamp(0.0, 1.0),
        "ts": hex_u64(latest.get("timestamp").unwrap_or(&Value::Null)),
        "txs": latest.get("transactions").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        "l1": opt_u64(latest.get("l1BlockNumber")),
        "miner": str_of(&latest, "miner"),
        "blocks": blocks,
        "transactions": txs,
        "tx_source": "rpc",
        "block_source": "rpc",
        "source": "rpc",
        "now": now_secs(),
    });
    Ok(enrich_head(out))
}


pub(crate) fn enrich_head(mut out: Value) -> Value {
    if let Some(obj) = out.as_object_mut() {
        let spark: Vec<f64> = obj
            .get("blocks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|b| b.get("load").and_then(|v| v.as_f64()))
                    .collect()
            })
            .unwrap_or_default();
        let used: Vec<u64> = obj
            .get("blocks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|b| b.get("gas_used").and_then(|v| v.as_u64()))
                    .collect()
            })
            .unwrap_or_default();
        let max_used = used.iter().copied().max().unwrap_or(1).max(1);
        let spark: Vec<f64> = if spark.iter().all(|v| *v < 0.01) {
            used.iter()
                .map(|u| (*u as f64 / max_used as f64).clamp(0.0, 1.0))
                .collect()
        } else {
            spark
        };
        obj.insert("spark".into(), json!(spark));
        let load = obj.get("load").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let relative = spark.first().copied().unwrap_or(load);
        let base = obj.get("base_fee_n").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let price = obj.get("gwei_n").and_then(|v| v.as_f64()).unwrap_or(base);
        let slow = if base > 0.0 { base } else { price * 0.9 };
        let avg = if price > 0.0 { price } else { base };
        let fast = (avg * 1.25).max(base * 1.125);
        let heat = if relative >= 0.85 {
            "hot"
        } else if relative >= 0.55 {
            "warm"
        } else {
            "cool"
        };
        let history: Vec<Value> = obj
            .get("blocks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|b| {
                        Some(json!({
                            "n": b.get("number")?,
                            "load": b.get("load")?,
                            "base": b.get("base_fee_n")?,
                        }))
                    })
                    .collect()
            })
            .unwrap_or_default();
        obj.insert(
            "gas".into(),
            json!({
                "ok": true,
                "block": obj.get("block"),
                "load": load,
                "relative": relative,
                "gas_used": obj.get("gas_used"),
                "base": base,
                "price": price,
                "slow": slow,
                "avg": avg,
                "fast": fast,
                "heat": heat,
                "gwei": obj.get("gwei"),
                "base_fee": obj.get("base_fee"),
                "spark": spark,
                "history": history,
                "now": obj.get("now"),
            }),
        );
        obj.entry("tx_source".to_string())
            .or_insert_with(|| json!("rpc"));
        obj.entry("block_source".to_string())
            .or_insert_with(|| json!("rpc"));
        obj.entry("source".to_string()).or_insert_with(|| json!("rpc"));
        if let Some(st) = crate::index::stats_if_ready() {
            obj.insert("index".into(), st);
        } else {
            obj.insert("index".into(), json!({ "ok": false }));
        }
        if let Some(px) = native_usd() {
            obj.insert("eth_usd".into(), json!(px));
        }
    }
    crate::liq::warm();
    if let Some(obj) = out.as_object_mut() {
        let liq = crate::liq::stats_if_ready().unwrap_or_else(|| {
            json!({
                "ok": false,
                "loading": true,
                "tvl_usd": 0.0,
                "vol24_usd": 0.0,
                "pools": 0,
                "tokens": 0
            })
        });
        obj.insert("liq".into(), liq);
    }
    out
}


pub(crate) fn block(id: &str, page: u64) -> Result<Value, String> {
    let raw = match classify(id) {
        Query::BlockNumber(n) => call("eth_getBlockByNumber", json!([hex_n(n), true]))?,
        Query::Hash(h) => call("eth_getBlockByHash", json!([h, true]))?,
        _ => {
            if id.chars().all(|c| c.is_ascii_digit()) {
                let n: u64 = id.parse().map_err(|_| "bad block")?;
                call("eth_getBlockByNumber", json!([hex_n(n), true]))?
            } else {
                call("eth_getBlockByHash", json!([id, true]))?
            }
        }
    };
    if !raw.is_object() {
        return Err("block not found".into());
    }
    pack_block(&raw, page)
}


pub(crate) fn live_head_number(fallback: u64) -> u64 {
    call("eth_blockNumber", json!([]))
        .ok()
        .map(|v| hex_u64(&v))
        .or_else(|| last_head().and_then(|h| h.get("block").and_then(|v| v.as_u64())))
        .unwrap_or(fallback)
}


pub(crate) fn pack_block(raw: &Value, page: u64) -> Result<Value, String> {
    let n = hex_u64(raw.get("number").unwrap_or(&Value::Null));
    let gas_used = hex_u64(raw.get("gasUsed").unwrap_or(&Value::Null));
    let gas_limit = hex_u64(raw.get("gasLimit").unwrap_or(&Value::Null)).max(1);
    let tx_count = raw
        .get("transactions")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let page = page.max(1);
    let pages = (tx_count.max(1) + BLOCK_TX_PAGE - 1) / BLOCK_TX_PAGE;
    let start = ((page as usize) - 1).saturating_mul(BLOCK_TX_PAGE);
    let txs = txs_from_block_range(raw, n, start, BLOCK_TX_PAGE);
    let head_n = live_head_number(n);
    Ok(json!({
        "ok": true,
        "number": n,
        "hash": str_of(raw, "hash"),
        "parent": str_of(raw, "parentHash"),
        "miner": str_of(raw, "miner"),
        "ts": hex_u64(raw.get("timestamp").unwrap_or(&Value::Null)),
        "gas_used": gas_used,
        "gas_limit": gas_limit,
        "load": (gas_used as f64 / gas_limit as f64).clamp(0.0, 1.0),
        "base_fee": fmt_gwei(raw.get("baseFeePerGas").unwrap_or(&Value::Null)),
        "l1": opt_u64(raw.get("l1BlockNumber")),
        "size": hex_u64(raw.get("size").unwrap_or(&Value::Null)),
        "extra": str_of(raw, "extraData"),
        "state_root": str_of(raw, "stateRoot"),
        "tx_root": str_of(raw, "transactionsRoot"),
        "receipts_root": str_of(raw, "receiptsRoot"),
        "txs": txs,
        "tx_count": tx_count,
        "page": page,
        "pages": pages,
        "head": head_n,
        "latest": n >= head_n,
        "now": now_secs(),
    }))
}


pub(crate) fn summarize_block(raw: &Value, full: bool) -> Value {
    let txs = raw.get("transactions").and_then(|v| v.as_array());
    let count = txs.map(|a| a.len()).unwrap_or(0);
    let lim = hex_u64(raw.get("gasLimit").unwrap_or(&Value::Null)).max(1);
    let used = hex_u64(raw.get("gasUsed").unwrap_or(&Value::Null));
    let load = (used as f64 / lim as f64).clamp(0.0, 1.0);
    json!({
        "number": hex_u64(raw.get("number").unwrap_or(&Value::Null)),
        "hash": str_of(raw, "hash"),
        "ts": hex_u64(raw.get("timestamp").unwrap_or(&Value::Null)),
        "txs": count,
        "gas_used": used,
        "gas_limit": lim,
        "l1": opt_u64(raw.get("l1BlockNumber")),
        "miner": str_of(raw, "miner"),
        "base_fee": fmt_gwei(raw.get("baseFeePerGas").unwrap_or(&Value::Null)),
        "base_fee_n": hex_u128_val(raw.get("baseFeePerGas").unwrap_or(&Value::Null)) as f64 / 1e9,
        "load": load,
        "full": full,
    })
}

