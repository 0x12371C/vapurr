//! First-party cookie jar on the shared WebView2 profile.

use serde_json::{json, Value};
use wry::WebView;

#[cfg(windows)]
use wry::WebViewExtWindows;

pub fn list(page: &WebView) -> Vec<Value> {
    match page.cookies() {
        Ok(cookies) => cookies.into_iter().map(row).collect(),
        Err(e) => {
            tracing::warn!("cookie list: {e}");
            Vec::new()
        }
    }
}

pub fn snapshot(rows: &[Value], page_url: &str) -> Value {
    json!({
        "ok": true,
        "count": rows.len(),
        "page": page_url,
        "cookies": rows,
        "profile": std::env::var("LOCALAPPDATA")
            .map(|h| format!("{h}\\vapurr\\edge"))
            .unwrap_or_else(|_| "vapurr/edge".into()),
    })
}

pub fn js_set(v: &Value) -> String {
    format!("window.__setCookies && window.__setCookies({})", v)
}

pub fn push(page: &WebView, page_url: &str) {
    if !crate::security::is_chrome_url(page_url) { return; }
    let rows = list(page);
    let snap = snapshot(&rows, page_url);
    let _ = crate::security::eval_chrome(page, &js_set(&snap));
}

#[cfg(windows)]
pub fn ensure_jar(page: &WebView) {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use windows::core::HSTRING;
    use windows::Win32::Foundation::BOOL;

    let Ok(mgr) = manager(page) else {
        return;
    };
    unsafe {
        let name = HSTRING::from("vapurr");
        let value = HSTRING::from("1p");
        let domain = HSTRING::from("vapurr.localhost");
        let path = HSTRING::from("/");
        let Ok(cookie) = mgr.CreateCookie(&name, &value, &domain, &path) else {
            return;
        };
        let _ = cookie.SetIsHttpOnly(BOOL::from(false));
        let _ = cookie.SetIsSecure(BOOL::from(false));
        let _ = cookie.SetSameSite(COREWEBVIEW2_COOKIE_SAME_SITE_KIND_LAX);
        let exp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as f64 + 86400.0 * 365.0 * 5.0)
            .unwrap_or(0.0);
        let _ = cookie.SetExpires(exp);
        if let Err(e) = mgr.AddOrUpdateCookie(&cookie) {
            tracing::warn!("cookie jar seed: {e}");
        }
    }
}

#[cfg(not(windows))]
pub fn ensure_jar(_page: &WebView) {}

#[cfg(windows)]
pub fn delete_one(page: &WebView, name: &str, domain: &str, path: &str) -> bool {
    use windows::core::HSTRING;
    let Ok(mgr) = manager(page) else {
        return false;
    };
    let path = if path.is_empty() { "/" } else { path };
    unsafe {
        mgr.DeleteCookiesWithDomainAndPath(
            &HSTRING::from(name),
            &HSTRING::from(domain),
            &HSTRING::from(path),
        )
        .is_ok()
    }
}

#[cfg(not(windows))]
pub fn delete_one(_page: &WebView, _name: &str, _domain: &str, _path: &str) -> bool {
    false
}

#[cfg(windows)]
pub fn delete_host(page: &WebView, host: &str) -> bool {
    let host = host.trim().trim_start_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return false;
    }
    let mut n = 0;
    for c in list(page) {
        let domain = c
            .get("domain")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_start_matches('.')
            .to_ascii_lowercase();
        if domain == host
            || domain.ends_with(&format!(".{host}"))
            || host.ends_with(&format!(".{domain}"))
        {
            let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let path = c.get("path").and_then(|v| v.as_str()).unwrap_or("/");
            let domain_raw = c.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            if delete_one(page, name, domain_raw, path) {
                n += 1;
            }
        }
    }
    n > 0
}

#[cfg(not(windows))]
pub fn delete_host(_page: &WebView, _host: &str) -> bool {
    false
}

#[cfg(windows)]
pub fn delete_all(page: &WebView) -> bool {
    let Ok(mgr) = manager(page) else {
        return false;
    };
    unsafe { mgr.DeleteAllCookies().is_ok() }
}

#[cfg(not(windows))]
pub fn delete_all(_page: &WebView) -> bool {
    false
}

#[cfg(windows)]
fn manager(
    page: &WebView,
) -> windows::core::Result<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2CookieManager>
{
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use windows::core::Interface;
    unsafe {
        let webview = page.controller().CoreWebView2()?;
        webview.cast::<ICoreWebView2_2>()?.CookieManager()
    }
}

fn row(c: wry::cookie::Cookie<'static>) -> Value {
    let name = c.name().to_string();
    let raw_val = c.value().to_string();
    let value = if raw_val.len() > 72 {
        format!("{}…", raw_val.chars().take(72).collect::<String>())
    } else {
        raw_val
    };
    let domain = c.domain().unwrap_or("").to_string();
    let path = c.path().unwrap_or("/").to_string();
    let http_only = c.http_only().unwrap_or(false);
    let secure = c.secure().unwrap_or(false);
    let same_site = match c.same_site() {
        Some(wry::cookie::SameSite::Strict) => "Strict",
        Some(wry::cookie::SameSite::Lax) => "Lax",
        Some(wry::cookie::SameSite::None) => "None",
        None => "",
    };
    let (session, expires) = match c.expires() {
        Some(wry::cookie::Expiration::Session) | None => (true, Value::Null),
        Some(wry::cookie::Expiration::DateTime(dt)) => {
            (false, json!(dt.unix_timestamp().max(0) as u64))
        }
    };
    json!({
        "name": name,
        "value": value,
        "domain": domain,
        "path": path,
        "http_only": http_only,
        "secure": secure,
        "session": session,
        "same_site": same_site,
        "expires": expires,
    })
}
