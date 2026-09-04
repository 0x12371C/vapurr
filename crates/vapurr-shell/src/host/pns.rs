use super::*;
use super::zzzmail_api::{json_body, mail_err, office};


pub fn adopt_wallet_address(addr: &str) {
    let _ = office().set_address(addr);
}


pub fn pns_snap_json() -> serde_json::Value {
    let (addr, me, local_primary, hood) = {
        let mut o = office();
        let me = o.me();
        let local = o.snapshot();
        let hood = local.get("hood").cloned().unwrap_or(serde_json::json!({}));
        let p = hood
            .get("primary")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        (me.address.clone(), me, p, hood)
    };
    let mut v = vapurr_zmail::chain::status_snapshot(&addr);
    if v.get("primary").and_then(|x| x.as_str()).unwrap_or("").is_empty() && !local_primary.is_empty()
    {
        v["primary"] = serde_json::json!(local_primary);
        v["label"] = serde_json::json!(local_primary.trim_end_matches(".hood"));
    }
    v["ok"] = serde_json::json!(true);
    v["me"] = serde_json::json!(me);
    v["address"] = serde_json::json!(addr);
    v["hood"] = hood.clone();
    v["pns_reg"] = hood;
    v
}


pub fn zzzmail_hood_register_json(name: &str) -> serde_json::Value {
    match office().register_hood(name) {
        Ok(mut r) => {
            sign_voucher(&mut r.voucher);
            serde_json::to_value(&r).unwrap_or_else(|_| serde_json::json!({ "ok": true, "kind": "hood", "name": r.name }))
        }
        Err(e) => mail_err(e),
    }
}


pub(crate) fn pns_q(query: &str) -> String {
    for part in query.split('&') {
        if let Some(v) = part.strip_prefix("q=") {
            return v.replace("%40", "@").replace("%2E", ".").replace("%2e", ".");
        }
    }
    String::new()
}


pub(crate) fn pns_scan_hit(query: &str) -> Option<serde_json::Value> {
    let raw = pns_q(query);
    let raw = raw.trim().trim_start_matches('@');
    if !vapurr_zmail::looks_like_hood(raw) {
        return None;
    }
    match office().resolve_hood_local(raw) {
        Some(v) => {
            let addr = v
                .get("record")
                .and_then(|r| r.get("addr"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if addr.len() != 42 {
                return Some(serde_json::json!({
                    "ok": true,
                    "kind": "search",
                    "items": [],
                    "pns": true
                }));
            }
            Some(serde_json::json!({
                "ok": true,
                "kind": "addr",
                "id": addr,
                "pns": true,
                "name": raw,
            }))
        }
        None => Some(serde_json::json!({
            "ok": true,
            "kind": "search",
            "items": [],
            "pns": true
        })),
    }
}


pub(crate) fn inject_pns(body: String) -> String {
    let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&body) else {
        return body;
    };
    let mut cache = std::collections::HashMap::new();
    stamp_pns(&mut v, &mut cache);
    v.to_string()
}


pub(crate) fn stamp_pns(
    v: &mut serde_json::Value,
    cache: &mut std::collections::HashMap<String, Option<String>>,
) {
    let Some(obj) = v.as_object_mut() else {
        return;
    };
    stamp_pns_obj(obj, cache);
    obj.remove("ens");
    for key in [
        "transactions",
        "txs",
        "blocks",
        "tokens",
        "transfers",
        "internal",
    ] {
        if let Some(serde_json::Value::Array(arr)) = obj.get_mut(key) {
            for item in arr {
                if let Some(o) = item.as_object_mut() {
                    stamp_pns_obj(o, cache);
                    o.remove("ens");
                }
            }
        }
    }
}


pub(crate) fn stamp_pns_obj(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    cache: &mut std::collections::HashMap<String, Option<String>>,
) {
    for key in ["address", "from", "to", "miner", "creator"] {
        let Some(addr) = obj.get(key).and_then(|x| x.as_str()).map(|s| s.to_string()) else {
            continue;
        };
        if addr.len() != 42 {
            continue;
        }
        let k = addr.to_ascii_lowercase();
        let name = if let Some(hit) = cache.get(&k) {
            hit.clone()
        } else {
            let n = office().reverse_hood_local(&k);
            cache.insert(k, n.clone());
            n
        };
        if let Some(n) = name {
            let pk = if key == "address" {
                "pns".to_string()
            } else {
                format!("{key}_pns")
            };
            obj.insert(pk, serde_json::json!(n));
        }
    }
}


pub(crate) fn pns_tx_url(tx: &str) -> String {
    if tx.is_empty() {
        String::new()
    } else {
        format!("{}/tx/{}", vapurr_rhc::TESTNET_EXPLORER, tx)
    }
}


pub(crate) fn stamp_pns_tx(snap: &mut serde_json::Value, rec: &serde_json::Value) {
    if let Some(tx) = rec.get("tx").and_then(|x| x.as_str()) {
        if !tx.is_empty() {
            snap["tx"] = serde_json::json!(tx);
            snap["tx_url"] = serde_json::json!(pns_tx_url(tx));
        }
    }
    if let Some(url) = rec.get("tx_url").and_then(|x| x.as_str()) {
        if !url.is_empty() {
            snap["tx_url"] = serde_json::json!(url);
        }
    }
    if rec.get("already") == Some(&serde_json::json!(true)) {
        snap["already"] = serde_json::json!(true);
    }
    if rec.get("onchain") == Some(&serde_json::json!(true)) {
        snap["onchain"] = serde_json::json!(true);
    }
}


pub(crate) fn pct_decode(s: &str) -> String {
    s.replace("%40", "@")
        .replace("%2E", ".")
        .replace("%2e", ".")
        .replace("%2D", "-")
        .replace("%2d", "-")
}


pub(crate) fn name_from_register(kind: &str, prefix: &str, body: &[u8]) -> String {
    let v: serde_json::Value = serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
    let from_body = v
        .get("name")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim();
    if !from_body.is_empty() {
        return from_body.to_string();
    }
    if let Some(rest) = kind.strip_prefix(prefix) {
        let rest = rest.trim_start_matches('/');
        if !rest.is_empty() {
            return pct_decode(rest);
        }
    }
    String::new()
}


pub(crate) fn pns_api(kind: &str, method: &Method, body: &[u8]) -> Option<Response<Cow<'static, [u8]>>> {
    if *method == Method::OPTIONS {
        return Some(json_body(serde_json::json!({ "ok": true })));
    }
    match kind {
        "pns" | "pns/me" => Some(json_body(pns_snap_json())),
        "pns/deploy" => match vapurr_zmail::chain::deploy() {
            Ok(v) => Some(json_body(v)),
            Err(e) => Some(json_body(mail_err(e))),
        },
        "pns/set-addr" => {
            let v: serde_json::Value =
                serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
            let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
            let addr = v.get("addr").and_then(|x| x.as_str()).unwrap_or("");
            match vapurr_zmail::chain::set_addr(name, addr) {
                Ok(tx) => {
                    let mut snap = pns_snap_json();
                    snap["ok"] = serde_json::json!(true);
                    snap["tx"] = serde_json::json!(tx);
                    snap["tx_url"] = serde_json::json!(pns_tx_url(&tx));
                    Some(json_body(snap))
                }
                Err(e) => Some(json_body(mail_err(e))),
            }
        }
        "pns/set-name" => {
            let v: serde_json::Value =
                serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
            let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
            match vapurr_zmail::chain::set_name(name) {
                Ok(tx) => {
                    let mut snap = pns_snap_json();
                    snap["ok"] = serde_json::json!(true);
                    snap["tx"] = serde_json::json!(tx);
                    snap["tx_url"] = serde_json::json!(pns_tx_url(&tx));
                    Some(json_body(snap))
                }
                Err(e) => Some(json_body(mail_err(e))),
            }
        }
        k if k == "pns/register" || k.starts_with("pns/register/") => {
            let name = name_from_register(k, "pns/register", body);
            let rec = zzzmail_hood_register_json(&name);
            if rec.get("ok") == Some(&serde_json::json!(false))
                || rec.get("error").is_some() && rec.get("ok") != Some(&serde_json::json!(true))
            {
                return Some(json_body(rec));
            }
            if rec.get("ok") == Some(&serde_json::json!(true))
                && rec.get("onchain") != Some(&serde_json::json!(true))
                && rec.get("tx").and_then(|x| x.as_str()).unwrap_or("").is_empty()
            {
                return Some(json_body(serde_json::json!({
                    "ok": false,
                    "error": "No hash. Nothing was broadcast."
                })));
            }
            let mut snap = pns_snap_json();
            snap["just_registered"] = serde_json::json!(true);
            snap["receipt"] = rec.clone();
            stamp_pns_tx(&mut snap, &rec);
            if let Some(n) = rec.get("name") {
                snap["name"] = n.clone();
                snap["kind"] = serde_json::json!("hood");
            }
            Some(json_body(snap))
        }
        other if other.starts_with("pns/resolve/") => {
            let name = other.trim_start_matches("pns/resolve/");
            match office().resolve_hood_local(name) {
                Some(v) => {
                    let rec = v.get("record").cloned().unwrap_or(serde_json::Value::Null);
                    let name = rec
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or(name)
                        .to_string();
                    let owner = rec
                        .get("owner")
                        .or_else(|| rec.get("addr"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    let addr = rec
                        .get("addr")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    let mut snap = pns_snap_json();
                    let full = if name.ends_with(".hood") {
                        name.clone()
                    } else {
                        format!("{name}.hood")
                    };
                    snap["lookup"] = serde_json::json!({
                        "name": full,
                        "available": owner.is_empty(),
                        "owner": owner,
                        "addr": addr,
                    });
                    snap["record"] = serde_json::json!({
                        "name": full,
                        "owner": owner,
                        "addr": addr,
                    });
                    Some(json_body(snap))
                }
                None => {
                    let mut snap = pns_snap_json();
                    let n = name.trim().trim_start_matches('@');
                    snap["lookup"] = serde_json::json!({
                        "name": if n.ends_with(".hood") { n.to_string() } else { format!("{n}.hood") },
                        "available": true,
                        "owner": "",
                        "addr": "",
                    });
                    Some(json_body(snap))
                }
            }
        }
        _ => Some(json_body(serde_json::json!({ "ok": false, "error": "unknown" }))),
    }
}

