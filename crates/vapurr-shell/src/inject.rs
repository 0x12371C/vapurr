use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use crate::desk::Desk;


pub(crate) fn js_set_zoom(pct: i64, show: bool, ctrl_scroll: bool) -> String {
    format!(
        "window.__setZoom && window.__setZoom({pct},{}); window.__vapurrCtrlScroll={};",
        if show { "true" } else { "false" },
        if ctrl_scroll { "true" } else { "false" }
    )
}


pub(crate) fn js_set_url(url: &str) -> String {
    format!(
        "window.__setUrl && window.__setUrl({})",
        serde_json::to_string(url).unwrap_or_else(|_| "\"\"".into())
    )
}


pub(crate) fn js_set_chain(json: &str) -> String {
    format!(
        "window.__setChain && window.__setChain({0});",
        json
    )
}


pub(crate) fn js_set_tabs(v: &serde_json::Value) -> String {
    format!(
        "window.__setTabs && window.__setTabs({})",
        v.to_string()
    )
}


pub(crate) fn js_set_blobs(v: &serde_json::Value) -> String {
    format!(
        "window.__setBlobs && window.__setBlobs({})",
        v.to_string()
    )
}


pub(crate) fn js_set_mail(v: &serde_json::Value) -> String {
    format!(
        "window.__setMail && window.__setMail({})",
        v
    )
}


pub(crate) fn js_on_boost(on: bool, vault: &serde_json::Value, hosts: &[String]) -> String {
    let mut v = serde_json::json!({
        "on": on,
        "warmed": hosts.len(),
        "hosts": hosts,
    });
    if let Some(dst) = v.as_object_mut() {
        if let Some(src) = vault.as_object() {
            if let Some(used) = src.get("used") {
                dst.insert("used".into(), used.clone());
            }
            if let Some(blobs) = src.get("blobs") {
                dst.insert("blobs".into(), blobs.clone());
            }
        }
    }
    format!("window.__onBoost && window.__onBoost({})", v)
}


pub(crate) fn is_warm_url(url: &str) -> bool {
    let u = url.to_ascii_lowercase();
    (u.starts_with("https://") || u.starts_with("http://"))
        && !u.contains("vapurr.localhost")
        && !u.starts_with("vapurr://")
        && !is_captcha_bait(&u)
}

/// Google/YouTube treat cookieless prefetch + DOM injection as a bot.
fn is_captcha_bait(url: &str) -> bool {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .trim_start_matches("www.");
    host == "youtu.be"
        || host == "youtube.com"
        || host.ends_with(".youtube.com")
        || host == "youtube-nocookie.com"
        || host.ends_with(".youtube-nocookie.com")
        || host == "gstatic.com"
        || host.ends_with(".gstatic.com")
        || host == "recaptcha.net"
        || host.ends_with(".recaptcha.net")
        || host == "ytimg.com"
        || host.ends_with(".ytimg.com")
        || host == "googlevideo.com"
        || host.ends_with(".googlevideo.com")
        || host == "googleusercontent.com"
        || host.ends_with(".googleusercontent.com")
        || host == "ggpht.com"
        || host.ends_with(".ggpht.com")
        || host == "google"
        || host.starts_with("google.")
        || host.ends_with(".google")
        || host.contains(".google.")
}


pub(crate) fn boost_targets(desk: &Desk) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |raw: &str| {
        if out.len() >= 8 || !is_warm_url(raw) {
            return;
        }
        if seen.insert(raw.to_string()) {
            out.push(raw.to_string());
        }
    };
    push("https://fomo.family");
    for h in desk.history.iter().rev() {
        push(&h.url);
    }
    for b in desk.bookmarks.iter().rev() {
        push(&b.url);
    }
    out
}


pub(crate) fn boost_hosts(urls: &[String]) -> Vec<String> {
    urls.iter()
        .filter_map(|u| {
            let rest = u
                .split("://")
                .nth(1)
                .unwrap_or(u)
                .split('/')
                .next()
                .unwrap_or("");
            if rest.is_empty() {
                None
            } else {
                Some(rest.trim_start_matches("www.").to_string())
            }
        })
        .collect()
}


pub(crate) fn js_boost_warm(urls: &[String]) -> String {
    if urls.is_empty() {
        return "window.__vapurrBoostClear && window.__vapurrBoostClear()".into();
    }
    let json = serde_json::to_string(urls).unwrap_or_else(|_| "[]".into());
    format!("window.__vapurrBoostWarm && window.__vapurrBoostWarm({json})")
}


pub(crate) fn snap_desk(desk: &Desk, vault: &std::cell::RefCell<vapurr_blob::Vault>) {
    if !desk.prefs.boost {
        return;
    }
    if let Ok(pt) = serde_json::to_vec(desk) {
        let _ = vault.borrow_mut().put_named("desk", "memory", &pt);
    }
}


pub(crate) fn js_set_boost(on: bool) -> String {
    format!(
        "window.__setBoost && window.__setBoost({})",
        if on { "true" } else { "false" }
    )
}


pub(crate) fn js_set_econ(v: &serde_json::Value) -> String {
    format!("window.__setEcon && window.__setEcon({})", v)
}


pub(crate) fn js_set_wallet(v: &serde_json::Value) -> String {
    format!("window.__setWallet && window.__setWallet({})", v)
}


pub(crate) fn js_set_outbid(v: &serde_json::Value) -> String {
    format!("window.__setOutbid && window.__setOutbid({})", v)
}


pub(crate) fn js_econ_err(which: &str, msg: &str) -> String {
    format!(
        "window.__econErr && window.__econErr({},{})",
        serde_json::to_string(which).unwrap_or_else(|_| "\"\"".into()),
        serde_json::to_string(msg).unwrap_or_else(|_| "\"\"".into())
    )
}


pub(crate) fn js_set_desk(v: &serde_json::Value) -> String {
    format!(
        "window.__setDesk && window.__setDesk({})",
        v.to_string()
    )
}


pub(crate) fn set_page_url(slot: &Arc<Mutex<String>>, url: &str) {
    if let Ok(mut g) = slot.lock() {
        *g = url.to_string();
    }
}


pub(crate) fn inject_shield(page: &wry::WebView, shield: &vapurr_shield::Shield, url: &str) {
    if crate::host::is_chrome_url(url) || vapurr_shield::is_pass_host(url) {
        return;
    }
    let js = shield.inject_js(url);
    if !js.is_empty() {
        let _ = page.evaluate_script(&js);
    }
}


pub(crate) fn js_apply_theme(theme: &str) -> String {
    let t = if theme.eq_ignore_ascii_case("light") {
        "light"
    } else {
        "dark"
    };
    format!(
        "(function(){{var t={};document.documentElement.setAttribute('data-theme',t);try{{localStorage.setItem('vapurr.theme',t);}}catch(e){{}}if(window.__applyTheme)window.__applyTheme(t);else if(window.__onTheme)window.__onTheme(t);}})();",
        serde_json::to_string(t).unwrap_or_else(|_| "\"dark\"".into())
    )
}


pub(crate) const BOOT_JS: &str = r#"
(function(){
  var chrome = false;
  try {
    var host = location.hostname || "";
    chrome = host === "vapurr.localhost" || String(location.protocol).indexOf("vapurr") === 0;
    if (chrome) {
      var t = localStorage.getItem("vapurr.theme");
      if (t !== "light") t = "dark";
      document.documentElement.setAttribute("data-theme", t);
    }
  } catch (e) {}
  // Lime cursor is chrome-only. Do not paint over fomo.family / WalletConnect modals.
  try {
    if (!chrome) return;
    if (document.getElementById("vapurr-cursor")) return;
    var s = document.createElement("style");
    s.id = "vapurr-cursor";
    s.textContent = "html,body{cursor:url(\"data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32'><path fill='%23c0f800' stroke='%230e0e0e' stroke-width='1.1' d='M3 2v21l7-6 4.6 10.4 3.2-1.4L13.2 17H22z'/></svg>\") 3 1, auto !important;}a,button,[role=button],select,label,summary{cursor:url(\"data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32'><path fill='%23c0f800' stroke='%230e0e0e' stroke-width='1.1' d='M3 2v21l7-6 4.6 10.4 3.2-1.4L13.2 17H22z'/></svg>\") 3 1, pointer !important;}input,textarea,[contenteditable=true]{cursor:url(\"data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='24' height='32'><path fill='none' stroke='%23c0f800' stroke-width='2' stroke-linecap='round' d='M6 4h12M12 4v24M6 28h12'/></svg>\") 12 16, text !important;caret-color:#c0f800 !important;}";
    (document.head || document.documentElement).appendChild(s);
  } catch (e) {}
})();
window.__vapurrWhenPaint = function (fn) {
  function go() {
    try {
      if (document.prerendering) {
        document.addEventListener("prerenderingchange", go, { once: true });
        return;
      }
      if (document.hidden) {
        document.addEventListener("visibilitychange", function v() {
          if (!document.hidden) {
            document.removeEventListener("visibilitychange", v);
            go();
          }
        });
        return;
      }
    } catch (e) {}
    fn();
  }
  go();
};
window.addEventListener("pageshow", function (e) {
  if (!e.persisted) return;
  try {
    if ((location.hostname || "") === "vapurr.localhost") {
      __vapurrPost({ cmd: "desk" });
    }
  } catch (err) {}
});
window.__vapurrReady = true;
function __vapurrPost(msg) {
  try {
    var p = JSON.stringify(msg);
    if (window.ipc && window.ipc.postMessage) window.ipc.postMessage(p);
    else if (window.chrome && window.chrome.webview) window.chrome.webview.postMessage(p);
  } catch (err) {}
}
document.addEventListener("keydown", function (e) {
  var c = e.ctrlKey || e.metaKey;
  if (e.altKey && e.key === "ArrowLeft") { e.preventDefault(); __vapurrPost({cmd:"back"}); return; }
  if (e.altKey && e.key === "ArrowRight") { e.preventDefault(); __vapurrPost({cmd:"forward"}); return; }
  if (!c) return;
  var k = e.key.toLowerCase();
  if (e.key === "Tab") {
    e.preventDefault();
    __vapurrPost({cmd: e.shiftKey ? "prevtab" : "nexttab"});
    return;
  }
  if (k >= "1" && k <= "8") { e.preventDefault(); __vapurrPost({cmd:"selecttabi", i: Number(k) - 1}); return; }
  if (k === "9") { e.preventDefault(); __vapurrPost({cmd:"selecttabi", i: 8}); return; }
  if (k === "t") { e.preventDefault(); __vapurrPost({cmd:"newtab"}); }
  else if (k === "w") { e.preventDefault(); __vapurrPost({cmd:"closetab"}); }
  else if (k === "l") { e.preventDefault(); __vapurrPost({cmd:"focusurl"}); }
  else if (k === "r") { e.preventDefault(); __vapurrPost({cmd:"reload"}); }
  else if (k === "f") { e.preventDefault(); __vapurrPost({cmd:"showfind"}); }
  else if (k === "d") { e.preventDefault(); __vapurrPost({cmd:"star"}); }
  else if (k === "h") { e.preventDefault(); __vapurrPost({cmd:"pane", id:"history"}); }
  else if (k === "," ) {
    e.preventDefault();
    var pane = "";
    try { pane = localStorage.getItem("vapurr.pane") || ""; } catch (err) {}
    __vapurrPost({cmd: pane === "settings" ? "home" : "settings"});
  }
  else if (k === "+" || k === "=" || e.key === "Add") { e.preventDefault(); __vapurrPost({cmd:"zoomin"}); }
  else if (k === "-" || e.key === "Subtract") { e.preventDefault(); __vapurrPost({cmd:"zoomout"}); }
  else if (k === "0") { e.preventDefault(); __vapurrPost({cmd:"zoomreset"}); }
}, true);
document.addEventListener("wheel", function (e) {
  if (!(e.ctrlKey || e.metaKey)) return;
  if (window.__vapurrCtrlScroll === false) return;
  e.preventDefault();
  __vapurrPost({cmd: e.deltaY < 0 ? "zoomin" : "zoomout"});
}, { capture: true, passive: false });
window.__vapurrBoostWarm = function (urls) {
  var old = document.getElementById("vapurr-boost-spec");
  if (old) old.remove();
  document.querySelectorAll("link[data-vapurr-boost]").forEach(function (n) { n.remove(); });
  if (!urls || !urls.length) return 0;
  var origins = [];
  urls.forEach(function (u) {
    try {
      var o = new URL(u).origin;
      if (origins.indexOf(o) === -1) origins.push(o);
    } catch (e) {}
  });
  origins.slice(0, 6).forEach(function (o) {
    ["dns-prefetch", "preconnect"].forEach(function (rel) {
      var l = document.createElement("link");
      l.rel = rel;
      l.href = o;
      if (rel === "preconnect") l.crossOrigin = "anonymous";
      l.setAttribute("data-vapurr-boost", "1");
      (document.head || document.documentElement).appendChild(l);
    });
  });
  var spec = document.createElement("script");
  spec.id = "vapurr-boost-spec";
  spec.type = "speculationrules";
  spec.textContent = JSON.stringify({
    prefetch: [{ source: "list", urls: urls.slice(0, 8), eagerness: "moderate" }],
    prerender: [{ source: "list", urls: urls.slice(0, 2), eagerness: "conservative" }]
  });
  (document.documentElement).appendChild(spec);
  return urls.length;
};
window.__vapurrBoostClear = function () {
  var old = document.getElementById("vapurr-boost-spec");
  if (old) old.remove();
  document.querySelectorAll("link[data-vapurr-boost]").forEach(function (n) { n.remove(); });
};
window.__boostTip = function (show, on) {
  var id = "vapurr-boost-tip";
  var el = document.getElementById(id);
  if (!show) {
    if (el) el.style.display = "none";
    return;
  }
  if (!el) {
    el = document.createElement("div");
    el.id = id;
    el.setAttribute("role", "tooltip");
    el.style.cssText = "position:fixed;top:12px;right:16px;z-index:2147483647;width:280px;padding:12px 14px;background:#1f2327;border:1px solid #c0f800;border-radius:12px;color:#f2f3f4;font:13px/1.45 Sora,ui-sans-serif,system-ui,sans-serif;box-shadow:0 10px 28px rgba(0,0,0,.5);pointer-events:none;";
    (document.body || document.documentElement).appendChild(el);
  }
  var state = on ? "ON" : "OFF";
  var body = on
    ? "Prefetch + prerender of recent sites. GPU stays hot. Next click is already in cache. Click Boost to turn off."
    : "Click Boost to prefetch and prerender recent sites. Off drops heat, bfcache still keeps Back instant.";
  el.innerHTML = "<div style='color:#c0f800;font-size:11px;letter-spacing:.12em;font-weight:600;margin-bottom:6px'>BOOST \\u00b7 " + state + "</div><div>" + body + "</div>";
  el.style.display = "block";
};
window.__boostTipSync = function (on) {
  var el = document.getElementById("vapurr-boost-tip");
  if (el && el.style.display !== "none") window.__boostTip(true, on);
};
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desk::Desk;

    #[test]
    fn boost_does_not_prefetch_google() {
        assert!(!is_warm_url("https://www.google.com/"));
        assert!(!is_warm_url("https://www.google.com/search?q=robinhood+chain"));
        assert!(!is_warm_url("https://accounts.google.com/signin"));
        assert!(!is_warm_url("https://www.youtube.com/watch?v=x"));
        assert!(!is_warm_url("https://www.gstatic.com/recaptcha/x.js"));
        assert!(is_warm_url("https://fomo.family/"));
        let d = Desk::default();
        for u in boost_targets(&d) {
            let low = u.to_ascii_lowercase();
            assert!(!low.contains("google"), "{u}");
            assert!(!low.contains("youtube"), "{u}");
        }
        assert!(
            !BOOT_JS.contains("credentials: \"omit\""),
            "cookieless fetch of warmed URLs is what Google flags as a bot"
        );
    }
}

