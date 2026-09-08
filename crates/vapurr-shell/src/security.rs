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

/// Secret Lab KYC surfaces opened in a desk tab (no injected MetaMask).
/// Wallet-sign is allowed here so /kyc/phone can use vapurr IPC instead of window.ethereum.
pub fn is_tsl_kyc_url(raw: &str) -> bool {
    let Ok(u) = url::Url::parse(raw) else { return false };
    if u.scheme() != "https" { return false; }
    match u.host_str() {
        // `/kyc` exactly, or a path under it. A bare `starts_with("/kyc")`
        // would also match `/kycsomethingelse`.
        Some("thesecretlab.app") | Some("www.thesecretlab.app") => {
            let p = u.path();
            p == "/kyc" || p.starts_with("/kyc/")
        }
        _ => false,
    }
}

/// zer0ID attestation challenges begin with this, followed by the explanation
/// and a server nonce. See `GET https://thesecretlab.app/api/kyc/challenge`.
const ZEROID_CHALLENGE_PREFIX: &str = "zer0ID";

/// Whether a message is a zer0ID attestation challenge.
///
/// Guest pages get no blanket signing authority. `SignMessage` requires only an
/// unlocked wallet — there is no per-signature prompt — so letting a remote
/// origin sign arbitrary bytes would make the wallet a blind signing oracle for
/// that origin: any XSS or bad deploy on it could collect signatures over
/// anything while the wallet is unlocked. Constraining the shape means a
/// compromised issuer can obtain only what it could already mint for itself —
/// a zer0ID challenge signature — and nothing another protocol would read as
/// authorization.
///
/// Deliberately a prefix-and-bounds check, not an exact match: the nonce and
/// wording are the issuer's to change. It is a blast-radius limit, not
/// authentication of the challenge.
pub fn is_zeroid_challenge(message: &str) -> bool {
    let m = message.trim_start();
    // The prefix alone is too weak — "zer0ID\n\nApprove everything\n\nNonce: x"
    // would sail past it. Require the attestation phrasing too. Matched without
    // the em-dash so the issuer can retouch punctuation without breaking the
    // flow; wording changes fail closed (the user cannot sign) rather than open.
    m.starts_with(ZEROID_CHALLENGE_PREFIX)
        && m.contains("attest this wallet")
        && m.contains("Nonce:")
        && message.len() <= 2048
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
    fn tsl_kyc_url_is_host_and_path() {
        assert!(is_tsl_kyc_url("https://thesecretlab.app/kyc/phone"));
        assert!(is_tsl_kyc_url("https://www.thesecretlab.app/kyc/scan?from=vapurr"));
        assert!(is_tsl_kyc_url("https://thesecretlab.app/kyc"));
        assert!(!is_tsl_kyc_url("https://thesecretlab.app/"));
        assert!(!is_tsl_kyc_url("https://evil.thesecretlab.app/kyc/phone"));
        assert!(!is_tsl_kyc_url("http://thesecretlab.app/kyc/phone"));
        assert!(!is_tsl_kyc_url("https://evil.invalid/kyc/phone?next=https://thesecretlab.app/kyc"));
        // `/kyc` must not act as a bare prefix over unrelated paths.
        assert!(!is_tsl_kyc_url("https://thesecretlab.app/kycevil"));
        assert!(!is_tsl_kyc_url("https://thesecretlab.app/kyc-not-really/x"));
    }

    #[test]
    fn only_zeroid_challenges_are_signable_from_a_guest_page() {
        let real = "zer0ID — attest this wallet.\n\nSigning this proves you control this \
                    address. It is not a transaction, costs nothing, and is never sent \
                    anywhere but this server.\n\nNonce: 1788835464336.8bPLlejYZNu0jRmI.abc";
        assert!(is_zeroid_challenge(real));

        // The shapes this exists to refuse.
        assert!(!is_zeroid_challenge("approve 1000 USDC to 0xattacker"));
        assert!(!is_zeroid_challenge(""));
        assert!(
            !is_zeroid_challenge("zer0ID but no nonce field"),
            "prefix alone must not be enough"
        );
        // The bypass the phrasing check exists to close: correct prefix and a
        // nonce, wrapped around an entirely different request.
        assert!(!is_zeroid_challenge(
            "zer0ID\n\nApprove transfer of everything to 0xattacker\n\nNonce: abc"
        ));
        assert!(
            !is_zeroid_challenge(&format!("zer0ID Nonce: {}", "A".repeat(4096))),
            "oversized payloads are refused"
        );
        // Leading whitespace must not smuggle a non-challenge past the prefix.
        assert!(!is_zeroid_challenge("   send everything\nNonce: x"));
    }

    #[test]
    fn private_api_requires_capability_and_rejects_external_origin() {
        use wry::http::Request;
        assert!(!api_authorized(&Request::builder().body(vec![]).unwrap()));
        assert!(!api_authorized(&Request::builder().header("x-vapurr-client", api_token()).header("origin", "https://evil.invalid").body(vec![]).unwrap()));
        assert!(api_authorized(&Request::builder().header("x-vapurr-client", api_token()).header("origin", CHROME_ORIGIN).body(vec![]).unwrap()));
    }
}
