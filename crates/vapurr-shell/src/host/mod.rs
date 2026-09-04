//! In-process chrome host: RAM asset cache, shared Edge profile, lean WebView2 flags.

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use rust_embed::RustEmbed;
use wry::http::{header::CONTENT_TYPE, Method, Response};

mod assets;
mod pns;
mod routes;
mod wv;
mod zzzmail_api;

pub(crate) use assets::*;
pub(crate) use pns::*;
pub use pns::{adopt_wallet_address, pns_snap_json, zzzmail_hood_register_json};
pub use routes::serve;
pub use wv::*;
pub(crate) use zzzmail_api::*;
pub use zzzmail_api::{zzzmail_inbox_json, zzzmail_send_json};

#[cfg(test)]
mod tests {
    use super::{allow_new_window, is_wallet_scheme};

    #[test]
    fn wc_uri_is_a_wallet_scheme() {
        assert!(is_wallet_scheme("wc:abc123@2?relay-protocol=irn&symKey=00"));
        assert!(is_wallet_scheme("ethereum:pay-0x00"));
        assert!(!is_wallet_scheme("https://fomo.family"));
        assert!(!is_wallet_scheme("about:blank"));
    }

    #[test]
    fn popups_keep_the_page() {
        assert!(allow_new_window("https://auth.privy.io/login"));
        assert!(allow_new_window("about:blank"));
        assert!(allow_new_window("about:blank?walletconnect"));
        assert!(allow_new_window(""));
        assert!(!allow_new_window("wc:abc@2"));
        assert!(!allow_new_window("javascript:alert(1)"));
    }

    #[test]
    fn hot_assets_are_immutable() {
        assert!(super::is_hot_asset("vendor/three.webgpu.min.js"));
        assert!(super::is_hot_asset("fonts/Sora-Regular.ttf"));
        assert!(super::is_hot_asset("logo.png"));
        assert!(!super::is_hot_asset("globe.js"));
        assert!(!super::is_hot_asset("home.html"));
        assert_eq!(
            super::cache_control("vendor/three.webgpu.min.js"),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(super::cache_control("home.html"), "no-store");
        assert_eq!(super::cache_control("globe.js"), "no-store");
        assert_eq!(
            super::cache_control("ketflix/posters/the-ketrix.png"),
            "no-store"
        );
        assert_eq!(super::mime("ketflix/posters/the-ketrix.png"), "image/png");
        assert_eq!(super::mime("ketflix/trailers/the-ketrix.mp4"), "video/mp4");
        assert_eq!(
            super::mime("ketflix/catalog.json"),
            "application/json; charset=utf-8"
        );
    }

    fn serve_get(path: &str) -> wry::http::Response<std::borrow::Cow<'static, [u8]>> {
        let req = wry::http::Request::builder()
            .uri(format!("http://vapurr.localhost/{path}"))
            .body(Vec::new())
            .unwrap();
        super::serve("", req)
    }

    #[test]
    fn ketflix_posters_are_served() {
        use wry::http::header::CONTENT_TYPE;
        let bytes = super::read_frontend("ketflix/posters/the-ketrix.png")
            .expect("live frontend/ketflix/posters/the-ketrix.png");
        assert!(bytes.len() > 10_000, "poster too small: {}B", bytes.len());

        let resp = serve_get("ketflix/posters/the-ketrix.png");
        assert_eq!(resp.status(), 200);
        let ct = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(ct, "image/png");
        assert!(resp.body().len() > 10_000);

        let catalog = serve_get("ketflix/catalog.json");
        assert_eq!(
            catalog.status(),
            200,
            "catalog.json must be on the chrome host"
        );
        let body = std::str::from_utf8(catalog.body()).unwrap_or("");
        assert!(body.contains("the-ketrix"), "catalog missing the-ketrix");

        for slug in [
            "the-ketrix",
            "ketbreaking-bad",
            "the-lion-of-ketdah",
            "pirates-of-ketibbean",
            "keterman",
            "home-alone-ketsters",
            "the-ketfather",
            "attack-on-ket",
            "back-to-the-keture",
            "the-lord-of-the-ket",
            "joket",
            "john-wick-ketribution",
        ] {
            let r = serve_get(&format!("ketflix/posters/{slug}.png"));
            assert_eq!(r.status(), 200, "{slug} poster");
            assert!(r.body().len() > 10_000, "{slug} too small");
        }
    }

    #[test]
    fn webview_flags_are_not_puppeteer() {
        let args = super::WV_ARGS;
        assert!(
            !args.contains("disable-client-side-phishing-detection"),
            "Google treats this flag as automation"
        );
        assert!(
            !args.contains("disable-background-networking"),
            "real Edge still phones home; this set is a bot fingerprint"
        );
        assert!(!args.contains("disable-hang-monitor"));
        assert!(args.contains("TrackingPrevention"));
    }

    #[test]
    fn pns_snap_does_not_block_protocol_thread() {
        use std::time::Instant;
        let t0 = Instant::now();
        let v = super::pns_snap_json();
        let ms = t0.elapsed().as_millis();
        eprintln!("pns_snap_json {ms}ms {}B", v.to_string().len());
        assert!(
            ms < 250,
            "pns_snap_json took {ms}ms; Explorer loadPns freezes the chrome. live={:?} loading={:?}",
            v.get("live"),
            v.get("loading")
        );
    }
}
