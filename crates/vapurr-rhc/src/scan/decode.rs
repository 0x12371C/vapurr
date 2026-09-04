use super::*;


pub(crate) fn decode_logs(v: &Value) -> Vec<Value> {
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };
    let metas = logs_metas(arr);
    arr.iter().map(|log| decode_log(log, &metas)).collect()
}


pub(crate) fn logs_metas(logs: &[Value]) -> HashMap<String, (u8, String)> {
    let mut seen = Vec::new();
    for log in logs {
        let a = str_of(log, "address").to_ascii_lowercase();
        if a.len() == 42 && !seen.contains(&a) {
            seen.push(a);
        }
    }
    let mut map = HashMap::new();
    for a in seen {
        if let Some(m) = resolve_token_meta(&a) {
            map.insert(a, m);
        }
    }
    map
}


pub(crate) fn decode_log(log: &Value, metas: &HashMap<String, (u8, String)>) -> Value {
    let topics: Vec<String> = log
        .get("topics")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(|s| s.to_ascii_lowercase()))
                .collect()
        })
        .unwrap_or_default();
    let topic0 = topics.first().cloned().unwrap_or_default();
    let emitter = str_of(log, "address");
    let key = emitter.to_ascii_lowercase();
    let usdg = emitter.eq_ignore_ascii_case(USDG);
    let data = str_of(log, "data");
    let mut out = json!({
        "tx": str_of(log, "transactionHash"),
        "block": opt_u64(log.get("blockNumber")),
        "index": hex_u64(log.get("logIndex").unwrap_or(&Value::Null)),
        "address": emitter,
        "token": emitter,
        "usdg": usdg,
        "topics": topics,
        "data": data,
        "event": event_name(&topic0),
    });
    let obj = out.as_object_mut().unwrap();
    if topic0 == TRANSFER_TOPIC {
        let from = topics.get(1).map(|s| unpad_addr(s));
        let to = topics.get(2).map(|s| unpad_addr(s));
        let nft = topics.len() >= 4;
        let amount = if nft {
            json!(format!("#{}", hex_u128(topics.get(3).map(|s| s.as_str()).unwrap_or("0x0"))))
        } else {
            json!(fmt_transfer_amount(&data, &key, usdg, metas.get(&key)))
        };
        obj.insert("from".into(), json!(from));
        obj.insert("to".into(), json!(to));
        obj.insert("amount".into(), amount.clone());
        obj.insert("nft".into(), json!(nft));
        obj.insert(
            "kind".into(),
            json!(if nft { "ERC-721" } else { "ERC-20" }),
        );
        if nft {
            obj.insert("token_id".into(), amount.clone());
        }
        obj.insert(
            "decoded".into(),
            json!({
                "name": "Transfer",
                "params": [
                    {"name": "from", "type": "address", "value": from},
                    {"name": "to", "type": "address", "value": to},
                    {"name": if nft { "tokenId" } else { "amount" }, "type": "uint256", "value": amount}
                ]
            }),
        );
    } else if topic0 == APPROVAL_TOPIC {
        let owner = topics.get(1).map(|s| unpad_addr(s));
        let spender = topics.get(2).map(|s| unpad_addr(s));
        obj.insert("from".into(), json!(owner));
        obj.insert("spender".into(), json!(spender));
        obj.insert(
            "amount".into(),
            json!(fmt_transfer_amount(&data, &key, usdg, metas.get(&key))),
        );
        obj.insert(
            "decoded".into(),
            json!({
                "name": "Approval",
                "params": [
                    {"name": "owner", "type": "address", "value": owner},
                    {"name": "spender", "type": "address", "value": spender},
                    {"name": "amount", "type": "uint256", "value": fmt_transfer_amount(&data, &key, usdg, metas.get(&key))}
                ]
            }),
        );
    }
    out
}


pub(crate) fn event_name(topic0: &str) -> &'static str {
    if topic0.eq_ignore_ascii_case(TRANSFER_TOPIC) {
        "Transfer"
    } else if topic0.eq_ignore_ascii_case(APPROVAL_TOPIC) {
        "Approval"
    } else {
        "log"
    }
}


pub(crate) fn fmt_transfer_amount(
    data: &str,
    token: &str,
    usdg: bool,
    meta: Option<&(u8, String)>,
) -> String {
    if usdg || token.eq_ignore_ascii_case(USDG) {
        return fmt_token(data, USDG_DECIMALS, "USDG");
    }
    if let Some((dec, sym)) = meta {
        let unit = if sym.is_empty() { "" } else { sym.as_str() };
        return fmt_fixed(hex_u128(data), *dec, unit);
    }
    fmt_raw_amount(data)
}


pub(crate) fn fmt_raw_amount(s: &str) -> String {
    let n = hex_u128(s);
    if n == 0 {
        let t = s.trim();
        if t.is_empty() || t == "0x" || t == "0x0" {
            return "0".into();
        }
        return t.to_string();
    }
    n.to_string()
}


pub(crate) fn resolve_token_meta(addr: &str) -> Option<(u8, String)> {
    let key = addr.to_ascii_lowercase();
    if key.eq_ignore_ascii_case(USDG) {
        return Some((USDG_DECIMALS, "USDG".into()));
    }
    if let Ok(g) = TOKEN_META.lock() {
        if let Some(map) = g.as_ref() {
            if let Some(hit) = map.get(&key) {
                return hit.clone();
            }
        }
    }
    let got = fetch_token_meta(&key);
    if let Ok(mut g) = TOKEN_META.lock() {
        g.get_or_insert_with(HashMap::new).insert(key, got.clone());
    }
    got
}


pub(crate) fn fetch_token_meta(addr: &str) -> Option<(u8, String)> {
    let dec_v = call(
        "eth_call",
        json!([{ "to": addr, "data": "0x313ce567" }, "latest"]),
    )
    .ok()?;
    let dec_s = dec_v.as_str().unwrap_or("0x");
    if dec_s.len() <= 2 {
        return None;
    }
    let dec = hex_u128(dec_s).min(18) as u8;
    let sym_v = call(
        "eth_call",
        json!([{ "to": addr, "data": "0x95d89b41" }, "latest"]),
    )
    .ok();
    let sym = sym_v
        .as_ref()
        .and_then(|v| v.as_str())
        .and_then(decode_abi_string)
        .unwrap_or_default();
    if dec == 0 && sym.is_empty() {
        return None;
    }
    Some((dec, sym))
}


pub(crate) fn decode_abi_string(hex: &str) -> Option<String> {
    let h = hex.trim().trim_start_matches("0x").trim_start_matches("0X");
    if h.is_empty() {
        return None;
    }
    if h.len() == 64 {
        let bytes = hex_to_bytes(h)?;
        let s: Vec<u8> = bytes.into_iter().take_while(|b| *b != 0).collect();
        let t = String::from_utf8(s).ok()?.trim().to_string();
        return if t.is_empty() { None } else { Some(t) };
    }
    if h.len() < 128 {
        return None;
    }
    let len = u64::from_str_radix(&h[64..128], 16).ok()? as usize;
    if len == 0 || len > 64 {
        return None;
    }
    let start: usize = 128;
    let end = start.saturating_add(len.saturating_mul(2));
    if h.len() < end {
        return None;
    }
    let bytes = hex_to_bytes(&h[start..end])?;
    let t = String::from_utf8(bytes).ok()?.trim().to_string();
    if t.is_empty() { None } else { Some(t) }
}


pub(crate) fn hex_to_bytes(h: &str) -> Option<Vec<u8>> {
    if h.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(h.len() / 2);
    let b = h.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        let s = std::str::from_utf8(&b[i..i + 2]).ok()?;
        out.push(u8::from_str_radix(s, 16).ok()?);
        i += 2;
    }
    Some(out)
}


pub(crate) fn decode_input(input: &str, token: &str) -> Value {
    let input = input.trim();
    if input.len() < 10 {
        return Value::Null;
    }
    let sel = input.get(..10).unwrap_or("").to_ascii_lowercase();
    if sel == "0x" || sel == "0x0" {
        return Value::Null;
    }
    let data = input.get(10..).unwrap_or("");
    let data = data.trim_start_matches("0x");
    let mut words = Vec::new();
    let mut i = 0;
    while i + 64 <= data.len() {
        words.push(format!("0x{}", &data[i..i + 64]));
        i += 64;
    }
    let leftover = if i < data.len() {
        Some(format!("0x{}", &data[i..]))
    } else {
        None
    };
    let method = method_name_val(input, 0);
    let args = match sel.as_str() {
        "0xa9059cbb" if words.len() >= 2 => json!([
            {"name": "to", "kind": "address", "type": "address", "value": unpad_addr(&words[0])},
            {"name": "amount", "kind": "uint256", "type": "uint256", "value": word_amt(&words[1], token)},
        ]),
        "0x095ea7b3" if words.len() >= 2 => json!([
            {"name": "spender", "kind": "address", "type": "address", "value": unpad_addr(&words[0])},
            {"name": "amount", "kind": "uint256", "type": "uint256", "value": word_amt(&words[1], token)},
        ]),
        "0x23b872dd" if words.len() >= 3 => json!([
            {"name": "from", "kind": "address", "type": "address", "value": unpad_addr(&words[0])},
            {"name": "to", "kind": "address", "type": "address", "value": unpad_addr(&words[1])},
            {"name": "amount", "kind": "uint256", "type": "uint256", "value": word_amt(&words[2], token)},
        ]),
        "0x70a08231" if !words.is_empty() => json!([
            {"name": "account", "kind": "address", "type": "address", "value": unpad_addr(&words[0])},
        ]),
        _ => json!([]),
    };
    json!({
        "selector": sel,
        "name": method.clone(),
        "method": method,
        "params": args.clone(),
        "args": args,
        "words": words,
        "leftover": leftover,
    })
}


pub(crate) fn word_amt(word: &str, token: &str) -> String {
    let key = token.to_ascii_lowercase();
    let usdg = key.eq_ignore_ascii_case(USDG);
    let meta = if key.len() == 42 {
        resolve_token_meta(&key)
    } else {
        None
    };
    fmt_transfer_amount(word, &key, usdg, meta.as_ref())
}


pub(crate) fn amountish_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("amount")
        || n.contains("value")
        || n == "wad"
        || n == "assets"
        || n == "shares"
        || n == "delta"
}


pub(crate) fn humanize_decoded(mut decoded: Value, token: &str) -> Value {
    if decoded.is_null() {
        return decoded;
    }
    for key in ["params", "args"] {
        let Some(arr) = decoded.get_mut(key).and_then(|v| v.as_array_mut()) else {
            continue;
        };
        for p in arr {
            let ty = p.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if !(ty.starts_with("uint") && amountish_name(&name)) {
                continue;
            }
            let Some(val) = p.get("value") else {
                continue;
            };
            let raw = match val {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => continue,
            };
            if raw.contains(' ') {
                continue;
            }
            let word = if raw.starts_with("0x") || raw.starts_with("0X") {
                raw
            } else if let Ok(n) = raw.parse::<u128>() {
                format!("0x{n:x}")
            } else {
                continue;
            };
            if let Some(obj) = p.as_object_mut() {
                obj.insert("value".into(), json!(word_amt(&word, token)));
            }
        }
    }
    decoded
}


pub(crate) fn merge_decoded(rpc: Value, index: Option<Value>, token: &str) -> Value {
    let Some(ix) = index.filter(|v| v.is_object()) else {
        return rpc;
    };
    let ix = humanize_decoded(ix, token);
    let rpc_empty = rpc
        .get("params")
        .and_then(|v| v.as_array())
        .map(|a| a.is_empty())
        .unwrap_or(true);
    let ix_params = ix
        .get("params")
        .and_then(|v| v.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    if rpc.is_null() || (rpc_empty && ix_params) {
        return ix;
    }
    let mut out = rpc;
    if let Some(obj) = out.as_object_mut() {
        if let Some(name) = ix.get("name").and_then(|v| v.as_str()) {
            if !name.is_empty() && !name.starts_with("0x") {
                obj.insert("name".into(), json!(name));
                obj.insert("method".into(), json!(name));
            }
        }
        if rpc_empty {
            if let Some(p) = ix.get("params") {
                obj.insert("params".into(), p.clone());
                obj.insert("args".into(), p.clone());
            }
        }
    }
    out
}


pub(crate) fn overlay_decoded_logs(logs: &mut [Value], ix: &[Value]) {
    for lg in logs.iter_mut() {
        let idx = lg.get("index").and_then(|v| v.as_u64());
        let Some(hit) = ix
            .iter()
            .find(|x| x.get("index").and_then(|v| v.as_u64()) == idx)
        else {
            continue;
        };
        let token = lg
            .get("address")
            .and_then(|v| v.as_str())
            .or_else(|| lg.get("token").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        let rpc_amt = lg
            .get("amount")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let rpc_dec = lg.get("decoded").cloned().unwrap_or(Value::Null);
        if let Some(d) = hit.get("decoded").cloned().filter(|v| !v.is_null()) {
            let merged = merge_decoded(rpc_dec, Some(d), &token);
            if let Some(obj) = lg.as_object_mut() {
                if let Some(name) = merged.get("name").and_then(|v| v.as_str()) {
                    if !name.is_empty() && !name.starts_with("0x") {
                        obj.insert("event".into(), json!(name));
                    }
                }
                obj.insert("decoded".into(), merged);
            }
        }
        if let Some(obj) = lg.as_object_mut() {
            let ix_amt = hit.get("amount").and_then(|v| v.as_str()).unwrap_or("");
            if rpc_amt.contains(' ') {
                obj.insert("amount".into(), json!(rpc_amt));
            } else if ix_amt.contains(' ') {
                obj.insert("amount".into(), json!(ix_amt));
            }
            if obj
                .get("topics")
                .and_then(|v| v.as_array())
                .map(|a| a.is_empty())
                .unwrap_or(true)
            {
                if let Some(t) = hit.get("topics") {
                    obj.insert("topics".into(), t.clone());
                }
            }
            if obj.get("data").and_then(|v| v.as_str()).unwrap_or("").len() <= 2 {
                if let Some(data) = hit.get("data") {
                    obj.insert("data".into(), data.clone());
                }
            }
        }
    }
}

