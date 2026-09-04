//! Network stack lives in the shell. 402 is a paywall. 404 is a miss.

use vapurr_pay::{paywall_from, Paywall, HEADER_REQUIRED};

#[derive(Debug, Clone)]
pub struct NavigationResult {
    pub url: String,
    pub status: u16,
    pub content_type: String,
    pub body: String,
    pub paywall: Option<Paywall>,
    pub final_url: String,
}

pub struct Net {
    http: reqwest::blocking::Client,
}

impl Default for Net {
    fn default() -> Self {
        Self::new()
    }
}

impl Net {
    pub fn new() -> Self {
        let http = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(8))
            .timeout(std::time::Duration::from_secs(20))
            .user_agent("vapurr/0.1")
            .build()
            .expect("reqwest");
        Self { http }
    }

    pub fn navigate(&self, url: &str) -> Result<NavigationResult, NetError> {
        let parsed = url::Url::parse(url).map_err(|_| NetError::BadUrl)?;
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err(NetError::BadUrl);
        }
        let resp = self.http.get(parsed.clone()).send().map_err(|_| NetError::Transport)?;
        let status = resp.status().as_u16();
        let final_url = resp.url().to_string();
        let ctype = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let pay_header = resp
            .headers()
            .get(HEADER_REQUIRED)
            .or_else(|| resp.headers().get("payment-required"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = resp.text().unwrap_or_default();
        let paywall = paywall_from(status, pay_header.as_deref(), Some(&body), &final_url);
        Ok(NavigationResult {
            url: url.to_string(),
            status,
            content_type: ctype,
            body,
            paywall,
            final_url,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("bad url")]
    BadUrl,
    #[error("transport")]
    Transport,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_vapurr_scheme() {
        let n = Net::new();
        assert!(n.navigate("vapurr://wallet").is_err());
    }
}
