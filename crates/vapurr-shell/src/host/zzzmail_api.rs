use super::*;

pub(crate) fn office() -> MutexGuard<'static, vapurr_zmail::PostOffice> {
    static OFFICE: OnceLock<Mutex<vapurr_zmail::PostOffice>> = OnceLock::new();
    let m = OFFICE.get_or_init(|| {
        let mut o = vapurr_zmail::PostOffice::open_default().unwrap_or_else(|_| {
            vapurr_zmail::PostOffice::open(std::env::temp_dir().join("vapurr-zzzmail"))
                .expect("zzzmail")
        });
        if let Some(addr) = vapurr_wallet::peek_address() {
            let _ = o.set_address(&addr);
        }
        Mutex::new(o)
    });
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn json_body(v: serde_json::Value) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .header(CONTENT_TYPE, "application/json; charset=utf-8")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "content-type")
        .header("Cache-Control", "no-store")
        .body(Cow::Owned(v.to_string().into_bytes()))
        .unwrap()
}

pub(crate) fn sign_voucher(v: &mut vapurr_zmail::Voucher) {
    let Some(key) = vapurr_wallet::DeviceKey::load() else {
        return;
    };
    if v.from.is_empty() || v.from.starts_with('@') {
        v.from = key.address.to_hex();
    }
    if let Ok(sig) = key.sign_digest(&v.digest()) {
        v.sig = vapurr_wallet::hex0x(&sig);
    }
}

pub fn zzzmail_send_json(to: &str, body: &str, asset: &str) -> serde_json::Value {
    match office().send(to, "", body, asset) {
        Ok(mut r) => {
            sign_voucher(&mut r.voucher);
            serde_json::to_value(&r)
                .unwrap_or_else(|_| serde_json::json!({ "ok": true, "cid": r.cid }))
        }
        Err(e) => mail_err(e),
    }
}

pub fn zzzmail_inbox_json() -> serde_json::Value {
    office().snapshot()
}

pub(crate) fn mail_err(e: vapurr_zmail::ZmailError) -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "error": e.to_string(),
        "need_card": matches!(e, vapurr_zmail::ZmailError::NeedCard),
        "need_name": matches!(e, vapurr_zmail::ZmailError::NeedName),
        "taken": matches!(e, vapurr_zmail::ZmailError::NameTaken),
        "reserved": matches!(e, vapurr_zmail::ZmailError::ReservedName),
        "need_gas": matches!(e, vapurr_zmail::ZmailError::NeedGas),
        "faucet": if matches!(e, vapurr_zmail::ZmailError::NeedGas) {
            vapurr_rhc::TESTNET_FAUCET
        } else {
            ""
        },
    })
}

pub(crate) fn zzzmail_api(
    rel: &str,
    method: &Method,
    body: &[u8],
) -> Option<Response<Cow<'static, [u8]>>> {
    let kind = rel.strip_prefix("zzzmail/api/")?;
    let kind = kind.trim_end_matches('/');
    if kind == "pns" || kind.starts_with("pns/") {
        return pns_api(kind, method, body);
    }
    if *method == Method::OPTIONS {
        return Some(json_body(serde_json::json!({ "ok": true })));
    }
    match kind {
        "quote" => {
            let q = office().quote("PUSD");
            let v = office().quote("VAPURR");
            Some(json_body(serde_json::json!({
                "ok": true,
                "pusd": q,
                "vapurr": v,
                "gasless": true,
                "cap": "1¢",
            })))
        }
        "me" => {
            let me = office().me();
            Some(json_body(serde_json::json!({ "ok": true, "me": me })))
        }
        "inbox" => {
            let snap = office().snapshot();
            Some(json_body(snap))
        }
        "send" => {
            let v: serde_json::Value =
                serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
            let to = v.get("to").and_then(|x| x.as_str()).unwrap_or("");
            let text = v.get("body").and_then(|x| x.as_str()).unwrap_or("");
            let subject = v.get("subject").and_then(|x| x.as_str()).unwrap_or("");
            let asset = v.get("asset").and_then(|x| x.as_str()).unwrap_or("PUSD");
            match office().send(to, subject, text, asset) {
                Ok(mut r) => {
                    sign_voucher(&mut r.voucher);
                    Some(json_body(serde_json::to_value(&r).unwrap_or_else(
                        |_| serde_json::json!({ "ok": true, "cid": r.cid }),
                    )))
                }
                Err(e) => Some(json_body(mail_err(e))),
            }
        }
        "hood" | "hood/me" => {
            let snap = office().snapshot();
            Some(json_body(serde_json::json!({
                "ok": true,
                "pns": true,
                "kind": "hood",
                "service": "PNS",
                "me": snap.get("me"),
                "hood": snap.get("hood"),
                "pns_reg": snap.get("pns"),
            })))
        }
        k if k == "hood/register" || k.starts_with("hood/register/") => {
            let name = name_from_register(k, "hood/register", body);
            Some(json_body(zzzmail_hood_register_json(&name)))
        }
        other if other.starts_with("hood/resolve/") => {
            let name = other.trim_start_matches("hood/resolve/");
            match office().resolve_hood_local(name) {
                Some(v) => Some(json_body(v)),
                None => Some(json_body(mail_err(vapurr_zmail::ZmailError::NeedName))),
            }
        }
        other if other.starts_with("hood/reverse/") => {
            let addr = other.trim_start_matches("hood/reverse/");
            match office().reverse_hood_local(addr) {
                Some(n) => Some(json_body(serde_json::json!({
                    "ok": true,
                    "pns": true,
                    "kind": "hood",
                    "name": n,
                    "addr": addr,
                }))),
                None => Some(json_body(mail_err(vapurr_zmail::ZmailError::NeedName))),
            }
        }
        other if other.starts_with("letter/") => {
            let cid = other.trim_start_matches("letter/");
            match office().open_cid(cid) {
                Ok(item) => Some(json_body(serde_json::json!({ "ok": true, "letter": item }))),
                Err(e) => Some(json_body(
                    serde_json::json!({ "ok": false, "error": e.to_string() }),
                )),
            }
        }
        _ => Some(json_body(
            serde_json::json!({ "ok": false, "error": "unknown" }),
        )),
    }
}
