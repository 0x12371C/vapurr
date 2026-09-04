//! Product engine is Servo. Reader path is HTTP + scraper. Not wired to the window.
//! There is no WebView2, wry, or Chromium path.

use vapurr_core::{SiteKey, Tab};
use vapurr_net::{NavigationResult, Net};
use vapurr_pay::Paywall;

#[derive(Debug, Clone)]
pub struct ReaderDocument {
    pub title: String,
    pub text: String,
    pub links: Vec<(String, String)>,
}

pub trait Engine {
    fn navigate(&mut self, tab: &mut Tab, url: &str) -> Result<View, EngineError>;
    fn freeze(&mut self, tab: &mut Tab);
    fn thaw(&mut self, tab: &mut Tab);
    fn memory_budget_bytes(&self) -> u64;
}

#[derive(Debug, Clone)]
pub enum View {
    Reader(ReaderDocument),
    Paywall(Paywall),
    Status { code: u16, body: String },
}

pub struct FetcherEngine {
    net: Net,
}

impl FetcherEngine {
    pub fn new() -> Self {
        Self { net: Net::new() }
    }
}

impl Default for FetcherEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for FetcherEngine {
    fn navigate(&mut self, tab: &mut Tab, url: &str) -> Result<View, EngineError> {
        tab.url = url.to_string();
        tab.site = SiteKey::from_url(url).map_err(|_| EngineError::BadUrl)?;
        tab.frozen = false;
        let nav = self.net.navigate(url).map_err(|_| EngineError::Net)?;
        if let Some(p) = nav.paywall {
            tab.title = format!("pay {}", p.amount_label);
            return Ok(View::Paywall(p));
        }
        if nav.status == 404 {
            tab.title = "not found".into();
            return Ok(View::Status {
                code: 404,
                body: "This page is gone. Not a bill.".into(),
            });
        }
        if nav.content_type.contains("html") || nav.body.trim_start().starts_with('<') {
            let doc = read_html(&nav);
            tab.title = doc.title.clone();
            return Ok(View::Reader(doc));
        }
        tab.title = nav.final_url.clone();
        Ok(View::Status {
            code: nav.status,
            body: nav.body.chars().take(8_000).collect(),
        })
    }

    fn freeze(&mut self, tab: &mut Tab) {
        tab.freeze();
    }

    fn thaw(&mut self, tab: &mut Tab) {
        tab.thaw();
    }

    fn memory_budget_bytes(&self) -> u64 {
        64 * 1024 * 1024
    }
}

pub fn read_html(nav: &NavigationResult) -> ReaderDocument {
    let dom = scraper::Html::parse_document(&nav.body);
    let title_sel = scraper::Selector::parse("title").unwrap();
    let title = dom
        .select(&title_sel)
        .next()
        .map(|t| t.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| nav.final_url.clone());
    let p_sel = scraper::Selector::parse("p, h1, h2, h3, li").unwrap();
    let mut text = String::new();
    for el in dom.select(&p_sel) {
        let t = el.text().collect::<String>();
        let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
        if !t.is_empty() {
            text.push_str(&t);
            text.push_str("\n\n");
        }
        if text.len() > 12_000 {
            break;
        }
    }
    let a_sel = scraper::Selector::parse("a[href]").unwrap();
    let mut links = Vec::new();
    for a in dom.select(&a_sel).take(40) {
        if let Some(href) = a.value().attr("href") {
            let label = a.text().collect::<String>();
            let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
            if !href.starts_with('#') {
                links.push((label, href.to_string()));
            }
        }
    }
    ReaderDocument { title, text, links }
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("bad url")]
    BadUrl,
    #[error("net")]
    Net,
}

/// JS-capable browsing. Off by default until libservo is pinned.
#[cfg(feature = "servo")]
compile_error!("Servo embedding is the product path; pin libservo before enabling this feature. Do not substitute WebView2.");

#[cfg(test)]
mod tests {
    use super::*;
    use vapurr_net::NavigationResult;

    #[test]
    fn reader_extracts_title_and_links() {
        let nav = NavigationResult {
            url: "https://example.com".into(),
            status: 200,
            content_type: "text/html".into(),
            body: r#"<html><head><title>Hello</title></head><body><h1>Hi</h1><p>Paper.</p><a href="/x">next</a></body></html>"#.into(),
            paywall: None,
            final_url: "https://example.com".into(),
        };
        let doc = read_html(&nav);
        assert_eq!(doc.title, "Hello");
        assert!(doc.text.contains("Paper"));
        assert_eq!(doc.links[0].1, "/x");
    }

    #[test]
    fn freeze_thaws() {
        let mut eng = FetcherEngine::new();
        let mut tab = Tab::new(1, "https://example.com").unwrap();
        eng.freeze(&mut tab);
        assert!(tab.frozen);
        eng.thaw(&mut tab);
        assert!(!tab.frozen);
    }
}
