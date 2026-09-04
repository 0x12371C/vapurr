use super::*;


pub(crate) fn native_usd() -> Option<f64> {
    crate::liq::token_hit(WETH)
        .and_then(|t| t.get("price_usd").and_then(|v| v.as_f64()))
        .and_then(crate::liq::sane_eth_px)
}


pub(crate) fn fmt_usd_f(n: f64) -> String {
    if !n.is_finite() || n <= 0.0 {
        return String::new();
    }
    if n >= 1.0 {
        format!("${n:.2}")
    } else if n >= 0.01 {
        format!("${n:.4}")
    } else {
        format!("${n:.6}")
    }
}


pub(crate) fn usd_from_wei(wei: u128, px: Option<f64>) -> Value {
    let Some(px) = px else {
        return Value::Null;
    };
    let eth = wei as f64 / 1e18;
    let usd = eth * px;
    let s = fmt_usd_f(usd);
    if s.is_empty() {
        Value::Null
    } else {
        json!(s)
    }
}


pub(crate) fn price_holdings(tokens: &mut [Value]) {
    for t in tokens {
        let Some(obj) = t.as_object_mut() else {
            continue;
        };
        let usdg = obj.get("usdg").and_then(|v| v.as_bool()).unwrap_or(false);
        let addr = obj
            .get("token")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("address").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        let px = if usdg || addr.eq_ignore_ascii_case(USDG) {
            Some(1.0)
        } else {
            crate::liq::token_hit(&addr)
                .and_then(|h| h.get("price_usd").and_then(|v| v.as_f64()))
                .filter(|p| *p > 0.0)
        };
        let Some(px) = px else {
            continue;
        };
        obj.entry("price_usd".to_string()).or_insert(json!(px));
        let amt = obj
            .get("amount")
            .and_then(|v| v.as_str())
            .map(parse_qty)
            .unwrap_or(0.0);
        if amt > 0.0 {
            let usd = fmt_usd_f(amt * px);
            if !usd.is_empty() {
                obj.insert("usd".into(), json!(usd));
            }
        }
    }
}


pub(crate) fn addr(raw: &str, tab: Option<&str>, cursor: Option<&str>) -> Result<Value, String> {
    let a = match classify(raw) {
        Query::Address(x) => x,
        _ => {
            let s = raw.trim();
            if s.len() == 42 && s.starts_with("0x") {
                s.to_ascii_lowercase()
            } else {
                return Err("bad address".into());
            }
        }
    };
    let n = hex_u64(&call("eth_blockNumber", json!([]))?);
    let idx_cursor = if is_rpc_cursor(cursor) { None } else { cursor };
    let reqs = vec![
        json!({"jsonrpc":"2.0","id":1,"method":"eth_getBalance","params":[a.clone(),"latest"]}),
        json!({"jsonrpc":"2.0","id":2,"method":"eth_getTransactionCount","params":[a.clone(),"latest"]}),
        json!({"jsonrpc":"2.0","id":3,"method":"eth_getCode","params":[a.clone(),"latest"]}),
        json!({"jsonrpc":"2.0","id":4,"method":"eth_call","params":[{"to": USDG, "data": balance_of_data(&a)}, "latest"]}),
    ];
    let parts = batch(reqs)?;
    let bal = parts.first().cloned().unwrap_or(Value::Null);
    let nonce = parts.get(1).cloned().unwrap_or(Value::Null);
    let code = parts.get(2).and_then(|v| v.as_str()).unwrap_or("0x").to_string();
    let usdg = parts.get(3).and_then(|v| v.as_str()).unwrap_or("0x0");
    let contract = code.len() > 4 && code != "0x" && code != "0x0";
    let out = json!({
        "ok": true,
        "address": a,
        "contract": contract,
        "eth": fmt_eth_hex(str_val(&bal).as_str()),
        "usdg": fmt_token(usdg, USDG_DECIMALS, "USDG"),
        "nonce": hex_u64(&nonce),
        "code": if contract { json!(code) } else { Value::Null },
        "code_bytes": if contract { (code.len().saturating_sub(2)) / 2 } else { 0 },
        "transfers": [],
        "events": [],
        "span": LOG_SPAN,
        "head": n,
        "now": now_secs(),
    });
    Ok(enrich_addr(a, out, tab, idx_cursor))
}


pub(crate) fn enrich_addr(
    addr: String,
    mut out: Value,
    tab: Option<&str>,
    cursor: Option<&str>,
) -> Value {
    crate::index::kick_addr(&addr, tab, cursor);
    let contract = out.get("contract").and_then(|v| v.as_bool()).unwrap_or(false);
    if contract {
        crate::index::kick_contract(&addr);
    }
    let usdg_amt = out
        .get("usdg")
        .and_then(|v| v.as_str())
        .unwrap_or("0 USDG")
        .to_string();
    let mut indexed = false;
    if let Some(b) = crate::index::addr_bundle_if_ready(&addr, tab, cursor) {
        indexed = true;
        if let Some(obj) = out.as_object_mut() {
            if let Some(info) = b.get("info") {
                obj.insert("name".into(), info.get("name").cloned().unwrap_or(json!("")));
                obj.insert(
                    "verified".into(),
                    info.get("verified").cloned().unwrap_or(json!(false)),
                );
                obj.insert("ens".into(), info.get("ens").cloned().unwrap_or(json!("")));
                obj.insert("creator".into(), info.get("creator").cloned().unwrap_or(json!("")));
                obj.insert(
                    "creation_tx".into(),
                    info.get("creation_tx").cloned().unwrap_or(json!("")),
                );
                if info.get("contract").and_then(|v| v.as_bool()).unwrap_or(false) {
                    obj.insert("contract".into(), json!(true));
                }
            }
            let take_page = |obj: &mut serde_json::Map<String, Value>, src: &str, items_key: &str, next_key: &str| {
                if let Some(page) = b.get(src) {
                    if let Some(items) = page.get("items") {
                        obj.insert(items_key.into(), items.clone());
                    }
                    obj.insert(next_key.into(), page.get("next").cloned().unwrap_or(Value::Null));
                }
            };
            take_page(obj, "txs", "txs", "txs_next");
            take_page(obj, "xfers", "token_transfers", "xfers_next");
            take_page(obj, "internal", "internal", "internal_next");
            take_page(obj, "events", "events", "events_next");
            if let Some(page) = b.get("tokens") {
                let items = page
                    .get("items")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                obj.insert("tokens".into(), json!(pin_usdg_first(items, false)));
                obj.insert("tokens_next".into(), page.get("next").cloned().unwrap_or(Value::Null));
            }
        }
    }
    if let Some(c) = crate::index::contract_if_ready(&addr) {
        if let Some(obj) = out.as_object_mut() {
            for k in [
                "source",
                "abi",
                "compiler",
                "optimization",
                "optimization_runs",
                "language",
                "license",
                "proxy",
                "implementation",
            ] {
                if let Some(v) = c.get(k) {
                    obj.insert(k.into(), v.clone());
                }
            }
            if c.get("verified").and_then(|v| v.as_bool()).unwrap_or(false) {
                obj.insert("verified".into(), json!(true));
            }
            if let Some(n) = c.get("name").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                if obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .is_empty()
                {
                    obj.insert("name".into(), json!(n));
                }
            }
        }
    }
    if let Some(obj) = out.as_object_mut() {
        let tokens_empty = obj
            .get("tokens")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true);
        if tokens_empty && parse_qty(&usdg_amt) > 0.0 {
            obj.insert(
                "tokens".into(),
                json!([{
                    "token": USDG,
                    "address": USDG,
                    "name": "USDG",
                    "symbol": "USDG",
                    "usdg": true,
                    "amount": usdg_amt,
                }]),
            );
        }
        if let Some(tokens) = obj.get_mut("tokens").and_then(|v| v.as_array_mut()) {
            price_holdings(tokens);
        }
        obj.insert("indexed".into(), json!(indexed));
        obj.insert("loading".into(), json!(!indexed));
        obj.insert("source".into(), json!(if indexed { "index" } else { "rpc" }));
        if !indexed {
            obj.entry("error".to_string())
                .or_insert_with(|| json!("index wait"));
        }
    }
    out
}

