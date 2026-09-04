//! One content webview. Per-tab URL stacks. Chrome-standard UX, no extra processes.

#[derive(Clone)]
pub struct Tab {
    pub id: u64,
    pub title: String,
    stack: Vec<String>,
    idx: usize,
}

impl Tab {
    pub fn url(&self) -> &str {
        self.stack.get(self.idx).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn label(&self) -> String {
        if let Some(c) = chrome_label(self.url()) {
            return c.into();
        }
        if self.title.is_empty() {
            let u = self.url();
            if u.is_empty() {
                return "New Tab".into();
            }
            u.split("://")
                .nth(1)
                .unwrap_or(u)
                .split('/')
                .next()
                .unwrap_or("Tab")
                .to_string()
        } else {
            self.title.chars().take(32).collect()
        }
    }

    pub fn is_chrome(&self) -> bool {
        let u = self.url();
        u.contains("vapurr.localhost") || u.starts_with("vapurr://")
    }
}

pub struct TabStrip {
    pub tabs: Vec<Tab>,
    pub active: usize,
    next_id: u64,
    pub suppress: bool,
}

impl TabStrip {
    pub fn new(home: String) -> Self {
        Self {
            tabs: vec![Tab {
                id: 1,
                title: String::new(),
                stack: vec![home],
                idx: 0,
            }],
            active: 0,
            next_id: 2,
            suppress: false,
        }
    }

    pub fn current(&self) -> &Tab {
        &self.tabs[self.active]
    }

    pub fn current_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }

    pub fn navigate(&mut self, url: String) {
        let t = self.current_mut();
        if t.idx + 1 < t.stack.len() {
            t.stack.truncate(t.idx + 1);
        }
        if t.stack.get(t.idx) == Some(&url) {
            return;
        }
        t.stack.push(url);
        t.idx = t.stack.len() - 1;
        t.title.clear();
    }

    pub fn observe(&mut self, url: String) {
        if self.suppress {
            return;
        }
        let t = self.current_mut();
        if t.stack.get(t.idx) == Some(&url) {
            return;
        }
        if t.idx + 1 < t.stack.len() {
            t.stack.truncate(t.idx + 1);
        }
        t.stack.push(url);
        t.idx = t.stack.len() - 1;
    }

    pub fn back(&mut self) -> Option<String> {
        let t = self.current_mut();
        if t.idx == 0 {
            return None;
        }
        t.idx -= 1;
        t.title.clear();
        Some(t.url().to_string())
    }

    pub fn forward(&mut self) -> Option<String> {
        let t = self.current_mut();
        if t.idx + 1 >= t.stack.len() {
            return None;
        }
        t.idx += 1;
        t.title.clear();
        Some(t.url().to_string())
    }

    pub fn new_tab(&mut self, home: String) -> String {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab {
            id,
            title: String::new(),
            stack: vec![home.clone()],
            idx: 0,
        });
        self.active = self.tabs.len() - 1;
        home
    }

    /// Close `id`, or the active tab if `id` is None.
    /// Returns the URL to load when the visible tab changed.
    pub fn close(&mut self, id: Option<u64>, home: &str) -> Option<String> {
        let i = match id {
            Some(id) => self.tabs.iter().position(|t| t.id == id)?,
            None => self.active,
        };
        if self.tabs.len() == 1 {
            self.tabs[0].stack = vec![home.to_string()];
            self.tabs[0].idx = 0;
            self.tabs[0].title.clear();
            return Some(home.to_string());
        }
        let closing_active = i == self.active;
        self.tabs.remove(i);
        if closing_active {
            if self.active >= self.tabs.len() {
                self.active = self.tabs.len() - 1;
            }
            Some(self.current().url().to_string())
        } else {
            if i < self.active {
                self.active -= 1;
            }
            None
        }
    }

    pub fn select(&mut self, id: u64) -> Option<String> {
        let i = self.tabs.iter().position(|t| t.id == id)?;
        if i == self.active {
            return None;
        }
        self.active = i;
        Some(self.current().url().to_string())
    }

    pub fn select_at(&mut self, i: usize) -> Option<String> {
        let i = if i >= 8 {
            self.tabs.len().saturating_sub(1)
        } else {
            i
        };
        if i >= self.tabs.len() || i == self.active {
            return None;
        }
        self.active = i;
        Some(self.current().url().to_string())
    }

    pub fn cycle(&mut self, back: bool) -> Option<String> {
        let n = self.tabs.len();
        if n < 2 {
            return None;
        }
        self.active = if back {
            if self.active == 0 {
                n - 1
            } else {
                self.active - 1
            }
        } else {
            (self.active + 1) % n
        };
        Some(self.current().url().to_string())
    }

    pub fn set_title(&mut self, title: String) {
        let t = self.current_mut();
        if !t.is_chrome() {
            t.title = title;
        }
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "active": self.current().id,
            "tabs": self.tabs.iter().map(|t| serde_json::json!({
                "id": t.id,
                "title": t.label(),
                "url": t.url(),
                "chrome": t.is_chrome(),
            })).collect::<Vec<_>>(),
        })
    }
}

/// Short tab title for vapurr chrome and Live Trenches.
pub fn chrome_label(url: &str) -> Option<&'static str> {
    let u = url.to_ascii_lowercase();
    if u.contains("fomo.family") {
        return Some("Live Trenches");
    }
    if !(u.contains("vapurr.localhost") || u.starts_with("vapurr://")) {
        return None;
    }
    let file = u
        .split('/')
        .last()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("");
    Some(match file {
        "" | "home.html" => "Home",
        "wallet.html" => "Wallet",
        "pay.html" => "404",
        "card.html" => "Card",
        "zzzmail.html" | "zmail.html" => "zzzmail",
        "id.html" => "Identity",
        "shield.html" => "Shield",
        "swap.html" => "Swap",
        "defi.html" => "DeFi",
        "pusd.html" => "PUSD",
        "pns.html" => "PNS",
        "login.html" => "Sign in",
        "bridge.html" => "Bridge",
        "dapps.html" => "dApps",
        "explorer.html" => "Scan",
        "floor.html" => "Floor",
        "ketflix.html" => "Ketflix",
        "ketcharts.html" => "Ketcharts",
        "earn.html" => "Earn",
        "history.html" => "History",
        "bookmarks.html" => "Bookmarks",
        "cookies.html" => "Cookies",
        "settings.html" => "Settings",
        "memory.html" => "Boost",
        "vapurrbid.html" | "outbid.html" => "vapurrbid",
        "radio.html" => "Radio",
        "pane.html" => "vapurr",
        _ => "vapurr",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_pages_are_named() {
        assert_eq!(
            chrome_label("http://vapurr.localhost/zzzmail.html"),
            Some("zzzmail")
        );
        assert_eq!(
            chrome_label("http://vapurr.localhost/explorer.html?tab=gas"),
            Some("Scan")
        );
        assert_eq!(
            chrome_label("http://vapurr.localhost/ketcharts.html"),
            Some("Ketcharts")
        );
        assert_eq!(
            chrome_label("https://fomo.family/foo"),
            Some("Live Trenches")
        );
        assert_eq!(chrome_label("https://google.com"), None);
    }

    #[test]
    fn new_home_tab_says_home_not_new_tab() {
        let s = TabStrip::new("http://vapurr.localhost/home.html?v=wordmark".into());
        assert_eq!(s.current().label(), "Home");
    }

    #[test]
    fn closing_background_tab_keeps_the_page() {
        let mut s = TabStrip::new("http://vapurr.localhost/home.html".into());
        s.new_tab("http://vapurr.localhost/wallet.html".into());
        s.new_tab("http://vapurr.localhost/zzzmail.html".into());
        s.select(1);
        assert_eq!(s.current().label(), "Home");
        let nav = s.close(Some(2), "http://vapurr.localhost/home.html");
        assert!(nav.is_none());
        assert_eq!(s.current().id, 1);
        assert_eq!(s.tabs.len(), 2);
        assert_eq!(s.tabs[1].label(), "zzzmail");
    }

    #[test]
    fn cycle_wraps() {
        let mut s = TabStrip::new("http://a".into());
        s.new_tab("http://b".into());
        s.cycle(false);
        assert_eq!(s.current().url(), "http://a");
        s.cycle(true);
        assert_eq!(s.current().url(), "http://b");
    }
}
