//! Process policy: chrome is not a website.

use serde::{Deserialize, Serialize};
use std::fmt;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SiteKey(pub String);

impl SiteKey {
    pub fn from_url(raw: &str) -> Result<Self, CoreError> {
        if let Some(rest) = raw.strip_prefix("vapurr://") {
            return Ok(SiteKey(format!("vapurr:{}", rest.split('/').next().unwrap_or("home"))));
        }
        let url = Url::parse(raw).map_err(|_| CoreError::BadUrl)?;
        match url.scheme() {
            "https" | "http" => {
                let host = url.host_str().ok_or(CoreError::BadUrl)?;
                Ok(SiteKey(registrable(host)))
            }
            other => Ok(SiteKey(format!("{other}:{}", url.host_str().unwrap_or("")))),
        }
    }

    pub fn is_chrome_surface(&self) -> bool {
        self.0.starts_with("vapurr:")
    }
}

fn registrable(host: &str) -> String {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host.parse::<std::net::IpAddr>().is_ok() {
        return host;
    }
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() <= 2 {
        return host;
    }
    // cheap eTLD+1: last two labels. good enough to share processes for v1.
    format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessKind {
    Shell,
    Site,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessPolicy;

impl ProcessPolicy {
    pub fn kind_for(site: &SiteKey) -> ProcessKind {
        if site.is_chrome_surface() {
            ProcessKind::Shell
        } else {
            ProcessKind::Site
        }
    }

    /// Tabs on the same eTLD+1 share a content process. Chrome surfaces never get one.
    pub fn share_process(a: &SiteKey, b: &SiteKey) -> bool {
        if a.is_chrome_surface() || b.is_chrome_surface() {
            return false;
        }
        a == b
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub site: SiteKey,
    pub frozen: bool,
}

impl Tab {
    pub fn new(id: u64, url: &str) -> Result<Self, CoreError> {
        let site = SiteKey::from_url(url)?;
        Ok(Self {
            id,
            url: url.to_string(),
            title: String::new(),
            site,
            frozen: false,
        })
    }

    pub fn freeze(&mut self) {
        if !self.site.is_chrome_surface() {
            self.frozen = true;
        }
    }

    pub fn thaw(&mut self) {
        self.frozen = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub handle: Option<String>,
    pub verified: bool,
    pub card_linked: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            handle: None,
            verified: false,
            card_linked: false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("bad url")]
    BadUrl,
}

impl fmt::Display for SiteKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_key_collapses_subdomains() {
        let a = SiteKey::from_url("https://a.news.example.com/x").unwrap();
        let b = SiteKey::from_url("https://b.news.example.com/y").unwrap();
        assert_eq!(a, b);
        assert!(ProcessPolicy::share_process(&a, &b));
    }

    #[test]
    fn chrome_surfaces_stay_in_shell() {
        let f = SiteKey::from_url("vapurr://fomo").unwrap();
        assert_eq!(f.0, "vapurr:fomo");
        let w = SiteKey::from_url("vapurr://wallet").unwrap();
        assert!(w.is_chrome_surface());
        assert_eq!(ProcessPolicy::kind_for(&w), ProcessKind::Shell);
        let web = SiteKey::from_url("https://example.com").unwrap();
        assert!(!ProcessPolicy::share_process(&w, &web));
    }

    #[test]
    fn freeze_only_content() {
        let mut t = Tab::new(1, "https://example.com").unwrap();
        t.freeze();
        assert!(t.frozen);
        let mut chrome = Tab::new(2, "vapurr://zzzmail").unwrap();
        chrome.freeze();
        assert!(!chrome.frozen);
    }
}
