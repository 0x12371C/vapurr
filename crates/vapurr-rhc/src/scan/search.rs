use super::*;


pub(crate) fn search(q: &str) -> Result<Value, String> {
    match classify(q) {
        Query::BlockNumber(n) => {
            let b = block(&n.to_string(), 1)?;
            Ok(json!({ "ok": true, "kind": "block", "id": n, "block": b }))
        }
        Query::Address(a) => search_address(a),
        Query::Hash(h) => {
            let t = call("eth_getTransactionByHash", json!([h.clone()]))?;
            if t.is_object() {
                return Ok(json!({ "ok": true, "kind": "tx", "id": h }));
            }
            let b = call("eth_getBlockByHash", json!([h.clone(), false]))?;
            if b.is_object() {
                return Ok(json!({
                    "ok": true,
                    "kind": "block",
                    "id": hex_u64(b.get("number").unwrap_or(&Value::Null)),
                }));
            }
            Ok(json!({ "ok": true, "kind": "search", "items": [] }))
        }
        Query::Unknown => {
            let items = crate::index::search(q).unwrap_or_default();
            Ok(json!({ "ok": true, "kind": "search", "items": items }))
        }
    }
}


pub(crate) fn search_address(a: String) -> Result<Value, String> {
    crate::index::kick_token(&a, None, None);
    if a.eq_ignore_ascii_case(USDG) || crate::liq::token_hit(&a).is_some() {
        return Ok(json!({ "ok": true, "kind": "token", "id": a }));
    }
    if let Some(page) = crate::index::tokens_if_ready() {
        if page.items.iter().any(|t| {
            t.get("address")
                .and_then(|v| v.as_str())
                .map(|s| s.eq_ignore_ascii_case(&a))
                .unwrap_or(false)
        }) {
            return Ok(json!({ "ok": true, "kind": "token", "id": a }));
        }
    }
    if let Some(t) = crate::index::token_if_ready(&a, None, None) {
        let ok = t.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        let sym = t.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
        if ok || !sym.is_empty() {
            return Ok(json!({ "ok": true, "kind": "token", "id": a }));
        }
    }
    Ok(json!({ "ok": true, "kind": "addr", "id": a }))
}


pub(crate) fn suggest(q: &str) -> Vec<Value> {
    let q = q.trim();
    if q.is_empty() {
        return Vec::new();
    }
    match classify(q) {
        Query::BlockNumber(n) => vec![json!({
            "kind": "block",
            "label": format!("Block {n}"),
            "id": n.to_string(),
        })],
        Query::Address(a) => {
            if a.eq_ignore_ascii_case(USDG) {
                vec![json!({
                    "kind": "token",
                    "label": "USDG",
                    "id": a,
                    "address": a,
                })]
            } else if let Some(hit) = crate::liq::token_hit(&a) {
                vec![json!({
                    "kind": "token",
                    "label": hit.get("symbol").and_then(|x| x.as_str()).unwrap_or("token"),
                    "id": a,
                    "address": a,
                })]
            } else {
                vec![json!({
                    "kind": "addr",
                    "label": a,
                    "id": a,
                })]
            }
        }
        Query::Hash(h) => vec![json!({
            "kind": "hash",
            "label": h,
            "id": h,
        })],
        Query::Unknown => {
            let mut items = Vec::new();
            if let Some(snap) = crate::liq::cached_ok() {
                let qlow = q.to_ascii_lowercase();
                if let Some(tokens) = snap.get("tokens").and_then(|x| x.as_array()) {
                    for t in tokens {
                        if items.len() >= 6 {
                            break;
                        }
                        let sym = t.get("symbol").and_then(|x| x.as_str()).unwrap_or("");
                        let addr = t.get("address").and_then(|x| x.as_str()).unwrap_or("");
                        if !sym.to_ascii_lowercase().contains(&qlow) {
                            continue;
                        }
                        items.push(json!({
                            "kind": "token",
                            "label": sym,
                            "id": addr,
                            "address": addr,
                        }));
                    }
                }
            }
            items
        }
    }
}

