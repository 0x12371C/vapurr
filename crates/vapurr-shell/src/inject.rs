use crate::desk::Desk;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

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
    format!("window.__setChain && window.__setChain({0});", json)
}

pub(crate) fn js_set_tabs(v: &serde_json::Value) -> String {
    format!("window.__setTabs && window.__setTabs({})", v.to_string())
}

pub(crate) fn js_set_blobs(v: &serde_json::Value) -> String {
    format!("window.__setBlobs && window.__setBlobs({})", v.to_string())
}

pub(crate) fn js_set_mail(v: &serde_json::Value) -> String {
    format!("window.__setMail && window.__setMail({})", v)
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

pub(crate) fn wants_wallet_snap(url: &str) -> bool {
    url.contains("wallet.html") || url.contains("pay.html") || url.contains("swap.html") || url.contains("bridge.html")
}

pub(crate) fn is_faucet_host(url: &str) -> bool {
    let h = url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_ascii_lowercase()))
        .unwrap_or_default();
    h == "faucet.testnet.chain.robinhood.com"
        || h.ends_with(".faucet.testnet.chain.robinhood.com")
}

/// Robinhood's faucet is a Connect-wallet SPA. We are not a dapp browser —
/// this bridge exists only on that host so Connect / Add testnet / Import
/// see this device. Writes never leave the page; they reject.
pub(crate) fn js_faucet_attach(addr: &str) -> String {
    let addr = serde_json::to_string(addr).unwrap_or_else(|_| "\"\"".into());
    let rpc = serde_json::to_string(vapurr_rhc::TESTNET_RPC_HTTP)
        .unwrap_or_else(|_| "\"\"".into());
    format!(
        r#"(function(){{
  var addr = {addr};
  if (!addr || addr.length < 42) return 0;
  var chainId = "{chain}";
  var netVer = "{cid}";
  var rpc = {rpc};
  function fill() {{
    try {{
      document.querySelectorAll("input,textarea").forEach(function (el) {{
        var t = (el.type || "text").toLowerCase();
        if (t === "password" || t === "hidden" || t === "checkbox" || t === "radio" || t === "file") return;
        var ph = ((el.placeholder || "") + " " + (el.name || "") + " " + (el.id || "") + " " + (el.getAttribute("aria-label") || "")).toLowerCase();
        if (el.value && /^0x[0-9a-fA-F]{{40}}$/.test(el.value.trim())) return;
        if (ph.indexOf("address") < 0 && ph.indexOf("0x") < 0 && ph.indexOf("ens") < 0 && ph.indexOf("wallet") < 0) return;
        var proto = Object.getOwnPropertyDescriptor(window.HTMLInputElement && HTMLInputElement.prototype, "value")
          || Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement && HTMLTextAreaElement.prototype, "value");
        var prev = el.value;
        if (proto && proto.set) proto.set.call(el, addr);
        else el.value = addr;
        try {{ if (el._valueTracker) el._valueTracker.setValue(prev || ""); }} catch (e0) {{}}
        el.dispatchEvent(new Event("input", {{ bubbles: true }}));
        el.dispatchEvent(new Event("change", {{ bubbles: true }}));
      }});
    }} catch (e) {{}}
  }}
  function rpcCall(method, params) {{
    return fetch(rpc, {{
      method: "POST",
      headers: {{ "content-type": "application/json" }},
      body: JSON.stringify({{ jsonrpc: "2.0", id: 1, method: method, params: params || [] }})
    }}).then(function (r) {{ return r.json(); }}).then(function (j) {{
      if (j.error) {{ var e = new Error(j.error.message || "rpc"); e.code = j.error.code; throw e; }}
      return j.result;
    }});
  }}
  function sameChain(id) {{
    var s = String(id || "").toLowerCase();
    if (s === chainId || s === "0xb626") return true;
    var n = s.indexOf("0x") === 0 ? parseInt(s, 16) : parseInt(s, 10);
    return n === {cid};
  }}
  function emit(ev, val) {{
    var a = (eth && eth._on && eth._on[ev]) || [];
    for (var i = 0; i < a.length; i++) {{
      try {{ a[i](val); }} catch (e1) {{}}
    }}
  }}
  function live(args) {{
    var m = (args && args.method) || "";
    var p = (args && args.params) || [];
    if (m === "eth_requestAccounts") {{
      emit("connect", {{ chainId: chainId }});
      emit("accountsChanged", [addr]);
      emit("chainChanged", chainId);
      return Promise.resolve([addr]);
    }}
    if (m === "eth_accounts") return Promise.resolve([addr]);
    if (m === "eth_chainId") return Promise.resolve(chainId);
    if (m === "net_version") return Promise.resolve(netVer);
    if (m === "eth_coinbase") return Promise.resolve(addr);
    if (m === "wallet_getPermissions" || m === "wallet_requestPermissions")
      return Promise.resolve([{{ parentCapability: "eth_accounts" }}]);
    if (m === "wallet_switchEthereumChain") {{
      if (sameChain(p[0] && p[0].chainId)) {{
        emit("chainChanged", chainId);
        return Promise.resolve(null);
      }}
      var err = new Error("Unrecognized chain"); err.code = 4902; return Promise.reject(err);
    }}
    if (m === "wallet_addEthereumChain") {{
      if (!p[0] || sameChain(p[0].chainId)) {{
        emit("chainChanged", chainId);
        return Promise.resolve(null);
      }}
      var addErr = new Error("Unrecognized chain"); addErr.code = 4902; return Promise.reject(addErr);
    }}
    if (m === "wallet_watchAsset") return Promise.resolve(true);
    if (m === "eth_sendTransaction" || m === "eth_signTransaction" || m === "eth_sign" || m === "personal_sign" || m.indexOf("eth_signTypedData") === 0) {{
      var rej = new Error("Sign in vapurr Wallet, not the faucet page."); rej.code = 4001; return Promise.reject(rej);
    }}
    if (m.indexOf("eth_") === 0 || m === "net_listening" || m === "web3_clientVersion")
      return rpcCall(m, p);
    var u = new Error("unsupported " + m); u.code = 4200; return Promise.reject(u);
  }}
  function announce() {{
    try {{
      var info = Object.freeze({{
        uuid: "8f3c2a91-6b0e-4d17-9c4a-1a2b3c4d5e6f",
        name: "vapurr",
        icon: "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32'><rect width='32' height='32' rx='8' fill='%23C0F800'/></svg>",
        rdns: "app.vapurr"
      }});
      window.dispatchEvent(new CustomEvent("eip6963:announceProvider", {{
        detail: Object.freeze({{ info: info, provider: eth }})
      }}));
    }} catch (e2) {{}}
  }}
  var eth = window.ethereum;
  if (!eth || !eth.isVapurr) {{
    eth = {{
      isVapurr: true,
      isMetaMask: true,
      isConnected: function () {{ return true; }},
      enable: function () {{ return live({{ method: "eth_requestAccounts" }}); }},
      on: function (ev, fn) {{ (eth._on[ev] = eth._on[ev] || []).push(fn); }},
      removeListener: function (ev, fn) {{
        var a = eth._on[ev] || [];
        eth._on[ev] = a.filter(function (x) {{ return x !== fn; }});
      }},
      _on: {{}},
      _metamask: {{ isUnlocked: function () {{ return Promise.resolve(true); }} }}
    }};
    window.ethereum = eth;
  }}
  eth.isVapurr = true;
  eth.isMetaMask = true;
  eth.chainId = chainId;
  eth.networkVersion = netVer;
  eth.selectedAddress = addr;
  eth.providers = [eth];
  eth._metamask = eth._metamask || {{ isUnlocked: function () {{ return Promise.resolve(true); }} }};
  eth._live = live;
  eth.request = function (a) {{ return live(a); }};
  eth.send = function (payload, cb) {{
    if (typeof payload === "string") {{
      return live({{ method: payload, params: Array.isArray(cb) ? cb : [] }});
    }}
    var p = live(payload);
    if (typeof cb === "function") {{
      p.then(function (r) {{ cb(null, {{ jsonrpc: "2.0", id: payload && payload.id, result: r }}); }}, function (e) {{ cb(e); }});
      return;
    }}
    return p;
  }};
  eth.sendAsync = eth.send;
  eth.enable = function () {{ return live({{ method: "eth_requestAccounts" }}); }};
  if (!window.__vapurrFaucet6963) {{
    window.__vapurrFaucet6963 = true;
    try {{ window.addEventListener("eip6963:requestProvider", announce); }} catch (e3) {{}}
  }}
  if (typeof eth._flush === "function") eth._flush();
  fill();
  announce();
  try {{
    if (!window.__vapurrFaucetObs) {{
      window.__vapurrFaucetObs = new MutationObserver(fill);
      window.__vapurrFaucetObs.observe(document.documentElement, {{ childList: true, subtree: true }});
    }}
  }} catch (e) {{}}
  try {{ window.dispatchEvent(new Event("ethereum#initialized")); }} catch (e) {{}}
  emit("connect", {{ chainId: chainId }});
  emit("accountsChanged", [addr]);
  emit("chainChanged", chainId);
  return 1;
}})();"#,
        addr = addr,
        chain = vapurr_rhc::TESTNET_CHAIN_ID_HEX,
        cid = vapurr_rhc::TESTNET_CHAIN_ID,
        rpc = rpc
    )
}

pub(crate) fn js_set_outbid(v: &serde_json::Value) -> String {
    format!("window.__setOutbid && window.__setOutbid({})", v)
}

pub(crate) fn js_set_ketlist(v: &serde_json::Value) -> String {
    format!("window.__setKetList && window.__setKetList({})", v)
}

pub(crate) fn js_econ_err(which: &str, msg: &str) -> String {
    format!(
        "window.__econErr && window.__econErr({},{})",
        serde_json::to_string(which).unwrap_or_else(|_| "\"\"".into()),
        serde_json::to_string(msg).unwrap_or_else(|_| "\"\"".into())
    )
}

pub(crate) fn js_set_desk(v: &serde_json::Value) -> String {
    format!("window.__setDesk && window.__setDesk({})", v.to_string())
}

pub(crate) fn set_page_url(slot: &Arc<Mutex<String>>, url: &str) {
    if let Ok(mut g) = slot.lock() {
        *g = url.to_string();
    }
}

pub(crate) fn inject_faucet(page: &wry::WebView, url: &str) {
    if !is_faucet_host(url) {
        return;
    }
    let Some(addr) = vapurr_wallet::peek_address() else {
        return;
    };
    let _ = page.evaluate_script(&js_faucet_attach(&addr));
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
    if (host === "faucet.testnet.chain.robinhood.com" && !(window.ethereum && window.ethereum.isVapurr)) {
      var q = [];
      var eth = {
        isVapurr: true,
        isMetaMask: true,
        isConnected: function () { return true; },
        chainId: "0xb626",
        networkVersion: "46630",
        request: function (a) {
          if (eth._live) return eth._live(a);
          return new Promise(function (ok, no) { q.push([a, ok, no]); });
        },
        enable: function () { return eth.request({ method: "eth_requestAccounts" }); },
        send: function (payload, cb) {
          if (typeof payload === "string") {
            return eth.request({ method: payload, params: Array.isArray(cb) ? cb : [] });
          }
          var p = eth.request(payload);
          if (typeof cb === "function") {
            p.then(function (r) { cb(null, { jsonrpc: "2.0", id: payload && payload.id, result: r }); }, function (e) { cb(e); });
            return;
          }
          return p;
        },
        on: function (ev, fn) { (eth._on[ev] = eth._on[ev] || []).push(fn); },
        removeListener: function (ev, fn) {
          var a = eth._on[ev] || [];
          eth._on[ev] = a.filter(function (x) { return x !== fn; });
        },
        _on: {},
        _metamask: { isUnlocked: function () { return Promise.resolve(true); } },
        _flush: function () {
          var x;
          while ((x = q.shift())) eth.request(x[0]).then(x[1], x[2]);
        }
      };
      eth.sendAsync = eth.send;
      eth.providers = [eth];
      window.ethereum = eth;
      function announce() {
        try {
          var info = Object.freeze({
            uuid: "8f3c2a91-6b0e-4d17-9c4a-1a2b3c4d5e6f",
            name: "vapurr",
            icon: "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32'><rect width='32' height='32' rx='8' fill='%23C0F800'/></svg>",
            rdns: "app.vapurr"
          });
          window.dispatchEvent(new CustomEvent("eip6963:announceProvider", {
            detail: Object.freeze({ info: info, provider: eth })
          }));
        } catch (e3) {}
      }
      try { window.addEventListener("eip6963:requestProvider", announce); } catch (e4) {}
      announce();
      try { window.dispatchEvent(new Event("ethereum#initialized")); } catch (e2) {}
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
    fn ketpay_gets_wallet_snap() {
        assert!(wants_wallet_snap("http://vapurr.localhost/pay.html"));
        assert!(wants_wallet_snap("http://vapurr.localhost/wallet.html"));
        assert!(wants_wallet_snap("http://vapurr.localhost/swap.html"));
        assert!(wants_wallet_snap("http://vapurr.localhost/bridge.html"));
        assert!(!wants_wallet_snap("http://vapurr.localhost/home.html"));
    }

    #[test]
    fn faucet_bridge_is_host_only() {
        assert!(is_faucet_host("https://faucet.testnet.chain.robinhood.com/"));
        assert!(is_faucet_host(
            "https://faucet.testnet.chain.robinhood.com/add-chain"
        ));
        assert!(!is_faucet_host("https://fomo.family/"));
        assert!(!is_faucet_host("http://vapurr.localhost/wallet.html"));
        assert!(!is_faucet_host(
            "https://evil.test/?next=https://faucet.testnet.chain.robinhood.com/"
        ));
        let js = js_faucet_attach("0x1111111111111111111111111111111111111111");
        assert!(js.contains("isVapurr"));
        assert!(js.contains("0x1111111111111111111111111111111111111111"));
        assert!(js.contains(vapurr_rhc::TESTNET_CHAIN_ID_HEX));
        assert!(js.contains("0xb626"));
        assert!(!js.contains("0xb616"));
        assert!(js.contains("4001"));
        assert!(js.contains("eip6963:announceProvider"));
        assert!(BOOT_JS.contains("faucet.testnet.chain.robinhood.com"));
        assert!(BOOT_JS.contains("0xb626"));
        assert!(BOOT_JS.contains("eip6963:announceProvider"));
        assert_eq!(vapurr_rhc::TESTNET_CHAIN_ID_HEX, "0xb626");
    }

    #[test]
    fn boost_does_not_prefetch_google() {
        assert!(!is_warm_url("https://www.google.com/"));
        assert!(!is_warm_url(
            "https://www.google.com/search?q=robinhood+chain"
        ));
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
