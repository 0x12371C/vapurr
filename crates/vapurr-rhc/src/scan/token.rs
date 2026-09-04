use super::*;


pub(crate) fn list_tokens(cursor: Option<&str>) -> Result<Value, String> {
    crate::index::kick();
    let first = cursor.map(str::trim).filter(|s| !s.is_empty()).is_none();
    if !first {
        return match crate::index::tokens(cursor) {
            Ok(page) => Ok(index_page(
                "tokens",
                Ok(crate::index::Page {
                    items: merge_token_rows(pin_usdg_first(page.items, false)),
                    next: page.next,
                }),
            )),
            Err(_) => Ok(index_page("tokens", Err("index wait".into()))),
        };
    }
    if let Some(page) = crate::index::tokens_if_ready() {
        if !page.items.is_empty() {
            return Ok(index_page(
                "tokens",
                Ok(crate::index::Page {
                    items: merge_token_rows(
                        pin_usdg_first(page.items, true).into_iter().take(48).collect(),
                    ),
                    next: page.next,
                }),
            ));
        }
    }
    if let Some(rows) = crate::liq::token_list() {
        let mut v = index_page("tokens", Err("index wait".into()));
        if let Some(obj) = v.as_object_mut() {
            obj.insert(
                "tokens".into(),
                json!(merge_token_rows(
                    pin_usdg_first(rows, true).into_iter().take(48).collect()
                )),
            );
            obj.insert("source".into(), json!("rhc-liq"));
            obj.insert("index".into(), json!(false));
            obj.remove("error");
        }
        return Ok(v);
    }
    Ok(index_page("tokens", Err("index wait".into())))
}


fn merge_token_rows(mut items: Vec<Value>) -> Vec<Value> {
    let census = crate::index::tokens_if_ready();
    for t in &mut items {
        let Some(obj) = t.as_object_mut() else {
            continue;
        };
        let addr = obj
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if addr.is_empty() {
            continue;
        }
        if let Some(hit) = crate::liq::token_hit(&addr) {
            for k in ["price_usd", "tvl_usd", "vol24_usd", "degree", "change24"] {
                let blank = match obj.get(k) {
                    None => true,
                    Some(Value::Null) => true,
                    Some(v) => v.as_f64() == Some(0.0),
                };
                if blank {
                    if let Some(v) = hit.get(k) {
                        obj.insert(k.into(), v.clone());
                    }
                }
            }
        }
        if obj.get("holders").and_then(|v| v.as_u64()).unwrap_or(0) == 0 {
            if let Some(page) = &census {
                if let Some(n) = page.items.iter().find_map(|x| {
                    let a = x.get("address")?.as_str()?;
                    if !a.eq_ignore_ascii_case(&addr) {
                        return None;
                    }
                    x.get("holders").and_then(|v| v.as_u64()).filter(|n| *n > 0)
                }) {
                    obj.insert("holders".into(), json!(n));
                }
            }
        }
    }
    items
}

pub(crate) fn pin_usdg_first(mut items: Vec<Value>, insert: bool) -> Vec<Value> {
    fn is_usdg(t: &Value) -> bool {
        t.get("usdg").and_then(|v| v.as_bool()).unwrap_or(false)
            || t.get("address")
                .and_then(|v| v.as_str())
                .map(|a| a.eq_ignore_ascii_case(USDG))
                .unwrap_or(false)
            || t.get("token")
                .and_then(|v| v.as_str())
                .map(|a| a.eq_ignore_ascii_case(USDG))
                .unwrap_or(false)
    }
    items.sort_by(|a, b| is_usdg(b).cmp(&is_usdg(a)));
    if insert && !items.iter().any(is_usdg) {
        items.insert(
            0,
            json!({
                "address": USDG,
                "name": "USDG",
                "symbol": "USDG",
                "type": "ERC-20",
                "decimals": USDG_DECIMALS,
                "supply": "",
                "usdg": true,
            }),
        );
    }
    items
}


pub(crate) fn token_api(raw: &str, xfer: Option<&str>, holders: Option<&str>) -> Result<Value, String> {
    let a = match classify(raw) {
        Query::Address(x) => x,
        _ => raw.trim().to_ascii_lowercase(),
    };
    crate::index::kick();
    crate::index::kick_token(&a, xfer, holders);
    let usdg = a.eq_ignore_ascii_case(USDG);
    let mut t = if let Some(ix) = crate::index::token_if_ready(&a, xfer, holders) {
        ix
    } else {
        json!({
            "ok": true,
            "address": a,
            "name": if usdg { json!("USDG") } else { json!("") },
            "symbol": if usdg { json!("USDG") } else { json!("") },
            "type": "ERC-20",
            "decimals": if usdg { json!(USDG_DECIMALS) } else { json!(0) },
            "supply": "",
            "usdg": usdg,
            "source": "rpc",
            "index": false,
            "loading": true,
            "degraded": true,
            "transfers": [],
            "holder_list": [],
        })
    };
    if t.get("index").and_then(|v| v.as_bool()) != Some(true) {
        if let Some(page) = crate::index::tokens_if_ready() {
            if let Some(hit) = page.items.iter().find(|x| {
                x.get("address")
                    .and_then(|v| v.as_str())
                    .map(|s| s.eq_ignore_ascii_case(&a))
                    .unwrap_or(false)
            }) {
                if let Some(obj) = t.as_object_mut() {
                    for k in ["name", "symbol", "type", "decimals", "supply", "usdg"] {
                        if let Some(v) = hit.get(k) {
                            if v.is_null() {
                                continue;
                            }
                            if v.as_str() == Some("") {
                                continue;
                            }
                            obj.insert(k.into(), v.clone());
                        }
                    }
                    if let Some(n) = hit.get("holders").and_then(|v| v.as_u64()).filter(|n| *n > 0) {
                        obj.insert("holders".into(), json!(n));
                    }
                    obj.insert("source".into(), json!("index"));
                    obj.insert("index".into(), json!(true));
                    obj.insert("degraded".into(), json!(false));
                }
            }
        }
    }
    Ok(overlay_liq_token(t, &a))
}


pub(crate) fn overlay_liq_token(mut t: Value, addr: &str) -> Value {
    let Some(hit) = crate::liq::token_hit(addr) else {
        return t;
    };
    if let Some(obj) = t.as_object_mut() {
        for k in ["price_usd", "tvl_usd", "vol24_usd", "mcap_usd", "degree", "change24"] {
            if let Some(v) = hit.get(k) {
                obj.insert(k.into(), v.clone());
            }
        }
        let blank = obj
            .get("symbol")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .is_empty();
        if blank {
            if let Some(v) = hit.get("symbol") {
                obj.insert("symbol".into(), v.clone());
            }
        }
        obj.insert("liq".into(), json!(true));
        if let Some(pools) = crate::liq::pools_for(addr) {
            obj.insert("pools".into(), json!(pools));
        }
    }
    t
}


pub(crate) fn holders_api(addr: &str, cursor: Option<&str>) -> Result<Value, String> {
    crate::index::kick();
    crate::index::kick_token(addr, None, cursor);
    if let Some(t) = crate::index::token_if_ready(addr, None, cursor)
        .or_else(|| crate::index::token_if_ready(addr, None, None))
    {
        return Ok(json!({
            "ok": true,
            "holders": t.get("holder_list").cloned().unwrap_or_else(|| json!([])),
            "next": t.get("holders_next").cloned().unwrap_or(Value::Null),
            "count": t.get("holders").cloned().unwrap_or(Value::Null),
            "now": now_secs(),
        }));
    }
    Ok(json!({
        "ok": true,
        "holders": [],
        "loading": true,
        "error": "index wait",
        "now": now_secs(),
    }))
}


#[allow(dead_code)]
pub(crate) fn token_from_rpc(addr: &str, xfer: Option<&str>) -> Option<Value> {
    let a = addr.trim();
    let code = call("eth_getCode", json!([a, "latest"])).ok()?;
    let cs = code.as_str().unwrap_or("0x");
    let has_code = cs.len() > 4 && cs != "0x" && cs != "0x0";
    if !has_code && !a.eq_ignore_ascii_case(USDG) {
        return None;
    }
    let meta = resolve_token_meta(a);
    if meta.is_none() && !a.eq_ignore_ascii_case(USDG) {
        return None;
    }
    let (dec, mut sym) = meta.unwrap_or((USDG_DECIMALS, "USDG".into()));
    if sym.is_empty() && a.eq_ignore_ascii_case(USDG) {
        sym = "USDG".into();
    }
    let name_v = call(
        "eth_call",
        json!([{ "to": a, "data": "0x06fdde03" }, "latest"]),
    )
    .ok();
    let mut name = name_v
        .as_ref()
        .and_then(|v| v.as_str())
        .and_then(decode_abi_string)
        .unwrap_or_default();
    if name.is_empty() && a.eq_ignore_ascii_case(USDG) {
        name = "USDG".into();
    }
    let supply_v = call(
        "eth_call",
        json!([{ "to": a, "data": "0x18160ddd" }, "latest"]),
    )
    .ok();
    let supply_hex = supply_v
        .as_ref()
        .and_then(|v| v.as_str())
        .unwrap_or("0x0");
    let usdg = a.eq_ignore_ascii_case(USDG);
    let (transfers, next) = rpc_token_transfers(a, xfer);
    Some(json!({
        "ok": true,
        "address": a,
        "name": name,
        "symbol": sym,
        "type": "ERC-20",
        "decimals": dec,
        "supply": fmt_fixed(hex_u128(supply_hex), dec, &sym),
        "usdg": usdg,
        "erc20": true,
        "source": "rpc",
        "index": false,
        "transfers": transfers,
        "transfers_next": next,
        "holder_list": [],
    }))
}


#[allow(dead_code)]
pub(crate) fn local_usdg_token(xfer: Option<&str>) -> Value {
    let (transfers, next) = rpc_token_transfers(USDG, xfer);
    json!({
        "ok": true,
        "address": USDG,
        "name": "USDG",
        "symbol": "USDG",
        "type": "ERC-20",
        "decimals": USDG_DECIMALS,
        "supply": "",
        "usdg": true,
        "source": "rpc",
        "index": false,
        "transfers": transfers,
        "transfers_next": next,
        "holder_list": [],
    })
}


#[allow(dead_code)]
pub(crate) fn rpc_token_transfers(addr: &str, cursor: Option<&str>) -> (Vec<Value>, Option<Value>) {
    let n = match call("eth_blockNumber", json!([])) {
        Ok(v) => hex_u64(&v),
        Err(_) => last_head()
            .and_then(|h| h.get("block").and_then(|x| x.as_u64()))
            .unwrap_or(0),
    };
    if n == 0 {
        return (Vec::new(), None);
    }
    let rpc_cur = if is_rpc_cursor(cursor) { cursor } else { None };
    let (from, to_blk, next) = log_window(n, rpc_cur);
    let logs = call(
        "eth_getLogs",
        json!([{
            "fromBlock": hex_n(from),
            "toBlock": hex_n(to_blk),
            "address": addr,
            "topics": [TRANSFER_TOPIC]
        }]),
    )
    .ok();
    let mut xfer = Vec::new();
    if let Some(arr) = logs.as_ref().and_then(|v| v.as_array()) {
        let metas = logs_metas(arr);
        for log in arr {
            xfer.push(decode_log(log, &metas));
        }
        xfer.sort_by_key(|v| {
            std::cmp::Reverse((
                v.get("block").and_then(|x| x.as_u64()).unwrap_or(0),
                v.get("index").and_then(|x| x.as_u64()).unwrap_or(0),
            ))
        });
    }
    (xfer, next)
}

