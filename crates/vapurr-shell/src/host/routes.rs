use super::assets::{cache_control, mime, read_frontend, HTML_PRELOAD};
use super::pns::{inject_pns, pns_scan_hit};
use super::zzzmail_api::{json_body, zzzmail_api};
use super::*;

pub fn serve(
    _id: wry::WebViewId<'_>,
    req: wry::http::Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    let path = req.uri().path();
    let query = req.uri().query().unwrap_or("");
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() { "home.html" } else { rel };
    if let Some(resp) = zzzmail_api(
        rel.split('?').next().unwrap_or(rel),
        req.method(),
        req.body(),
    ) {
        return resp;
    }
    // Custom-protocol query strings vanish. Keep a path-stuffed `?…` for Scan.
    if let Some(kind_raw) = rel.strip_prefix("scan/api/") {
        let kind_raw = kind_raw.trim_end_matches('/');
        let (kind, stuffed) = match kind_raw.split_once('?') {
            Some((k, q)) => (k.trim_end_matches('/'), q),
            None => (kind_raw, ""),
        };
        let q = if query.is_empty() { stuffed } else { query };
        if kind == "search" || kind == "suggest" {
            if let Some(hit) = pns_scan_hit(q) {
                return json_body(hit);
            }
        }
        let body = vapurr_rhc::scan::api(kind, q);
        let verb = kind.split('/').next().unwrap_or(kind);
        let stamped = if matches!(
            verb,
            "tokens" | "txs" | "blocks" | "liq" | "head" | "gas" | "suggest"
        ) {
            body
        } else {
            inject_pns(body)
        };
        return Response::builder()
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(stamped.into_bytes()))
            .unwrap();
    }
    if let Some(rest) = rel.strip_prefix("trenches/api") {
        let rest = rest.trim_start_matches('/');
        let (kind, stuffed) = match rest.split_once('?') {
            Some((k, q)) => (k.trim_end_matches('/'), q),
            None => (rest.trim_end_matches('/'), ""),
        };
        let q = if query.is_empty() { stuffed } else { query };
        let body = vapurr_fomo::trenches_json(kind, q);
        return Response::builder()
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(body.into_bytes()))
            .unwrap();
    }
    let rel = rel.split('?').next().unwrap_or(rel);
    let rel = if rel == "ketbook" || rel == "ketbook/" {
        "ketbook/index.html"
    } else {
        rel
    };
    if let Some(kind) = rel.strip_prefix("patch/api/") {
        let kind = kind.trim_end_matches('/');
        if *req.method() == Method::OPTIONS {
            return json_body(serde_json::json!({ "ok": true }));
        }
        if kind == "status" {
            return json_body(crate::patch::status_json());
        }
        return json_body(serde_json::json!({ "ok": false, "error": "unknown" }));
    }
    if rel == "fomo/api/desk" {
        let body = vapurr_fomo::desk_json();
        return Response::builder()
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(body.into_bytes()))
            .unwrap();
    }
    if rel == "fomo/api/trenches" {
        let body = vapurr_fomo::trenches_json("", query);
        return Response::builder()
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(body.into_bytes()))
            .unwrap();
    }
    if rel == "liq/api" || rel == "liq/api/" {
        let body = vapurr_rhc::liq::snapshot_json();
        return Response::builder()
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(body.into_bytes()))
            .unwrap();
    }
    if let Some(rest) = rel.strip_prefix("liq/api/") {
        let rest = rest.trim_end_matches('/');
        let body = if rest == "tape" {
            vapurr_rhc::liq::tape_json()
        } else if let Some(pool) = rest.strip_prefix("trades/") {
            vapurr_rhc::liq::trades_json(pool)
        } else {
            serde_json::json!({ "ok": false, "error": "unknown" }).to_string()
        };
        return Response::builder()
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(body.into_bytes()))
            .unwrap();
    }
    if rel == "route/api/quote" {
        let body = vapurr_rhc::route::quote_json(query);
        return Response::builder()
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(body.into_bytes()))
            .unwrap();
    }
    if rel == "route/api/tokens" {
        let body = vapurr_rhc::route::tokens_json(query);
        return Response::builder()
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(body.into_bytes()))
            .unwrap();
    }
    if rel.contains("..") {
        return Response::builder()
            .status(400)
            .header(CONTENT_TYPE, "text/plain")
            .body(Cow::Borrowed(&b"bad path"[..]))
            .unwrap();
    }
    // Prefer the live frontend folder so logo/html edits show without a rebuild.
    if let Some(bytes) = read_frontend(rel) {
        let mut b = Response::builder()
            .header(CONTENT_TYPE, mime(rel))
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", cache_control(rel));
        if rel.ends_with(".html") {
            b = b.header("Link", HTML_PRELOAD);
        }
        return b.body(bytes).unwrap();
    }
    Response::builder()
        .status(404)
        .header(CONTENT_TYPE, "text/plain")
        .body(Cow::Borrowed(&b"not found"[..]))
        .unwrap()
}
