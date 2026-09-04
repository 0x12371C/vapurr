//! Brave adblock-rust engine. EasyList / EasyPrivacy / uBlock Origin network + cosmetic filters.
//! Bundled core list is always on. Full lists load from cache, then refresh in the background.

use adblock::engine::Engine;
use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const CORE: &str = include_str!("../lists/core.txt");

struct RemoteList {
    url: &'static str,
    file: &'static str,
    /// `privacy` lists drop when the privacy pref is off. `annoy` lists drop when annoyances are off.
    kind: ListKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Ads,
    Privacy,
    Annoy,
    Fix,
}

const REMOTE: &[RemoteList] = &[
    RemoteList {
        url: "https://easylist.to/easylist/easylist.txt",
        file: "easylist.txt",
        kind: ListKind::Ads,
    },
    RemoteList {
        url: "https://easylist.to/easylist/easyprivacy.txt",
        file: "easyprivacy.txt",
        kind: ListKind::Privacy,
    },
    RemoteList {
        url: "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters.min.txt",
        file: "ubo-filters.min.txt",
        kind: ListKind::Ads,
    },
    RemoteList {
        url: "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/privacy.min.txt",
        file: "ubo-privacy.min.txt",
        kind: ListKind::Privacy,
    },
    RemoteList {
        url: "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/badware.min.txt",
        file: "ubo-badware.min.txt",
        kind: ListKind::Fix,
    },
    RemoteList {
        url: "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/quick-fixes.min.txt",
        file: "ubo-quick-fixes.min.txt",
        kind: ListKind::Fix,
    },
    RemoteList {
        url: "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/unbreak.min.txt",
        file: "ubo-unbreak.min.txt",
        kind: ListKind::Fix,
    },
    RemoteList {
        url: "https://secure.fanboy.co.nz/fanboy-cookiemonster.txt",
        file: "fanboy-cookiemonster.txt",
        kind: ListKind::Annoy,
    },
    RemoteList {
        url: "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/annoyances.min.txt",
        file: "ubo-annoyances.min.txt",
        kind: ListKind::Annoy,
    },
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShieldPrefs {
    pub enabled: bool,
    pub privacy: bool,
    pub annoyances: bool,
    pub cosmetic: bool,
}

impl Default for ShieldPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            privacy: true,
            annoyances: true,
            cosmetic: true,
        }
    }
}

pub struct Shield {
    engine: Mutex<Engine>,
    prefs: Mutex<ShieldPrefs>,
    blocked: AtomicU64,
    lists_ready: AtomicBool,
    gen: AtomicU64,
    last: Mutex<String>,
}

impl Shield {
    pub fn new() -> Arc<Self> {
        Self::from_prefs(ShieldPrefs::default(), false)
    }

    pub fn with_prefs(prefs: ShieldPrefs) -> Arc<Self> {
        Self::from_prefs(prefs, true)
    }

    fn from_prefs(prefs: ShieldPrefs, boot: bool) -> Arc<Self> {
        let engine = compile(CORE);
        let s = Arc::new(Self {
            engine: Mutex::new(engine),
            prefs: Mutex::new(prefs),
            blocked: AtomicU64::new(0),
            lists_ready: AtomicBool::new(false),
            gen: AtomicU64::new(0),
            last: Mutex::new(String::new()),
        });
        if boot {
            s.boot();
        }
        s
    }

    pub fn prefs(&self) -> ShieldPrefs {
        *self.prefs.lock().expect("prefs")
    }

    pub fn set_prefs(&self, prefs: ShieldPrefs) {
        *self.prefs.lock().expect("prefs") = prefs;
    }

    pub fn blocked(&self) -> u64 {
        self.blocked.load(Ordering::Relaxed)
    }

    pub fn ready(&self) -> bool {
        self.lists_ready.load(Ordering::Relaxed)
    }

    /// Never block chrome surfaces or the top-level page document.
    pub fn should_block(&self, url: &str, source_url: &str, resource_type: &str) -> bool {
        let prefs = self.prefs();
        if !prefs.enabled {
            return false;
        }
        if is_chrome(url) || is_chrome(source_url) {
            return false;
        }
        if is_pass_host(url) || is_pass_host(source_url) {
            return false;
        }
        if is_google_surface(source_url) || is_challenge(url) || is_faucet_page(source_url) {
            return false;
        }
        if resource_type == "main_frame" || resource_type == "document" {
            return false;
        }
        let Ok(req) = Request::new(url, source_url, resource_type) else {
            return false;
        };
        let Ok(guard) = self.engine.lock() else {
            return false;
        };
        let hit = guard.check_network_request(&req).matched;
        drop(guard);
        if hit {
            self.blocked.fetch_add(1, Ordering::Relaxed);
            if let Ok(mut last) = self.last.lock() {
                *last = host(url);
            }
        }
        hit
    }

    pub fn cosmetic_css(&self, page_url: &str) -> String {
        let prefs = self.prefs();
        if !prefs.enabled || !prefs.cosmetic || is_chrome(page_url) || is_pass_host(page_url) || is_google_surface(page_url) || is_faucet_page(page_url) {
            return String::new();
        }
        let Ok(guard) = self.engine.lock() else {
            return String::new();
        };
        let resources = guard.url_cosmetic_resources(page_url);
        drop(guard);
        let mut css = String::from(BASE_COSMETIC);
        if !resources.hide_selectors.is_empty() {
            let mut sels: Vec<String> = resources.hide_selectors.into_iter().collect();
            sels.sort();
            sels.truncate(2500);
            css.push('\n');
            css.push_str(&sels.join(","));
            css.push_str("{display:none!important}");
        }
        css
    }

    pub fn extra_hide_css(&self, page_url: &str, classes: &[String], ids: &[String]) -> String {
        let prefs = self.prefs();
        if !prefs.enabled || !prefs.cosmetic || is_chrome(page_url) || is_pass_host(page_url) || is_google_surface(page_url) || is_faucet_page(page_url) {
            return String::new();
        }
        if classes.is_empty() && ids.is_empty() {
            return String::new();
        }
        let Ok(guard) = self.engine.lock() else {
            return String::new();
        };
        let resources = guard.url_cosmetic_resources(page_url);
        if resources.generichide {
            return String::new();
        }
        let sels = guard.hidden_class_id_selectors(classes, ids, &resources.exceptions);
        drop(guard);
        if sels.is_empty() {
            return String::new();
        }
        format!("{}{{display:none!important}}", sels.join(","))
    }

    pub fn inject_js(&self, page_url: &str) -> String {
        let css = self.cosmetic_css(page_url);
        if css.is_empty() {
            return String::new();
        }
        format!(
            r#"(function(){{
  var id='vapurr-shield';
  var s=document.getElementById(id);
  if(!s){{
    s=document.createElement('style');
    s.id=id;
    (document.documentElement||document.head||document.body).appendChild(s);
  }}
  s.textContent={css};
  function harvest(){{
    try {{
      var ids=[], classes=[], seen={{}};
      var all=document.querySelectorAll('[id],[class]');
      var n=Math.min(all.length, 500);
      for (var i=0;i<n;i++){{
        var el=all[i];
        if(el.id && !seen['#'+el.id]){{ seen['#'+el.id]=1; ids.push(el.id); }}
        if(el.classList){{
          for(var j=0;j<el.classList.length;j++){{
            var c=el.classList[j];
            if(c && !seen['.'+c]){{ seen['.'+c]=1; classes.push(c); }}
          }}
        }}
      }}
      var p=JSON.stringify({{cmd:'shield-dom', ids:ids.slice(0,200), classes:classes.slice(0,200)}});
      if(window.ipc&&window.ipc.postMessage) window.ipc.postMessage(p);
      else if(window.chrome&&window.chrome.webview) window.chrome.webview.postMessage(p);
    }} catch(e){{}}
  }}
  if(document.readyState==='loading') document.addEventListener('DOMContentLoaded', harvest);
  else harvest();
  setTimeout(harvest, 1400);
}})();"#,
            css = serde_json::to_string(&css).unwrap_or_else(|_| "\"\"".into())
        )
    }

    pub fn extra_inject_js(&self, page_url: &str, classes: &[String], ids: &[String]) -> String {
        let css = self.extra_hide_css(page_url, classes, ids);
        if css.is_empty() {
            return String::new();
        }
        format!(
            r#"(function(){{
  var s=document.getElementById('vapurr-shield');
  if(!s){{
    s=document.createElement('style');
    s.id='vapurr-shield';
    (document.documentElement||document.head||document.body).appendChild(s);
  }}
  s.textContent += {css};
}})();"#,
            css = serde_json::to_string(&format!("\n{css}")).unwrap_or_else(|_| "\"\"".into())
        )
    }

    fn boot(self: &Arc<Self>) {
        self.refresh_remote();
    }

    /// Pull EasyList / EasyPrivacy / uBO in the background and hot-swap the engine.
    pub fn refresh_remote(self: &Arc<Self>) {
        let me = Arc::clone(self);
        let gen = me.gen.fetch_add(1, Ordering::Relaxed) + 1;
        std::thread::Builder::new()
            .name("vapurr-shield".into())
            .spawn(move || {
                let prefs = me.prefs();
                if !prefs.enabled {
                    return;
                }
                let mut text = String::from(CORE);
                text.push('\n');
                let mut from_cache = 0usize;
                for list in REMOTE {
                    if !include_list(list.kind, prefs) {
                        continue;
                    }
                    if let Some(body) = read_cache(list.file) {
                        if body.len() > 80 {
                            text.push_str(&body);
                            text.push('\n');
                            from_cache += 1;
                        }
                    }
                }
                if from_cache >= 2 {
                    let engine = compile(&text);
                    if me.gen.load(Ordering::Relaxed) == gen {
                        if let Ok(mut slot) = me.engine.lock() {
                            *slot = engine;
                        }
                        me.lists_ready.store(true, Ordering::Relaxed);
                        tracing::info!("vapurr-shield: loaded {from_cache} cached lists");
                    }
                }

                let http = match reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(25))
                    .user_agent("vapurr-shield/0.1")
                    .build()
                {
                    Ok(c) => c,
                    Err(_) => return,
                };
                let mut fresh = String::from(CORE);
                fresh.push('\n');
                let mut fetched = 0usize;
                for list in REMOTE {
                    if !include_list(list.kind, prefs) {
                        continue;
                    }
                    match http.get(list.url).send() {
                        Ok(resp) if resp.status().is_success() => {
                            if let Ok(body) = resp.text() {
                                if body.len() > 80 {
                                    write_cache(list.file, &body);
                                    fresh.push_str(&body);
                                    fresh.push('\n');
                                    fetched += 1;
                                }
                            }
                        }
                        Ok(resp) => tracing::warn!(
                            "vapurr-shield: {} -> {}",
                            list.file,
                            resp.status()
                        ),
                        Err(e) => tracing::warn!("vapurr-shield: {} {e}", list.file),
                    }
                }
                if fetched < 2 {
                    return;
                }
                let engine = compile(&fresh);
                if me.gen.load(Ordering::Relaxed) != gen {
                    return;
                }
                if let Ok(mut slot) = me.engine.lock() {
                    *slot = engine;
                }
                me.lists_ready.store(true, Ordering::Relaxed);
                tracing::info!("vapurr-shield: compiled {fetched} remote lists");
            })
            .ok();
    }

    pub fn snapshot(&self) -> serde_json::Value {
        let p = self.prefs();
        serde_json::json!({
            "enabled": p.enabled,
            "privacy": p.privacy,
            "annoyances": p.annoyances,
            "cosmetic": p.cosmetic,
            "blocked": self.blocked(),
            "ready": self.ready(),
            "last": self.last.lock().map(|s| s.clone()).unwrap_or_default(),
        })
    }
}

fn include_list(kind: ListKind, prefs: ShieldPrefs) -> bool {
    match kind {
        ListKind::Ads | ListKind::Fix => true,
        ListKind::Privacy => prefs.privacy,
        ListKind::Annoy => prefs.annoyances,
    }
}

fn compile(text: &str) -> Engine {
    let mut set = FilterSet::new(false);
    set.add_filter_list(text, ParseOptions::default());
    Engine::from_filter_set(set, true)
}

fn cache_dir() -> std::path::PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|h| std::path::PathBuf::from(h).join("AppData/Local"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let dir = base.join("vapurr").join("shield");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn read_cache(file: &str) -> Option<String> {
    let p = cache_dir().join(file);
    let bytes = std::fs::read(p).ok()?;
    if bytes.len() < 80 {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn write_cache(file: &str, body: &str) {
    let p = cache_dir().join(file);
    let _ = std::fs::write(p, body.as_bytes());
}

fn is_chrome(url: &str) -> bool {
    url.contains("vapurr.localhost")
        || url.starts_with("vapurr://")
        || url.starts_with("http://vapurr.")
}

/// Guest origins whose scripts, sockets, and cosmetics we never touch.
/// fomo.family Connect (Privy + WalletConnect / Reown) lives here.
pub fn is_pass_host(url: &str) -> bool {
    let h = host(url);
    let h = if h.is_empty() {
        url.to_ascii_lowercase()
    } else {
        h
    };
    const ROOTS: &[&str] = &[
        "fomo.family",
        "fomoapi.io",
        "thesecretlab.app",
        "robinhood.com",
        "robinhood.net",
        "walletconnect.com",
        "walletconnect.org",
        "reown.com",
        "privy.io",
        "web3modal.com",
        "web3modal.org",
    ];
    ROOTS.iter().any(|root| host_matches(&h, root))
}

/// Google Search / YouTube / reCAPTCHA. EasyPrivacy on these looks like a bot.
fn is_faucet_page(url: &str) -> bool {
    let h = host(url);
    h == "faucet.testnet.chain.robinhood.com"
        || h.ends_with(".faucet.testnet.chain.robinhood.com")
}

fn is_google_surface(url: &str) -> bool {
    let h = host(url);
    let h = h.trim_start_matches("www.");
    host_matches(h, "google.com")
        || h.starts_with("google.")
        || host_matches(h, "youtube.com")
        || host_matches(h, "youtu.be")
}

fn is_challenge(url: &str) -> bool {
    let u = url.to_ascii_lowercase();
    if u.contains("recaptcha") || u.contains("/sorry/") {
        return true;
    }
    let h = host(&u);
    let h = h.trim_start_matches("www.");
    host_matches(h, "recaptcha.net") || host_matches(h, "gstatic.com")
}

fn host_matches(host: &str, root: &str) -> bool {
    host == root
        || host
            .strip_suffix(root)
            .map(|p| p.ends_with('.'))
            .unwrap_or(false)
}

const BASE_COSMETIC: &str = r#"
iframe[src*="doubleclick"],iframe[src*="googlesyndication"],iframe[id^="google_ads"],
ins.adsbygoogle,[id^="div-gpt-ad"],[id^="google_ads_"],[data-google-query-id],
.adsbygoogle,.ad-slot,.ad-banner,[id^="taboola-"],.OUTBRAIN,[id^="outbrain_"]{
  display:none!important
}
"#;

pub fn resource_type_from_webview(ctx: i32, request_url: &str, page_url: &str) -> &'static str {
    // COREWEBVIEW2_WEB_RESOURCE_CONTEXT_* (webview2-com 0.33).
    match ctx {
        1 => {
            if page_url.is_empty() || same_url(request_url, page_url) {
                "document"
            } else {
                "sub_frame"
            }
        }
        2 => "stylesheet",
        3 => "image",
        4 => "media",
        5 => "font",
        6 => "script",
        7 | 8 => "xhr",
        11 => "websocket",
        14 => "ping",
        15 => "csp_report",
        _ => "other",
    }
}

fn same_url(a: &str, b: &str) -> bool {
    let na = a.split('#').next().unwrap_or(a);
    let nb = b.split('#').next().unwrap_or(b);
    na == nb
}

fn host(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_doubleclick() {
        let s = Shield::new();
        assert!(s.should_block(
            "https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js",
            "https://news.example.com/story",
            "script"
        ));
    }

    #[test]
    fn google_search_is_not_a_bot() {
        let s = Shield::new();
        assert!(!s.should_block(
            "https://www.google.com/gen_204",
            "https://www.google.com/search?q=vapurr",
            "xhr"
        ));
        assert!(!s.should_block(
            "https://www.gstatic.com/recaptcha/releases/x/recaptcha__en.js",
            "https://www.google.com/",
            "script"
        ));
        assert!(!s.should_block(
            "https://www.google.com/recaptcha/api.js",
            "https://news.example.com/login",
            "script"
        ));
        assert!(s.cosmetic_css("https://www.google.com/search?q=x").is_empty());
        assert!(s.should_block(
            "https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js",
            "https://news.example.com/story",
            "script"
        ));
    }

    #[test]
    fn faucet_page_is_not_stripped() {
        let s = Shield::new();
        assert!(!s.should_block(
            "https://accounts.google.com/gsi/client",
            "https://faucet.testnet.chain.robinhood.com/",
            "script"
        ));
        assert!(!s.should_block(
            "https://challenges.cloudflare.com/turnstile/v0/api.js",
            "https://faucet.testnet.chain.robinhood.com/",
            "script"
        ));
        assert!(s
            .cosmetic_css("https://faucet.testnet.chain.robinhood.com/")
            .is_empty());
    }

    #[test]
    fn passes_fomo_family() {
        let s = Shield::new();
        assert!(!s.should_block(
            "https://prod-api.fomo.family/v1/feed",
            "https://fomo.family/",
            "xhr"
        ));
        assert!(!s.should_block(
            "https://api.fomoapi.io/v2/alerts",
            "https://fomo.family/",
            "script"
        ));
        assert!(!s.should_block(
            "https://cdn.robinhood.com/assets/app.js",
            "https://robinhood.com/wallet/",
            "script"
        ));
        assert!(!s.should_block(
            "wss://relay.walletconnect.com/",
            "https://fomo.family/",
            "websocket"
        ));
        assert!(!s.should_block(
            "https://auth.privy.io/api/v1/siwe/init",
            "https://news.example.com/",
            "xhr"
        ));
        assert!(!s.should_block(
            "https://api.web3modal.com/getWallets",
            "https://fomo.family/",
            "xhr"
        ));
        assert!(!s.should_block(
            "https://thesecretlab.app/kyc",
            "https://thesecretlab.app/kyc",
            "xhr"
        ));
        assert!(s.cosmetic_css("https://thesecretlab.app/kyc").is_empty());
    }

    #[test]
    fn cosmetic_skips_fomo() {
        let s = Shield::new();
        assert!(s.cosmetic_css("https://fomo.family/").is_empty());
        assert!(s.cosmetic_css("https://auth.privy.io/").is_empty());
        assert!(!s.cosmetic_css("https://example.com/").is_empty());
    }

    #[test]
    fn allows_first_party_document() {
        let s = Shield::new();
        assert!(!s.should_block(
            "https://news.example.com/story",
            "https://news.example.com/story",
            "document"
        ));
    }

    #[test]
    fn skips_chrome_surfaces() {
        let s = Shield::new();
        assert!(!s.should_block(
            "https://pagead2.googlesyndication.com/x.js",
            "http://vapurr.localhost/home.html",
            "script"
        ));
    }

    #[test]
    fn cosmetic_emits_hide_css() {
        let s = Shield::new();
        let css = s.cosmetic_css("https://example.com/");
        assert!(css.contains("adsbygoogle"));
    }

    #[test]
    fn master_switch_disables() {
        let s = Shield::new();
        s.set_prefs(ShieldPrefs {
            enabled: false,
            ..ShieldPrefs::default()
        });
        assert!(!s.should_block(
            "https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js",
            "https://news.example.com/story",
            "script"
        ));
    }

    #[test]
    fn blocks_ad_iframe_as_sub_frame() {
        let s = Shield::new();
        assert!(s.should_block(
            "https://tpc.googlesyndication.com/safeframe/1",
            "https://news.example.com/story",
            "sub_frame"
        ));
    }

    #[test]
    fn document_context_stays_main_frame() {
        assert_eq!(
            resource_type_from_webview(
                1,
                "https://news.example.com/story",
                "https://news.example.com/story"
            ),
            "document"
        );
        assert_eq!(
            resource_type_from_webview(
                1,
                "https://tpc.googlesyndication.com/safeframe",
                "https://news.example.com/story"
            ),
            "sub_frame"
        );
        assert_eq!(resource_type_from_webview(6, "https://x.com/a.js", "https://x.com/"), "script");
    }
}
