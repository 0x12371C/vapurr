//! Local desk: history, bookmarks, opt-in browsing receipts.
//! Earn payload is host + https + time. No path, query, cookies, or title.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;
use vapurr_core::SiteKey;

const HISTORY_CAP: usize = 400;
const VISIT_CAP: usize = 2000;
const VISIT_DWELL: u64 = 30 * 60;
const RECEIPT_HOSTS: usize = 48;
const PUSD_MINOR_PER_VISIT: u128 = 1_000; // $0.001 $PUSD

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub url: String,
    pub title: String,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRow {
    pub url: String,
    pub title: String,
    pub host: String,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Visit {
    pub host: String,
    pub https: bool,
    pub ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub id: u64,
    pub ts: u64,
    #[serde(default)]
    pub from: u64,
    #[serde(default)]
    pub to: u64,
    pub visits: u32,
    pub hosts: u32,
    #[serde(default)]
    pub https: u32,
    #[serde(default)]
    pub host_list: Vec<String>,
    pub usdg_minor: u128,
    #[serde(default)]
    pub pusd_minor: u128,
    pub status: String,
    #[serde(default)]
    pub hash: String,
}

impl Receipt {
    fn amount(&self) -> u128 {
        if self.pusd_minor > 0 {
            self.pusd_minor
        } else {
            self.usdg_minor
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EarnClaim {
    Empty,
    Sealed(Receipt),
    Queued(Receipt),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefs {
    #[serde(default = "default_zoom")]
    pub zoom: f64,
    #[serde(default = "default_homepage")]
    pub homepage: String,
    #[serde(default = "default_true")]
    pub ctrl_scroll_zoom: bool,
    #[serde(default = "default_true")]
    pub show_zoom_chip: bool,
    #[serde(default)]
    pub restore_last: bool,
    #[serde(default = "default_true")]
    pub adblock: bool,
    #[serde(default = "default_true")]
    pub adblock_privacy: bool,
    #[serde(default = "default_true")]
    pub adblock_annoyances: bool,
    #[serde(default = "default_true")]
    pub adblock_cosmetic: bool,
    #[serde(default)]
    pub boost: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Idle auto-lock seconds. Default 900 (15 min). 0 disables.
    #[serde(default = "default_lock_timeout")]
    pub lock_timeout_secs: u64,
}

fn default_zoom() -> f64 {
    1.0
}
fn default_homepage() -> String {
    "vapurr://home".into()
}
fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "dark".into()
}

fn default_lock_timeout() -> u64 {
    900
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            homepage: default_homepage(),
            ctrl_scroll_zoom: true,
            show_zoom_chip: true,
            restore_last: false,
            adblock: true,
            adblock_privacy: true,
            adblock_annoyances: true,
            adblock_cosmetic: true,
            boost: false,
            theme: default_theme(),
            lock_timeout_secs: default_lock_timeout(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Desk {
    #[serde(default)]
    pub opt_in: bool,
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
    #[serde(default)]
    pub history: Vec<HistoryRow>,
    #[serde(default)]
    pub visits: Vec<Visit>,
    #[serde(default)]
    pub receipts: Vec<Receipt>,
    #[serde(default)]
    pub pending_usdg_minor: u128,
    #[serde(default)]
    pub paid_usdg_minor: u128,
    #[serde(default)]
    next_receipt: u64,
    #[serde(default)]
    pub prefs: Prefs,
    #[serde(default)]
    pub last_url: String,
    #[serde(skip)]
    pub last_earn_error: Option<String>,
}

impl Default for Desk {
    fn default() -> Self {
        Self {
            opt_in: false,
            bookmarks: vec![],
            history: vec![],
            visits: vec![],
            receipts: vec![],
            pending_usdg_minor: 0,
            paid_usdg_minor: 0,
            next_receipt: 1,
            prefs: Prefs::default(),
            last_url: String::new(),
            last_earn_error: None,
        }
    }
}

impl Desk {
    pub fn profile_dir() -> PathBuf {
        let base = std::env::var("LOCALAPPDATA")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("USERPROFILE")
                    .ok()
                    .map(|h| PathBuf::from(h).join("AppData/Local"))
            })
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("vapurr")
    }

    pub fn path() -> PathBuf {
        Self::profile_dir().join("desk.json")
    }

    fn legacy_path() -> PathBuf {
        // env!(CARGO_MANIFEST_DIR) embeds compile-host paths (not remapped).
        // Dev-only migration from repo data/; release must not leak C:\Users\<builder>.
        #[cfg(debug_assertions)]
        {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/desk.json")
        }
        #[cfg(not(debug_assertions))]
        {
            PathBuf::from("data/desk.json")
        }
    }

    pub fn load() -> Self {
        let p = Self::path();
        if !p.exists() {
            let legacy = Self::legacy_path();
            if legacy.exists() {
                if let Some(dir) = p.parent() {
                    let _ = fs::create_dir_all(dir);
                }
                let _ = fs::copy(&legacy, &p);
            }
        }
        let mut d = if let Ok(bytes) = fs::read(&p) {
            serde_json::from_slice::<Desk>(&bytes).unwrap_or_default()
        } else {
            Self::default()
        };
        let hist = d.compact_history();
        let stars = d.compact_bookmarks();
        if hist || stars {
            d.save();
        }
        d
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(self) {
            let _ = fs::write(p, bytes);
        }
    }

    pub fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn record_nav(&mut self, url: &str, title: &str) {
        let ts = Self::now();
        if !self.ingest_nav(url, title, ts) {
            return;
        }
        self.save();
    }

    /// Returns true if history (or last_url) changed.
    pub fn ingest_nav(&mut self, url: &str, title: &str, ts: u64) -> bool {
        if is_history_noise(url) {
            return false;
        }
        let url = canonicalize_url(url);
        if is_history_noise(&url) {
            return false;
        }
        let host = SiteKey::from_url(&url)
            .map(|k| k.0)
            .unwrap_or_else(|_| "unknown".into());
        if host.starts_with("vapurr:") {
            return false;
        }
        let title = pretty_title(title, &url, &host);
        let key = star_key(&url);
        let https = url.starts_with("https://");
        let mut changed = false;
        if let Some(last) = self.history.last_mut() {
            if star_key(&last.url) == key {
                last.title = better_title(&last.title, &title, &host);
                last.url = url.clone();
                last.host = host.clone();
                last.ts = ts;
                changed = true;
            } else if last.host == host
                && ts.saturating_sub(last.ts) <= 8
                && looks_like_hop(&last.url)
            {
                last.url = url.clone();
                last.title = better_title(&last.title, &title, &host);
                last.host = host.clone();
                last.ts = ts;
                changed = true;
            }
        }
        if !changed {
            self.history.push(HistoryRow {
                url: url.clone(),
                title,
                host: host.clone(),
                ts,
            });
            if self.history.len() > HISTORY_CAP {
                let extra = self.history.len() - HISTORY_CAP;
                self.history.drain(0..extra);
            }
            changed = true;
        }
        if self.opt_in {
            let same_visit = self.visits.iter().rev().any(|v| {
                v.host == host && ts.saturating_sub(v.ts) <= VISIT_DWELL
            });
            if !same_visit {
                self.visits.push(Visit { host, https, ts });
                if self.visits.len() > VISIT_CAP {
                    let extra = self.visits.len() - VISIT_CAP;
                    self.visits.drain(0..extra);
                }
            }
        }
        self.last_url = url;
        changed
    }

    pub fn touch_title(&mut self, url: &str, title: &str) -> bool {
        if title.trim().is_empty() || is_history_noise(url) {
            return false;
        }
        let key = star_key(url);
        let Some(row) = self
            .history
            .iter_mut()
            .rev()
            .find(|h| star_key(&h.url) == key)
        else {
            return false;
        };
        let host = row.host.clone();
        let next = better_title(&row.title, title, &host);
        if next == row.title {
            return false;
        }
        row.title = next;
        self.save();
        true
    }

    pub fn compact_history(&mut self) -> bool {
        let old = self.history.len();
        let mut out: Vec<HistoryRow> = Vec::new();
        for row in std::mem::take(&mut self.history) {
            if is_history_noise(&row.url) {
                continue;
            }
            let url = canonicalize_url(&row.url);
            if is_history_noise(&url) {
                continue;
            }
            let host = SiteKey::from_url(&url)
                .map(|k| k.0)
                .unwrap_or(row.host.clone());
            if host.starts_with("vapurr:") {
                continue;
            }
            let title = pretty_title(&row.title, &url, &host);
            let key = star_key(&url);
            if let Some(last) = out.last_mut() {
                if star_key(&last.url) == key {
                    last.title = better_title(&last.title, &title, &host);
                    last.url = url;
                    last.host = host;
                    last.ts = last.ts.max(row.ts);
                    continue;
                }
            }
            out.push(HistoryRow {
                url,
                title,
                host,
                ts: row.ts,
            });
        }
        let changed = out.len() != old;
        self.history = out;
        changed
    }

    pub fn compact_bookmarks(&mut self) -> bool {
        let old = self.bookmarks.clone();
        let mut out: Vec<Bookmark> = Vec::new();
        for b in std::mem::take(&mut self.bookmarks) {
            if b.url.trim().is_empty() {
                continue;
            }
            let url = chrome_url(&b.url).unwrap_or_else(|| canonicalize_url(&b.url));
            let key = star_key(&url);
            if out.iter().any(|x| star_key(&x.url) == key) {
                continue;
            }
            let host = SiteKey::from_url(&url)
                .map(|k| k.0)
                .unwrap_or_else(|_| url.clone());
            let title = pretty_title(&b.title, &url, &host);
            out.push(Bookmark {
                url,
                title,
                ts: b.ts,
            });
        }
        let changed = out.len() != old.len()
            || out
                .iter()
                .zip(old.iter())
                .any(|(a, b)| a.url != b.url || a.title != b.title);
        self.bookmarks = out;
        changed
    }

    pub fn set_zoom(&mut self, z: f64) {
        self.prefs.zoom = z;
        self.save();
    }

    pub fn set_homepage(&mut self, url: &str) {
        let u = url.trim();
        if u.is_empty() {
            self.prefs.homepage = default_homepage();
        } else {
            self.prefs.homepage = u.to_string();
        }
        self.save();
    }

    pub fn set_ctrl_scroll_zoom(&mut self, on: bool) {
        self.prefs.ctrl_scroll_zoom = on;
        self.save();
    }

    pub fn set_show_zoom_chip(&mut self, on: bool) {
        self.prefs.show_zoom_chip = on;
        self.save();
    }

    pub fn set_restore_last(&mut self, on: bool) {
        self.prefs.restore_last = on;
        self.save();
    }

    pub fn set_adblock(&mut self, on: bool) {
        self.prefs.adblock = on;
        self.save();
    }

    pub fn set_adblock_privacy(&mut self, on: bool) {
        self.prefs.adblock_privacy = on;
        self.save();
    }

    pub fn set_adblock_annoyances(&mut self, on: bool) {
        self.prefs.adblock_annoyances = on;
        self.save();
    }

    pub fn set_adblock_cosmetic(&mut self, on: bool) {
        self.prefs.adblock_cosmetic = on;
        self.save();
    }

    pub fn set_boost(&mut self, on: bool) {
        self.prefs.boost = on;
        self.save();
    }

    pub fn set_theme(&mut self, theme: &str) {
        self.prefs.theme = if theme.eq_ignore_ascii_case("light") {
            "light".into()
        } else {
            "dark".into()
        };
        self.save();
    }

    pub fn set_lock_timeout_secs(&mut self, secs: u64) {
        // Clamp to sane set: 0 (off) or 60..=86400
        self.prefs.lock_timeout_secs = if secs == 0 {
            0
        } else {
            secs.clamp(60, 86_400)
        };
        self.save();
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.save();
    }

    pub fn clear_bookmarks(&mut self) {
        self.bookmarks.clear();
        self.save();
    }

    pub fn clear_visits(&mut self) {
        self.visits.clear();
        self.save();
    }

    pub fn is_starred(&self, url: &str) -> bool {
        if url.trim().is_empty() {
            return false;
        }
        let key = star_key(url);
        self.bookmarks.iter().any(|b| star_key(&b.url) == key)
    }

    pub fn toggle_star(&mut self, url: &str, title: &str) -> bool {
        let on = self.toggle_star_inner(url, title, Self::now());
        self.save();
        on
    }

    pub fn toggle_star_inner(&mut self, url: &str, title: &str, ts: u64) -> bool {
        if url.trim().is_empty() {
            return false;
        }
        let stored = chrome_url(url).unwrap_or_else(|| canonicalize_url(url));
        if stored.is_empty() {
            return false;
        }
        let key = star_key(&stored);
        if let Some(i) = self.bookmarks.iter().position(|b| star_key(&b.url) == key) {
            self.bookmarks.remove(i);
            return false;
        }
        let host = SiteKey::from_url(&stored)
            .map(|k| k.0)
            .unwrap_or_else(|_| stored.clone());
        let title = pretty_title(title, &stored, &host);
        self.bookmarks.push(Bookmark {
            url: stored,
            title,
            ts,
        });
        true
    }

    pub fn set_opt_in(&mut self, on: bool) {
        self.opt_in = on;
        self.save();
    }

    pub fn submit(&mut self, kyc_proven: bool) -> EarnClaim {
        let out = self.submit_inner(kyc_proven, Self::now());
        if matches!(out, EarnClaim::Queued(_)) {
            self.save();
        }
        out
    }

    pub fn submit_inner(&mut self, kyc_proven: bool, ts: u64) -> EarnClaim {
        if self.visits.is_empty() {
            self.last_earn_error = if kyc_proven {
                None
            } else {
                Some("Seal a window first. Payout still needs KYC.".into())
            };
            if kyc_proven {
                self.promote_held();
            }
            return EarnClaim::Empty;
        }
        let rec = self.seal_window(ts, kyc_proven);
        if kyc_proven {
            self.promote_held();
            self.last_earn_error = None;
            EarnClaim::Queued(rec)
        } else {
            self.last_earn_error = Some(
                "Receipt sealed. Payout needs zer0ID KYC on this install.".into(),
            );
            EarnClaim::Sealed(rec)
        }
    }

    fn seal_window(&mut self, ts: u64, kyc_proven: bool) -> Receipt {
        let visits_n = self.visits.len() as u32;
        let https = self.visits.iter().filter(|v| v.https).count() as u32;
        let from = self.visits.iter().map(|v| v.ts).min().unwrap_or(ts);
        let to = self.visits.iter().map(|v| v.ts).max().unwrap_or(ts);
        let mut hosts: Vec<String> = self.visits.iter().map(|v| v.host.clone()).collect();
        hosts.sort();
        hosts.dedup();
        if hosts.len() > RECEIPT_HOSTS {
            hosts.truncate(RECEIPT_HOSTS);
        }
        let host_n = hosts.len() as u32;
        let minor = (visits_n as u128) * PUSD_MINOR_PER_VISIT;
        let id = self.next_receipt;
        let hash = receipt_hash(id, from, to, visits_n, &hosts);
        let status = if kyc_proven { "queued" } else { "held" };
        let rec = Receipt {
            id,
            ts,
            from,
            to,
            visits: visits_n,
            hosts: host_n,
            https,
            host_list: hosts,
            usdg_minor: minor,
            pusd_minor: minor,
            status: status.into(),
            hash,
        };
        self.next_receipt += 1;
        if kyc_proven {
            self.pending_usdg_minor = self.pending_usdg_minor.saturating_add(minor);
        }
        self.visits.clear();
        self.receipts.insert(0, rec.clone());
        if self.receipts.len() > 50 {
            self.receipts.truncate(50);
        }
        rec
    }

    fn promote_held(&mut self) {
        for r in &mut self.receipts {
            if r.status == "held" {
                r.status = "queued".into();
                self.pending_usdg_minor = self.pending_usdg_minor.saturating_add(r.amount());
            }
        }
    }

    pub fn snapshot(&self) -> serde_json::Value {
        let mut hosts: Vec<String> = self.visits.iter().map(|v| v.host.clone()).collect();
        hosts.sort();
        hosts.dedup();
        let history: Vec<serde_json::Value> = self
            .history
            .iter()
            .rev()
            .take(200)
            .map(|h| {
                serde_json::json!({
                    "url": h.url,
                    "title": h.title,
                    "host": h.host,
                    "ts": h.ts,
                    "starred": self.is_starred(&h.url),
                    "display": display_url(&h.url),
                })
            })
            .collect();
        let bookmarks: Vec<serde_json::Value> = self
            .bookmarks
            .iter()
            .rev()
            .map(|b| {
                serde_json::json!({
                    "url": b.url,
                    "title": b.title,
                    "ts": b.ts,
                    "host": SiteKey::from_url(&b.url).map(|k| k.0).unwrap_or_default(),
                    "display": display_url(&b.url),
                    "starred": true,
                })
            })
            .collect();
        let downloads = std::env::var("USERPROFILE")
            .map(|h| format!("{h}\\Downloads"))
            .unwrap_or_else(|_| ".".into());
        let pending = format!("{:.3}", self.pending_usdg_minor as f64 / 1_000_000.0);
        let paid = format!("{:.3}", self.paid_usdg_minor as f64 / 1_000_000.0);
        let kyc = vapurr_id::load_verified(&Self::profile_dir());
        let install_id = fs::read_to_string(Self::profile_dir().join("install_id"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let window_minor = (self.visits.len() as u128) * PUSD_MINOR_PER_VISIT;
        let window = if self.visits.is_empty() {
            serde_json::Value::Null
        } else {
            let https = self.visits.iter().filter(|v| v.https).count();
            serde_json::json!({
                "visits": self.visits.len(),
                "hosts": hosts.len(),
                "host_list": hosts,
                "https": https,
                "from": self.visits.iter().map(|v| v.ts).min(),
                "to": self.visits.iter().map(|v| v.ts).max(),
                "pusd_minor": window_minor,
                "pusd": format!("{:.3}", window_minor as f64 / 1_000_000.0),
            })
        };
        serde_json::json!({
            "opt_in": self.opt_in,
            "pending_visits": self.visits.len(),
            "window_pusd": format!("{:.3}", window_minor as f64 / 1_000_000.0),
            "window_pusd_minor": window_minor,
            "pending_usdg": pending,
            "pending_usdg_minor": self.pending_usdg_minor,
            "pending_pusd": pending,
            "pending_pusd_minor": self.pending_usdg_minor,
            "paid_usdg": paid,
            "paid_pusd": paid,
            "rate": "$0.001 / host session",
            "bookmarks": bookmarks,
            "history": history,
            "receipts": self.receipts,
            "hosts_unsubmitted": hosts,
            "window": window,
            "install_id": install_id,
            "prefs": self.prefs,
            "zoom": self.prefs.zoom,
            "zoom_pct": (self.prefs.zoom * 100.0).round() as i64,
            "downloads": downloads,
            "last_url": self.last_url,
            "kyc_proven": kyc.as_ref().map(vapurr_id::payout_ready).unwrap_or(false),
            "kyc_handle": kyc.as_ref().map(|a| a.handle.clone()),
            "kyc_url": vapurr_id::KYC_URL,
            "earn_error": self.last_earn_error,
        })
    }
}

const TRACKING_KEYS: &[&str] = &[
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "utm_id",
    "gclid",
    "gbraid",
    "wbraid",
    "fbclid",
    "mc_eid",
    "msclkid",
    "dclid",
    "twclid",
    "yclid",
    "_ga",
    "_gl",
    "igshid",
    "mc_cid",
];

pub fn chrome_url(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if let Some(rest) = raw.strip_prefix("vapurr://") {
        let id = rest.trim_matches('/').split('?').next().unwrap_or("home");
        let id = if id.is_empty() { "home" } else { id };
        let id = match id {
            "ketpay" => "pay",
            other => other,
        };
        return Some(format!("vapurr://{id}"));
    }
    if raw.contains("vapurr.localhost") {
        let after = raw.split("vapurr.localhost/").nth(1).unwrap_or("");
        let page = after.split('?').next().unwrap_or("home.html");
        if page.starts_with("ketbook") {
            return Some("vapurr://ketbook".into());
        }
        let stem = page.trim_end_matches('/').trim_end_matches(".html");
        let id = if stem.is_empty() { "home" } else { stem };
        let id = match id {
            "ketpay" => "pay",
            other => other,
        };
        return Some(format!("vapurr://{id}"));
    }
    None
}

pub fn canonicalize_url(raw: &str) -> String {
    let raw = raw.trim();
    if let Some(c) = chrome_url(raw) {
        return c;
    }
    let Ok(mut u) = Url::parse(raw) else {
        return raw.to_string();
    };
    if !matches!(u.scheme(), "http" | "https") {
        return raw.to_string();
    }
    u.set_fragment(None);
    if let Some(h) = u.host_str() {
        let h = h.to_ascii_lowercase();
        let _ = u.set_host(Some(&h));
    }
    if (u.scheme() == "https" && u.port() == Some(443))
        || (u.scheme() == "http" && u.port() == Some(80))
    {
        let _ = u.set_port(None);
    }
    let kept: Vec<(String, String)> = u
        .query_pairs()
        .filter(|(k, _)| !is_tracking_key(k))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if kept.is_empty() {
        u.set_query(None);
    } else {
        u.query_pairs_mut().clear();
        for (k, v) in &kept {
            u.query_pairs_mut().append_pair(k, v);
        }
    }
    let path = u.path().to_string();
    if path.ends_with('/') && path.len() > 1 {
        u.set_path(path.trim_end_matches('/'));
    }
    let mut s = u.to_string();
    if (u.path() == "/" || u.path().is_empty()) && u.query().is_none() && s.ends_with('/') {
        s.pop();
    }
    s
}

pub fn star_key(raw: &str) -> String {
    let c = canonicalize_url(raw);
    if c.starts_with("vapurr://") {
        return c;
    }
    let Ok(mut u) = Url::parse(&c) else {
        return c.to_ascii_lowercase();
    };
    if let Some(h) = u.host_str() {
        let h = h.trim_start_matches("www.").to_ascii_lowercase();
        let _ = u.set_host(Some(&h));
    }
    u.set_fragment(None);
    let mut s = u.to_string();
    if (u.path() == "/" || u.path().is_empty()) && u.query().is_none() && s.ends_with('/') {
        s.pop();
    }
    s
}

pub fn display_url(raw: &str) -> String {
    if let Some(c) = chrome_url(raw) {
        return c;
    }
    let Ok(u) = Url::parse(raw) else {
        return raw.to_string();
    };
    let host = u.host_str().unwrap_or("");
    let path = u.path();
    if path.is_empty() || path == "/" {
        host.to_string()
    } else if path.len() > 48 {
        format!("{host}{}â€¦", &path[..48])
    } else {
        format!("{host}{path}")
    }
}

fn receipt_hash(id: u64, from: u64, to: u64, visits: u32, hosts: &[String]) -> String {
    let mut h = Sha256::new();
    h.update(b"vapurr-earn/1\n");
    h.update(format!("{id}\n{from}\n{to}\n{visits}\n").as_bytes());
    for host in hosts {
        h.update(host.as_bytes());
        h.update(b"\n");
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub fn is_history_noise(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    if u.is_empty()
        || u == "about:blank"
        || u.starts_with("about:")
        || u.starts_with("blob:")
        || u.starts_with("data:")
        || u.starts_with("javascript:")
        || u.starts_with("chrome-error")
        || u.contains("vapurr.localhost")
        || u.starts_with("vapurr://")
    {
        return true;
    }
    const PATHS: &[&str] = &[
        "/checkcookie",
        "/setsid",
        "/servicelogin",
        "/signin/oauth",
        "/signin/identifier",
        "/signin/challenge",
        "/signin/continue",
        "/v3/signin/",
        "/accounts/setsid",
        "/rotatecookies",
        "/o/oauth2/",
        "/oauth2/auth",
        "/oauth/authorize",
        "/oauth/v2/",
    ];
    if PATHS.iter().any(|p| u.contains(p)) {
        return true;
    }
    const HOSTS: &[&str] = &[
        "gds.google.com/",
        "accounts.youtube.com/accounts/",
        "login.microsoftonline.com/",
        "login.live.com/oauth",
        "appleid.apple.com/auth",
        "facebook.com/login.php",
        "api.twitter.com/oauth",
        "x.com/i/oauth",
        "accounts.google.com/info/",
        "accounts.google.com/signin/",
        "accounts.google.com/v3/",
        "accounts.google.com/servicelogin",
        "accounts.google.com/checkcookie",
    ];
    HOSTS.iter().any(|h| u.contains(h))
}

fn is_tracking_key(k: &str) -> bool {
    let k = k.to_ascii_lowercase();
    TRACKING_KEYS.iter().any(|t| k == *t) || k.starts_with("utm_")
}

fn looks_like_hop(url: &str) -> bool {
    if is_history_noise(url) {
        return true;
    }
    let Ok(u) = Url::parse(url) else {
        return false;
    };
    let path = u.path();
    let q = u.query().unwrap_or("");
    ((path == "/" || path.is_empty()) && q.is_empty()) || q.len() > 80 || q.contains("continue=")
}

fn pretty_title(title: &str, url: &str, host: &str) -> String {
    let t = title.trim();
    if t.is_empty() || t.eq_ignore_ascii_case(host) || t == "New Tab" {
        if let Some(c) = chrome_url(url) {
            return c.trim_start_matches("vapurr://").to_string();
        }
        return host.to_string();
    }
    t.chars().take(120).collect()
}

fn better_title(a: &str, b: &str, host: &str) -> String {
    let weak = |t: &str| t.is_empty() || t.eq_ignore_ascii_case(host) || t == "New Tab";
    if !weak(b) {
        b.chars().take(120).collect()
    } else if !weak(a) {
        a.to_string()
    } else if !b.is_empty() {
        b.to_string()
    } else {
        a.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_strips_tracking_and_slash() {
        let u = canonicalize_url("https://WWW.Example.com/path/?utm_source=x&v=1#frag");
        assert_eq!(u, "https://www.example.com/path?v=1");
        assert_eq!(
            canonicalize_url("https://youtube.com/"),
            "https://youtube.com"
        );
    }

    #[test]
    fn star_key_collapses_www() {
        assert_eq!(
            star_key("https://youtube.com/"),
            star_key("https://www.youtube.com")
        );
        assert_eq!(
            star_key("http://vapurr.localhost/fomo.html"),
            star_key("vapurr://fomo")
        );
    }

    #[test]
    fn chrome_pages_are_starrable() {
        let mut d = Desk::default();
        assert!(d.toggle_star_inner("http://vapurr.localhost/fomo.html", "fomo", 1));
        assert!(d.is_starred("vapurr://fomo"));
        assert!(d.is_starred("http://vapurr.localhost/fomo.html?x=1"));
        assert!(!d.toggle_star_inner("vapurr://fomo", "", 2));
        assert!(d.bookmarks.is_empty());
    }

    #[test]
    fn star_survives_www_redirect() {
        let mut d = Desk::default();
        assert!(d.toggle_star_inner("https://youtube.com/", "YouTube", 1));
        assert!(d.is_starred("https://www.youtube.com/"));
        assert_eq!(d.bookmarks[0].url, "https://youtube.com");
    }

    #[test]
    fn history_skips_oauth_and_coalesces() {
        let mut d = Desk::default();
        assert!(!d.ingest_nav(
            "https://accounts.google.com/CheckCookie?continue=https://www.youtube.com",
            "YouTube",
            10
        ));
        assert!(d.ingest_nav("https://youtube.com/", "youtube.com", 11));
        assert!(d.ingest_nav("https://www.youtube.com/", "YouTube", 12));
        assert_eq!(d.history.len(), 1);
        assert_eq!(d.history[0].title, "YouTube");
        assert_eq!(d.history[0].url, "https://www.youtube.com");
    }

    #[test]
    fn compact_drops_signin_spam() {
        let mut d = Desk::default();
        d.history = vec![
            HistoryRow {
                url: "https://youtube.com/".into(),
                title: "youtube.com".into(),
                host: "youtube.com".into(),
                ts: 1,
            },
            HistoryRow {
                url: "https://www.youtube.com/".into(),
                title: "YouTube".into(),
                host: "youtube.com".into(),
                ts: 2,
            },
            HistoryRow {
                url: "https://accounts.google.com/ServiceLogin?service=youtube".into(),
                title: "YouTube".into(),
                host: "google.com".into(),
                ts: 3,
            },
        ];
        assert!(d.compact_history());
        assert_eq!(d.history.len(), 1);
        assert_eq!(d.history[0].title, "YouTube");
    }

    #[test]
    fn touch_title_upgrades_host_placeholder() {
        let mut d = Desk::default();
        d.ingest_nav("https://veil.markets/", "veil.markets", 1);
        assert!(d.touch_title("https://veil.markets/", "VEIL - Private Execution Network"));
        assert!(d.history[0].title.starts_with("VEIL"));
    }

    #[test]
    fn ketpay_aliases_pay() {
        assert_eq!(chrome_url("vapurr://ketpay"), Some("vapurr://pay".into()));
        assert_eq!(chrome_url("vapurr://pay"), Some("vapurr://pay".into()));
    }

    #[test]
    fn same_host_session_is_one_visit() {
        let mut d = Desk::default();
        d.opt_in = true;
        assert!(d.ingest_nav("https://example.com/", "Example", 100));
        assert!(d.ingest_nav("https://example.com/a", "A", 100 + 60));
        assert_eq!(d.visits.len(), 1);
        assert!(d.ingest_nav("https://other.dev/", "Other", 100 + 90));
        assert_eq!(d.visits.len(), 2);
        assert!(d.ingest_nav("https://example.com/b", "B", 100 + VISIT_DWELL + 1));
        assert_eq!(d.visits.len(), 3);
    }

    #[test]
    fn submit_without_kyc_seals_held() {
        let mut d = Desk::default();
        d.visits.push(Visit {
            host: "example.com".into(),
            https: true,
            ts: 10,
        });
        d.visits.push(Visit {
            host: "other.dev".into(),
            https: true,
            ts: 40,
        });
        match d.submit_inner(false, 50) {
            EarnClaim::Sealed(r) => {
                assert_eq!(r.status, "held");
                assert_eq!(r.visits, 2);
                assert_eq!(r.hosts, 2);
                assert_eq!(r.from, 10);
                assert_eq!(r.to, 40);
                assert_eq!(r.hash.len(), 64);
                assert!(r.host_list.contains(&"example.com".into()));
                assert_eq!(r.pusd_minor, 2_000);
            }
            other => panic!("{other:?}"),
        }
        assert!(d.visits.is_empty());
        assert_eq!(d.pending_usdg_minor, 0);
        assert_eq!(d.receipts.len(), 1);
    }

    #[test]
    fn submit_with_kyc_queues_and_promotes_held() {
        let mut d = Desk::default();
        d.visits.push(Visit {
            host: "example.com".into(),
            https: true,
            ts: 1,
        });
        match d.submit_inner(false, 2) {
            EarnClaim::Sealed(r) => assert_eq!(r.status, "held"),
            other => panic!("{other:?}"),
        }
        assert_eq!(d.pending_usdg_minor, 0);
        d.visits.push(Visit {
            host: "next.dev".into(),
            https: true,
            ts: 3,
        });
        match d.submit_inner(true, 4) {
            EarnClaim::Queued(r) => {
                assert_eq!(r.pusd_minor, 1_000);
                assert_eq!(r.status, "queued");
                assert_eq!(r.hash.len(), 64);
            }
            other => panic!("{other:?}"),
        }
        assert!(d.visits.is_empty());
        assert_eq!(d.pending_usdg_minor, 2_000);
        assert_eq!(d.receipts[0].status, "queued");
        assert_eq!(d.receipts[1].status, "queued");
    }
}
