//! Native trust boundaries. URL substrings and renderer confirmation are not authority.
use std::cell::RefCell;
use std::sync::{mpsc, Mutex, OnceLock};

pub const CHROME_ORIGIN: &str = "http://vapurr.localhost";

pub fn is_chrome_url(raw: &str) -> bool {
    let Ok(u) = url::Url::parse(raw) else { return false };
    u.scheme() == "http" && u.host_str() == Some("vapurr.localhost")
        && u.port_or_known_default() == Some(80)
        && u.username().is_empty() && u.password().is_none()
}

pub fn chrome_path(raw: &str) -> Option<String> {
    is_chrome_url(raw).then(|| url::Url::parse(raw).unwrap().path().to_owned())
}

#[derive(Clone, Debug)]
pub struct Document {
    pub url: String,
    pub token: String,
}

thread_local! { static DOCUMENT: RefCell<Option<Document>> = const { RefCell::new(None) }; }
pub fn set_document(doc: Option<Document>) { DOCUMENT.with(|d| *d.borrow_mut() = doc); }
pub fn document() -> Option<Document> { DOCUMENT.with(|d| d.borrow().clone()) }

/// The execution-time check also covers navigation between evaluate_script and execution.
pub fn guarded_script(script: &str, doc: Option<&Document>) -> String {
    let binding = doc.map(|d| format!(
        " && location.href === {} && window.__vapurrDocument === {}",
        serde_json::to_string(&d.url).unwrap(), serde_json::to_string(&d.token).unwrap()
    )).unwrap_or_default();
    format!("if (location.origin === 'http://vapurr.localhost' && !location.username && !location.password{binding}) {{ {script}\n }}")
}

pub fn eval_chrome(view: &wry::WebView, script: &str) -> wry::Result<()> {
    if !view.url().map(|u| is_chrome_url(&u)).unwrap_or(false) { return Ok(()); }
    view.evaluate_script(&guarded_script(script, document().as_ref()))
}

pub struct BoundSender<T>(mpsc::Sender<(Option<Document>, T)>);
impl<T> BoundSender<T> {
    pub fn send(&self, cmd: T) -> Result<(), mpsc::SendError<(Option<Document>, T)>> {
        self.0.send((document(), cmd))
    }
}
pub fn bound_channel<T>() -> (BoundSender<T>, mpsc::Receiver<(Option<Document>, T)>) {
    let (tx, rx) = mpsc::channel(); (BoundSender(tx), rx)
}

pub fn api_token() -> &'static str {
    static TOKEN: OnceLock<String> = OnceLock::new();
    TOKEN.get_or_init(|| {
        use rand::RngCore;
        let mut bytes = [0u8; 32]; rand::rngs::OsRng.fill_bytes(&mut bytes);
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    })
}

/// Installed only in our origin. The fetch capability is never sent to external hosts.
pub fn init_script() -> String {
    include_str!("security.js").replace("__API_TOKEN__", &serde_json::to_string(api_token()).unwrap())
}

pub fn api_authorized(req: &wry::http::Request<Vec<u8>>) -> bool {
    req.headers().get("x-vapurr-client").and_then(|h| h.to_str().ok()) == Some(api_token())
        && req.headers().get("origin").map(|h| h.to_str().ok() == Some(CHROME_ORIGIN)).unwrap_or(true)
        && req.headers().get("sec-fetch-site").map(|h| h == "same-origin" || h == "none").unwrap_or(true)
}

/// Native approval cannot be satisfied by clicking or scripting the HTML confirmation sheet.
#[cfg(windows)]
#[path = "confirm_win.rs"]
mod confirm_win;

pub fn confirm(description: &str) -> bool {
    static PROMPT: Mutex<()> = Mutex::new(());
    let Ok(_guard) = PROMPT.try_lock() else { return false };
    #[cfg(windows)]
    {
        confirm_win::show(description)
    }
    #[cfg(not(windows))]
    {
        let _ = description;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chrome_origin_is_exact() {
        assert!(is_chrome_url("http://vapurr.localhost/wallet.html"));
        for u in ["https://example.invalid/cookies.html?vapurr.localhost", "http://vapurr.localhost.evil/", "http://vapurr.localhost@evil/", "http://evil@vapurr.localhost/", "http://vapurr.localhost:9000/", "https://vapurr.localhost/", "vapurr://wallet", "data:text/html,vapurr.localhost"] {
            assert!(!is_chrome_url(u), "{u}");
        }
    }
    #[test]
    fn private_api_requires_capability_and_rejects_external_origin() {
        use wry::http::Request;
        assert!(!api_authorized(&Request::builder().body(vec![]).unwrap()));
        assert!(!api_authorized(&Request::builder().header("x-vapurr-client", api_token()).header("origin", "https://evil.invalid").body(vec![]).unwrap()));
        assert!(api_authorized(&Request::builder().header("x-vapurr-client", api_token()).header("origin", CHROME_ORIGIN).body(vec![]).unwrap()));
    }
}
