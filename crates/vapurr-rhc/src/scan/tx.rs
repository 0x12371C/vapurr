use super::*;


pub(crate) fn list_txs(cursor: Option<&str>) -> Result<Value, String> {
    crate::index::kick();
    let paged = cursor.map(str::trim).filter(|s| !s.is_empty()).is_some();
    if paged {
        return Ok(index_page("transactions", crate::index::latest_txs(cursor)));
    }
    if let Some(page) = crate::index::latest_txs_if_ready() {
        return Ok(index_page("transactions", Ok(page)));
    }
    let mut v = index_page("transactions", Err("index wait".into()));
    if let Some(obj) = v.as_object_mut() {
        obj.insert("transactions".into(), json!(rpc_window_txs()));
    }
    Ok(v)
}


pub(crate) fn tx(hash: &str) -> Result<Value, String> {
    let h = match classify(hash) {
        Query::Hash(x) => x,
        _ => {
            let s = hash.trim().to_ascii_lowercase();
            let s = if s.starts_with("0x") {
                s
            } else {
                format!("0x{s}")
            };
            match classify(&s) {
                Query::Hash(x) => x,
                _ => return Err("bad hash".into()),
            }
        }
    };
    let t = match call("eth_getTransactionByHash", json!([h.clone()])) {
        Ok(v) if v.is_object() => v,
        Ok(_) | Err(_) => {
            if let Some(cached) = tx_from_head_cache(&h) {
                return Ok(cached);
            }
            return Err(format!("tx not found ({h})"));
        }
    };
    let receipt = call("eth_getTransactionReceipt", json!([h.clone()])).unwrap_or(Value::Null);
    let pending = !receipt.is_object();
    let status = if receipt.is_object() {
        hex_u64(receipt.get("status").unwrap_or(&Value::Null))
    } else {
        2
    };
    let mut logs = decode_logs(receipt.get("logs").unwrap_or(&Value::Null));
    let input = str_of(&t, "input");
    let to = str_of(&t, "to");
    let contract = str_of(&receipt, "contractAddress");
    let value_wei = hex_u128(str_of(&t, "value").as_str());
    let mut decoded = humanize_decoded(decode_input(&input, &to), &to);
    let mut method = method_name_val(&input, value_wei);
    let mut l1 = opt_u64(receipt.get("l1BlockNumber"));
    let mut l1_fee = l1_fee_from_receipt(&receipt);
    let mut l1_gas = l1_gas_from_receipt(&receipt);
    let mut l1_gas_price = l1_gas_price_from_receipt(&receipt);
    let mut revert = Value::Null;
    let block_hdr = match t.get("blockNumber") {
        Some(Value::Null) | None => None,
        Some(bn) => call("eth_getBlockByNumber", json!([bn.clone(), false])).ok(),
    };
    if l1.is_null() {
        if let Some(b) = &block_hdr {
            l1 = opt_u64(b.get("l1BlockNumber"));
        }
    }
    crate::index::kick_tx(&h);
    let mut index_loading = true;
    if let Some(ov) = crate::index::tx_overlay_if_ready(&h) {
        index_loading = false;
        if let Some(ix) = ov.get("tx") {
            decoded = merge_decoded(decoded, ix.get("decoded").cloned(), &to);
            if method.starts_with("0x") {
                if let Some(n) = decoded.get("name").and_then(|v| v.as_str()) {
                    if !n.is_empty() && !n.starts_with("0x") {
                        method = n.to_string();
                    }
                } else if let Some(m) = ix.get("method").and_then(|v| v.as_str()) {
                    if !m.is_empty() && !m.starts_with("0x") {
                        method = m.to_string();
                    }
                }
            }
            if l1.is_null() {
                if let Some(v) = ix.get("l1").cloned().filter(|v| !v.is_null()) {
                    l1 = v;
                }
            }
            if l1_fee.is_null() {
                if let Some(v) = ix.get("l1_fee").cloned().filter(|v| !v.is_null()) {
                    l1_fee = v;
                }
            }
            if l1_gas.is_null() {
                if let Some(v) = ix.get("l1_gas").cloned().filter(|v| !v.is_null()) {
                    l1_gas = v;
                }
            }
            if l1_gas_price.is_null() {
                if let Some(v) = ix.get("l1_gas_price").cloned().filter(|v| !v.is_null()) {
                    l1_gas_price = v;
                }
            }
            if let Some(v) = ix.get("revert").cloned().filter(|v| {
                v.as_str().map(|s| !s.trim().is_empty()).unwrap_or(false)
            }) {
                revert = v;
            }
        }
        if let Some(items) = ov.get("logs").and_then(|v| v.as_array()) {
            overlay_decoded_logs(&mut logs, items);
        }
    }
    let mut internal = if let Some(ov) = crate::index::tx_overlay_if_ready(&h) {
        ov.get("internal")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    for item in &mut internal {
        label_tx_addrs(item);
    }
    let ts = block_hdr
        .as_ref()
        .and_then(|b| match opt_u64(b.get("timestamp")) {
            Value::Null => None,
            v => Some(v),
        })
        .unwrap_or(Value::Null);
    let hash_out = str_of(&t, "hash");
    let gas_limit_tx = hex_u64(t.get("gas").unwrap_or(&Value::Null));
    let gas_used_n = if pending {
        None
    } else {
        Some(hex_u64(receipt.get("gasUsed").unwrap_or(&Value::Null)))
    };
    let eff = receipt
        .get("effectiveGasPrice")
        .filter(|v| !v.is_null())
        .cloned()
        .unwrap_or_else(|| t.get("gasPrice").cloned().unwrap_or(Value::Null));
    let fee_wei = gas_used_n
        .map(|g| g as u128 * hex_u128_val(&eff))
        .unwrap_or(0);
    let gas_pct = match (gas_used_n, gas_limit_tx) {
        (Some(used), lim) if lim > 0 => {
            ((used as f64 / lim as f64) * 100.0).clamp(0.0, 100.0)
        }
        _ => -1.0,
    };
    let block_n = hex_u64(t.get("blockNumber").unwrap_or(&Value::Null));
    let head_n = last_head()
        .and_then(|h| h.get("block").and_then(|v| v.as_u64()))
        .or_else(|| call("eth_blockNumber", json!([])).ok().map(|v| hex_u64(&v)))
        .unwrap_or(block_n);
    let confirmations = if pending || block_n == 0 {
        Value::Null
    } else {
        json!(head_n.saturating_sub(block_n) + 1)
    };
    let input_bytes = input.len().saturating_sub(input.starts_with("0x") as usize * 2) / 2;
    let transfers = enrich_tx_transfers(
        logs.iter()
            .filter(|lg| lg.get("event").and_then(|v| v.as_str()) == Some("Transfer"))
            .cloned()
            .collect(),
    );
    let ty = hex_u64(t.get("type").unwrap_or(&Value::Null));
    let method_action = if to.is_empty() {
        "Contract creation".to_string()
    } else {
        method.clone()
    };
    let action = tx_action_summary(
        value_wei,
        str_of(&t, "value").as_str(),
        &transfers,
        &method_action,
    );
    let from = str_of(&t, "from");
    let headline = headline_value(value_wei, str_of(&t, "value").as_str(), &transfers);
    let base_fee_wei = block_hdr
        .as_ref()
        .map(|b| hex_u128_val(b.get("baseFeePerGas").unwrap_or(&Value::Null)))
        .unwrap_or(0);
    let burnt_wei = gas_used_n.map(|g| g as u128 * base_fee_wei).unwrap_or(0);
    let max_fee_wei = hex_u128_val(t.get("maxFeePerGas").unwrap_or(&Value::Null));
    let eff_wei = hex_u128_val(&eff);
    let savings_wei = if max_fee_wei > eff_wei {
        gas_used_n
            .map(|g| g as u128 * (max_fee_wei - eff_wei))
            .unwrap_or(0)
    } else {
        0
    };
    let eth_usd = native_usd();
    let fee_usd = usd_from_wei(fee_wei, eth_usd);
    let value_usd = usd_from_wei(value_wei, eth_usd);
    let log_len = logs.len();
    Ok(json!({
        "ok": true,
        "hash": if hash_out.is_empty() { json!(h) } else { json!(hash_out) },
        "chain": CHAIN_NAME,
        "chain_id": CHAIN_ID,
        "rpc": RPC_HTTP,
        "block": if pending || block_n == 0 { Value::Null } else { json!(block_n) },
        "block_hash": str_of(&t, "blockHash"),
        "index": opt_u64(t.get("transactionIndex")),
        "from_label": addr_label(&from),
        "from": from,
        "to_label": addr_label(&to),
        "to": if to.is_empty() { Value::Null } else { json!(to) },
        "contract": if contract.is_empty() { Value::Null } else { json!(contract) },
        "value": fmt_eth_hex(str_of(&t, "value").as_str()),
        "value_wei": str_of(&t, "value"),
        "value_usd": value_usd,
        "headline": headline,
        "nonce": hex_u64(t.get("nonce").unwrap_or(&Value::Null)),
        "gas": gas_limit_tx,
        "gas_used": match gas_used_n { Some(n) => json!(n), None => Value::Null },
        "gas_pct": if gas_pct < 0.0 { Value::Null } else { json!(gas_pct) },
        "gas_price": fmt_gwei(t.get("gasPrice").unwrap_or(&Value::Null)),
        "effective_gas_price": fmt_gwei(&eff),
        "max_fee": fmt_gwei(t.get("maxFeePerGas").unwrap_or(&Value::Null)),
        "priority": fmt_gwei(t.get("maxPriorityFeePerGas").unwrap_or(&Value::Null)),
        "fee": if pending { Value::Null } else { json!(fmt_fixed(fee_wei, 18, NATIVE_SYMBOL)) },
        "fee_wei": if pending { Value::Null } else { json!(fee_wei.to_string()) },
        "fee_usd": fee_usd,
        "burnt": if pending || burnt_wei == 0 { Value::Null } else { json!(fmt_fixed(burnt_wei, 18, NATIVE_SYMBOL)) },
        "savings": if pending || savings_wei == 0 { Value::Null } else { json!(fmt_fixed(savings_wei, 18, NATIVE_SYMBOL)) },
        "input": input,
        "input_bytes": input_bytes,
        "method": method,
        "action": action,
        "decoded": decoded,
        "status": status,
        "success": status == 1,
        "revert": revert,
        "type": ty,
        "type_name": tx_type_name(ty),
        "logs": logs,
        "log_count": log_len,
        "transfers": transfers,
        "internal": internal,
        "index_loading": index_loading,
        "loading": index_loading,
        "l1": l1,
        "l1_fee": l1_fee,
        "l1_gas": l1_gas,
        "l1_gas_price": l1_gas_price,
        "confirmations": confirmations,
        "head": head_n,
        "ts": ts,
        "now": now_secs(),
    }))
}

/// Ensure each Transfer map has `symbol` + `usdg` for UI summaries.

/// Ensure each Transfer map has `symbol` + `usdg` for UI summaries.
pub(crate) fn enrich_tx_transfers(mut transfers: Vec<Value>) -> Vec<Value> {
    for t in &mut transfers {
        let Some(obj) = t.as_object_mut() else {
            continue;
        };
        let token = obj
            .get("token")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("address").and_then(|v| v.as_str()))
            .unwrap_or("");
        let usdg = token.eq_ignore_ascii_case(USDG);
        let symbol = if usdg {
            "USDG".to_string()
        } else {
            obj.get("symbol")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };
        obj.insert("symbol".into(), json!(symbol));
        obj.insert("usdg".into(), json!(usdg));
        let nft = obj.get("nft").and_then(|v| v.as_bool()).unwrap_or(false)
            || obj
                .get("amount")
                .and_then(|v| v.as_str())
                .map(|s| s.starts_with('#'))
                .unwrap_or(false)
            || obj
                .get("kind")
                .and_then(|v| v.as_str())
                .map(|s| s.to_ascii_uppercase().contains("721") || s.to_ascii_uppercase().contains("1155"))
                .unwrap_or(false);
        obj.insert("nft".into(), json!(nft));
        if obj.get("kind").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
            obj.insert(
                "kind".into(),
                json!(if nft { "ERC-721" } else { "ERC-20" }),
            );
        }
    }
    transfers
}

/// Etherscan-style one-line action: native value, first token transfer, or method.

/// Etherscan-style one-line action: native value, first token transfer, or method.
pub(crate) fn tx_action_summary(
    value_wei: u128,
    value_hex: &str,
    transfers: &[Value],
    fallback: &str,
) -> String {
    if value_wei > 0 {
        return format!("Transfer {}", fmt_eth_hex(value_hex));
    }
    if let Some(first) = transfers.first() {
        let amount = first
            .get("amount")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let sym = first
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let token = first
            .get("token")
            .and_then(|v| v.as_str())
            .or_else(|| first.get("address").and_then(|v| v.as_str()))
            .unwrap_or("")
            .trim();
        let unit = if !sym.is_empty() { sym } else { token };
        // amount may already include the unit (e.g. "1 USDG")
        if amount.is_empty() {
            return if unit.is_empty() {
                "Transfer".into()
            } else {
                format!("Transfer {unit}")
            };
        }
        if unit.is_empty()
            || amount.ends_with(unit)
            || amount.contains(&format!(" {unit}"))
        {
            return format!("Transfer {amount}");
        }
        return format!("Transfer {amount} {unit}");
    }
    fallback.to_string()
}


pub(crate) fn headline_value(value_wei: u128, value_hex: &str, transfers: &[Value]) -> String {
    if value_wei > 0 {
        return fmt_eth_hex(value_hex);
    }
    if let Some(first) = transfers.first() {
        let amount = first
            .get("amount")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if !amount.is_empty() {
            return amount.to_string();
        }
    }
    fmt_eth_hex(value_hex)
}


pub(crate) fn parse_qty(s: &str) -> f64 {
    let t = s.trim();
    let num: String = t
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    num.replace(',', "").parse().unwrap_or(0.0)
}


pub(crate) fn tx_type_name(n: u64) -> &'static str {
    match n {
        0 => "legacy",
        1 => "EIP-2930",
        2 => "EIP-1559",
        3 => "EIP-4844",
        _ => "tx",
    }
}


pub(crate) fn txs_from_block(raw: &Value, number: u64, cap: Option<usize>) -> Vec<Value> {
    match cap {
        Some(n) => txs_from_block_range(raw, number, 0, n),
        None => txs_from_block_range(raw, number, 0, usize::MAX),
    }
}


pub(crate) fn txs_from_block_range(raw: &Value, number: u64, start: usize, count: usize) -> Vec<Value> {
    let Some(arr) = raw.get("transactions").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let ts = hex_u64(raw.get("timestamp").unwrap_or(&Value::Null));
    arr.iter()
        .skip(start)
        .take(count)
        .filter_map(|tx| {
            if let Some(h) = tx.as_str() {
                let h = h.trim();
                if h.is_empty() {
                    return None;
                }
                return Some(json!({
                    "hash": h,
                    "from": Value::Null,
                    "to": Value::Null,
                    "value": "",
                    "method": "tx",
                    "block": number,
                    "ts": ts,
                }));
            }
            if !tx.is_object() {
                return None;
            }
            let hash = str_of(tx, "hash");
            if hash.is_empty() {
                return None;
            }
            let to = str_of(tx, "to");
            let input = str_of(tx, "input");
            let value_wei = hex_u128(str_of(tx, "value").as_str());
            Some(json!({
                "hash": hash,
                "from": str_of(tx, "from"),
                "from_label": addr_label(&str_of(tx, "from")),
                "to": if to.is_empty() { Value::Null } else { json!(to) },
                "to_label": addr_label(&to),
                "value": fmt_eth_hex(str_of(tx, "value").as_str()),
                "method": method_name_val(&input, value_wei),
                "block": number,
                "ts": ts,
                "nonce": hex_u64(tx.get("nonce").unwrap_or(&Value::Null)),
                "index": opt_u64(tx.get("transactionIndex")),
                "gas": hex_u64(tx.get("gas").unwrap_or(&Value::Null)),
                "gas_price": fmt_gwei(tx.get("gasPrice").unwrap_or(&Value::Null)),
            }))
        })
        .collect()
}


pub(crate) fn tx_from_head_cache(h: &str) -> Option<Value> {
    let head = last_head()?;
    let arr = head.get("transactions")?.as_array()?;
    let row = arr.iter().find(|t| {
        t.get("hash")
            .and_then(|v| v.as_str())
            .map(|s| s.eq_ignore_ascii_case(h))
            .unwrap_or(false)
    })?;
    Some(json!({
        "ok": true,
        "hash": row.get("hash"),
        "chain": CHAIN_NAME,
        "chain_id": CHAIN_ID,
        "rpc": RPC_HTTP,
        "from": row.get("from"),
        "to": row.get("to"),
        "value": row.get("value"),
        "method": row.get("method"),
        "action": row.get("method"),
        "block": row.get("block"),
        "ts": row.get("ts"),
        "status": 2,
        "type": 0,
        "type_name": "legacy",
        "logs": [],
        "transfers": [],
        "partial": true,
        "now": now_secs(),
    }))
}


pub(crate) fn l1_receipt_val<'a>(receipt: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    for k in keys {
        if let Some(v) = receipt.get(*k) {
            if v.is_null() {
                continue;
            }
            match v {
                Value::String(s) if s.is_empty() || s.eq_ignore_ascii_case("0x") => continue,
                _ => return Some(v),
            }
        }
    }
    None
}


pub(crate) fn l1_fee_from_receipt(receipt: &Value) -> Value {
    match l1_receipt_val(receipt, &["l1Fee", "l1_fee"]) {
        Some(v) => {
            let n = match v {
                Value::String(s) => hex_u128(s),
                _ => hex_u128_val(v),
            };
            if n == 0 {
                Value::Null
            } else {
                json!(fmt_fixed(n, 18, NATIVE_SYMBOL))
            }
        }
        None => Value::Null,
    }
}


pub(crate) fn l1_gas_from_receipt(receipt: &Value) -> Value {
    match l1_receipt_val(
        receipt,
        &["gasUsedForL1", "l1GasUsed", "l1_gas_used", "gas_used_for_l1"],
    ) {
        Some(v) => opt_u64(Some(v)),
        None => Value::Null,
    }
}


pub(crate) fn l1_gas_price_from_receipt(receipt: &Value) -> Value {
    match l1_receipt_val(receipt, &["l1GasPrice", "l1_gas_price"]) {
        Some(v) => {
            let s = fmt_gwei(v);
            if s == "—" {
                Value::Null
            } else {
                json!(s)
            }
        }
        None => Value::Null,
    }
}

