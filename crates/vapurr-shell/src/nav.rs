use crate::desk::Desk;


pub(crate) fn vapurr_url(page: &str) -> String {
    format!("http://vapurr.localhost/{page}")
}


pub(crate) const FOMO_FAMILY: &str = "https://fomo.family";


pub(crate) fn needs_login(id: &str) -> bool {
    matches!(id, "wallet" | "portfolio")
}


pub(crate) fn pane_url(id: &str) -> String {
    if matches!(id, "fomo" | "family") {
        return FOMO_FAMILY.into();
    }
    if needs_login(id) && !vapurr_wallet::is_logged_in() {
        return vapurr_url(&format!("login.html?next={id}"));
    }
    let page = match id {
        "login" | "signin" | "signup" => "login.html",
        "wallet" | "portfolio" => "wallet.html?v=in",
        "pay" | "404" => "pay.html",
        "card" => "card.html",
        "zzzmail" | "zmail" | "mail" => "zzzmail.html",
        "id" => "id.html",
        "shield" | "adblock" => "shield.html",
        "swap" => "swap.html",
        "defi" | "finance" => "defi.html",
        "stake" => "pusd.html",
        "pusd" | "vapurr" | "mint" | "lithe" => "pusd.html",
        "bridge" => "bridge.html",
        "dapps" => "dapps.html",
        "scan" | "explorer" | "xray" | "blocks" => "explorer.html",
        "gas" | "gwei" => "explorer.html?tab=gas",
        "floor" | "list" | "projects" => "floor.html",
        "ketflix" => "ketflix.html",
        "ketcharts" | "charts" | "chart" => "ketcharts.html",
        "ketbook" | "docs" | "honkit" | "book" => "ketbook.html",
        "earn" => "earn.html",
        "history" => "history.html",
        "bookmarks" => "bookmarks.html",
        "cookies" | "cookie" | "jar" => "cookies.html",
        "settings" => "settings.html",
        "boost" | "memory" | "blobs" => "memory.html",
        "vapurrbid" | "outbid" | "bid" | "board" => "vapurrbid.html",
        "pns" | "hood" | "names" => "pns.html",
        _ => "pane.html",
    };
    if page == "pane.html" {
        vapurr_url(&format!("pane.html?id={id}"))
    } else {
        vapurr_url(page)
    }
}


pub(crate) fn resolve_nav(raw: &str) -> String {
    let u = raw.trim();
    if let Some(rest) = u.strip_prefix("vapurr://") {
        let rest = rest.trim_matches('/');
        let (id, qs) = rest.split_once('?').unwrap_or((rest, ""));
        let id = id.trim_matches('/');
        if id.is_empty() || id == "home" {
            return vapurr_url("home.html?v=wordmark");
        }
        if matches!(id, "scan" | "explorer" | "xray" | "blocks" | "gas" | "gwei") {
            if id == "gas" || id == "gwei" {
                return vapurr_url("explorer.html?tab=gas");
            }
            if qs.is_empty() {
                return vapurr_url("explorer.html");
            }
            return vapurr_url(&format!("explorer.html?{qs}"));
        }
        let mut url = pane_url(id);
        if !qs.is_empty() {
            if url.contains('?') {
                url.push('&');
            } else {
                url.push('?');
            }
            url.push_str(qs);
        }
        return url;
    }
    let hood = u.trim().trim_start_matches('@');
    if vapurr_zmail::looks_like_hood(hood) {
        return vapurr_url(&format!("explorer.html?q={hood}"));
    }
    if vapurr_rhc::scan::is_scan_query(u) {
        return vapurr_url(&format!("explorer.html?q={u}"));
    }
    u.to_string()
}


pub(crate) fn home_url(desk: &Desk) -> String {
    resolve_nav(&desk.prefs.homepage)
}

