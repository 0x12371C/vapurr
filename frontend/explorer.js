(function () {
  var view = document.getElementById("view");
  var input = document.getElementById("q");
  var live = document.getElementById("live");
  var pip = document.getElementById("livepip");
  var poll = 0;
  var liveTick = 0;
  var state = { page: "head" };
  var ignoreHash = false;
  var clock = 0;
  var sugIx = -1;
  var WARM = "History is catching up. Live head is on Overview.";

  function api(path) {
    return fetch("/scan/api/" + path).then(function (r) {
      return r.text().then(function (t) {
        try { return JSON.parse(t); }
        catch (e) { return { ok: false, error: t ? "bad json" : "empty" }; }
      });
    });
  }
  function scanApi(verb, id, extras) {
    var p = verb;
    if (id != null && id !== "") p += "/" + encodeURIComponent(String(id));
    if (extras) {
      Object.keys(extras).forEach(function (k) {
        var v = extras[k];
        if (v == null || v === "") return;
        if (typeof v !== "string") v = cursorParam(v);
        if (!v) return;
        p += "/" + encodeURIComponent(k) + "/" + encodeURIComponent(v);
      });
    }
    return p;
  }
  function isField(el) {
    if (!el || el === document.body || el === document.documentElement) return false;
    var tag = (el.tagName || "").toLowerCase();
    if (tag === "input" || tag === "textarea" || tag === "select") return true;
    if (el.isContentEditable) return true;
    return false;
  }
  function normHash(h) {
    h = String(h || "").trim().toLowerCase();
    if (h && h.indexOf("0x") !== 0) h = "0x" + h;
    return h;
  }
  function comma(n) {
    var s = String(n == null ? "" : n);
    if (!/^\d+$/.test(s)) return s;
    return s.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  }
  function tokenHolderCensus(d) {
    if (!d || Array.isArray(d.holders) || d.holders == null || d.holders === "") return null;
    var n = Number(d.holders);
    if (!isFinite(n) || n < 0) return null;
    if (d.source === "rhc-liq" && d.degree != null && Number(d.degree) === n) return null;
    return n;
  }
  function short(h, n) {
    h = String(h || "");
    n = n || 6;
    if (h.length <= n * 2 + 2) return h;
    return h.slice(0, n + 2) + "…" + h.slice(-n);
  }
  function ago(ts, now) {
    ts = Number(ts || 0);
    now = Number(now || Math.floor(Date.now() / 1000));
    if (!ts) return "—";
    var d = Math.max(0, now - ts);
    if (d < 60) return d + "s ago";
    if (d < 3600) return Math.floor(d / 60) + "m ago";
    if (d < 86400) return Math.floor(d / 3600) + "h ago";
    return Math.floor(d / 86400) + "d ago";
  }
  function ageEl(ts) {
    var s = document.createElement("span");
    s.setAttribute("data-ts", String(ts || 0));
    s.textContent = ago(ts);
    if (ts) {
      try { s.title = new Date(Number(ts) * 1000).toISOString(); } catch (e) {}
    }
    return s;
  }
  function tickAges() {
    var now = Math.floor(Date.now() / 1000);
    document.querySelectorAll("[data-ts]").forEach(function (el) {
      el.textContent = ago(Number(el.getAttribute("data-ts")), now);
    });
    var live = document.getElementById("liq-live");
    if (live && liqData && liqData.ts) live.textContent = liqAge(liqData.ts);
  }
  function setHash(h) {
    if (location.hash === h) return;
    ignoreHash = true;
    location.hash = h;
  }
  function dropQ() {
    if (!location.search) return;
    try {
      history.replaceState(null, "", location.pathname + (location.hash || ""));
    } catch (e) {}
  }
  function toast(msg) {
    var t = document.getElementById("toast");
    if (!t) return;
    t.textContent = msg;
    t.classList.add("on");
    clearTimeout(toast._t);
    toast._t = setTimeout(function () { t.classList.remove("on"); }, 1100);
  }
  function execCopy(t) {
    try {
      var ta = document.createElement("textarea");
      ta.value = t;
      ta.setAttribute("readonly", "");
      ta.style.position = "fixed";
      ta.style.left = "-9999px";
      document.body.appendChild(ta);
      ta.select();
      var ok = document.execCommand("copy");
      document.body.removeChild(ta);
      return !!ok;
    } catch (e) {
      return false;
    }
  }
  function copy(t) {
    t = String(t || "");
    if (!t) return;
    function ok() { toast("copied"); }
    function fail() { toast("copy failed"); }
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(t).then(ok).catch(function () {
        if (execCopy(t)) ok();
        else fail();
      });
      return;
    }
    if (execCopy(t)) ok();
    else fail();
  }
  function hashWrap(h, kind, preview, full) {
    h = String(h || "");
    var wrap = document.createElement("span");
    wrap.className = "hashwrap";
    var b = document.createElement("button");
    b.type = "button";
    b.className = "hash" + (full ? " full" : "");
    b.textContent = full ? h : short(h);
    b.title = h;
    b.onclick = function (e) {
      e.stopPropagation();
      if (kind === "tx") goTx(h, preview);
      else if (kind === "block") goBlock(h);
      else if (kind === "addr") goAddr(h);
      else if (kind === "token") goToken(h);
    };
    var c = document.createElement("button");
    c.type = "button";
    c.className = "copy";
    c.textContent = "copy";
    c.title = "Copy " + h;
    c.onclick = function (e) {
      e.stopPropagation();
      copy(h);
    };
    c.addEventListener("keydown", function (e) {
      e.stopPropagation();
    });
    wrap.appendChild(b);
    wrap.appendChild(c);
    return wrap;
  }
  function armRow(tr, fn) {
    tr.tabIndex = 0;
    tr.setAttribute("role", "link");
    tr.addEventListener("click", fn);
    tr.addEventListener("keydown", function (e) {
      if (e.key !== "Enter" && e.key !== " ") return;
      if (e.target && e.target.closest && e.target.closest("button")) return;
      e.preventDefault();
      fn(e);
    });
  }
  function pinUsdg(rows) {
    rows = (rows || []).slice();
    rows.sort(function (a, b) {
      return (b.usdg ? 1 : 0) - (a.usdg ? 1 : 0);
    });
    return rows;
  }
  function tokenWarm(rows, d) {
    var source = d && d.source;
    var err = d && d.error;
    if (source === "rpc" || err === "index wait" || (d && d.index === false)) {
      if (!rows.length) return true;
      if (rows.length === 1 && rows[0].usdg) return true;
    }
    return false;
  }
  function pushStack(stack, cursor) {
    return (stack || []).concat([cursor == null ? "" : cursor]);
  }
  function popStack(stack) {
    stack = (stack || []).slice();
    var prev = stack.pop();
    if (prev === "") prev = null;
    return { cursor: prev == null ? null : prev, stack: stack };
  }
  function ident(addr) {
    var el = document.createElement("span");
    el.className = "ident";
    var hex = String(addr || "").replace(/^0x/i, "") + "0000000000000000000000000";
    for (var i = 0; i < 25; i++) {
      var cell = document.createElement("i");
      if (parseInt(hex.charAt(i % hex.length), 16) > 7) cell.className = "on";
      el.appendChild(cell);
    }
    return el;
  }
  var KNOWN = {
    "0x5fc5360d0400a0fd4f2af552add042d716f1d168": "USDG",
    "0x0bd7d308f8e1639fab988df18a8011f41eacad73": "WETH",
    "0x5d3a1ff2b6bab83b63cd9ad0787074081a52ef34": "USDe",
    "0x1f7d7550b1b028f7571e69a784071f0205fd2efa": "Uniswap V3 Factory",
    "0x8bceaa40b9acdfaedf85adf4ff01f5ad6517937f": "Uniswap V2 Factory",
    "0xe51960f1b45f1c9fb6d166e6a884f866fc70433b": "Sushi V3 Factory",
    "0x8366a39cc670b4001a1121b8f6a443a643e40951": "Uniswap v4 PoolManager",
    "0x58daec3116aae6d93017baaea7749052e8a04fa7": "Uniswap v4 PositionManager",
    "0x000000000022d473030f116ddee9f6b43ac78ba3": "Permit2",
    "0x0000000071727de22e5e9d8baf0edac6f37da032": "EntryPoint v0.7",
    "0x4337084d9e255ff0702461cf8895ce9e3b5ff108": "EntryPoint v0.8",
    "0x0000000000000000000000000000000000000000": "native"
  };
  var METHODS = {
    "0xa9059cbb": "transfer",
    "0x23b872dd": "transferFrom",
    "0x095ea7b3": "approve",
    "0x70a08231": "balanceOf",
    "0x18160ddd": "totalSupply",
    "0x313ce567": "decimals",
    "0x06fdde03": "name",
    "0x95d89b41": "symbol",
    "0x2e1a7d4d": "withdraw",
    "0xd0e30db0": "deposit",
    "0x3593564c": "execute",
    "0xb61d27f6": "execute",
    "0x24856bc3": "execute",
    "0x40c10f19": "mint",
    "0x6a627842": "mint",
    "0xa0712d68": "mint",
    "0x1249c58b": "mint",
    "0x42966c68": "burn",
    "0xa22cb465": "setApprovalForAll",
    "0x42842e0e": "safeTransferFrom",
    "0xb88d4fde": "safeTransferFrom",
    "0xf242432a": "safeTransferFrom",
    "0x2eb2c2d6": "safeBatchTransferFrom",
    "0x38ed1739": "swapExactTokensForTokens",
    "0x7ff36ab5": "swapExactETHForTokens",
    "0x18cbafe5": "swapExactTokensForETH",
    "0x5c11d795": "swapExactTokensForTokensSupportingFeeOnTransferTokens",
    "0xfb3bdb41": "swapETHForExactTokens",
    "0x4a25d94a": "swapTokensForExactETH",
    "0x8803dbee": "swapTokensForExactTokens",
    "0x022c0d9f": "swap",
    "0x128acb08": "swap",
    "0x414bf389": "exactInputSingle",
    "0x04e45aaf": "exactInputSingle",
    "0xc04b8d59": "exactInput",
    "0xdb3e2198": "exactOutputSingle",
    "0xf28c0498": "exactOutput",
    "0xac9650d8": "multicall",
    "0x5ae401dc": "multicall",
    "0x12aa3caf": "swap",
    "0x7c025200": "swap",
    "0x5f575529": "swap",
    "0xd9627aa4": "sellToUniswap",
    "0x83bd37f9": "unxswapTo",
    "0x0502b1c5": "unoswap",
    "0x4e71d92d": "claim",
    "0x379607f5": "claim",
    "0x3d18b912": "getReward"
  };
  function knownLabel(a) {
    a = String(a || "").toLowerCase();
    return KNOWN[a] || "";
  }
  function methodLabel(m) {
    m = String(m || "").trim();
    if (!m) return "call";
    var key = m.toLowerCase();
    if (METHODS[key]) return METHODS[key];
    if (/^0x[0-9a-f]{8}$/i.test(m) && METHODS[key]) return METHODS[key];
    return m;
  }
  function msLabel(n) {
    n = Number(n || 0);
    if (!n) return "—";
    if (n >= 1000) return (n / 1000).toFixed(2) + "s";
    return Math.round(n) + " ms";
  }
  function hexUtf8(hex) {
    hex = String(hex || "").replace(/^0x/i, "");
    if (!hex || hex.length % 2) return "";
    var out = "";
    for (var i = 0; i < hex.length; i += 2) {
      var c = parseInt(hex.slice(i, i + 2), 16);
      if (!isFinite(c)) return "";
      if (c === 0) continue;
      if (c < 32 || c > 126) out += ".";
      else out += String.fromCharCode(c);
    }
    return out.replace(/\.+$/, "").trim();
  }
  function isNftXfer(x) {
    if (!x) return false;
    if (x.nft) return true;
    var k = String(x.kind || "").toUpperCase();
    if (k.indexOf("721") >= 0 || k.indexOf("1155") >= 0) return true;
    return String(x.amount || "").charAt(0) === "#";
  }
  var pnsRev = {};
  function pnsName(a) {
    a = String(a || "").toLowerCase();
    return pnsRev[a] || "";
  }
  function loadPns() {
    fetch("/zzzmail/api/hood").then(function (r) { return r.json(); }).then(function (d) {
      var rev = (d && (d.pns_reg || d.pns || d.hood) || {}).reverse || {};
      Object.keys(rev).forEach(function (k) { pnsRev[String(k).toLowerCase()] = rev[k]; });
      if (d && d.hood && d.hood.primary && d.me && d.me.address) {
        pnsRev[String(d.me.address).toLowerCase()] = d.hood.primary;
      }
    }).catch(function () {});
  }
  function addrChip(a, kind, compact, name) {
    var box = document.createElement("span");
    box.className = "addrchip";
    if (a && !compact) box.appendChild(ident(a));
    var lab = name || pnsName(a) || knownLabel(a);
    if (lab) {
      var tag = document.createElement("span");
      tag.className = "tag";
      tag.textContent = lab;
      box.appendChild(tag);
    }
    box.appendChild(hashWrap(a || "—", kind || "addr"));
    return box;
  }
  function pillEl(text, cls) {
    var p = document.createElement("span");
    p.className = "pill" + (cls ? " " + cls : "");
    p.textContent = text;
    return p;
  }
  function when(ts) {
    ts = Number(ts || 0);
    if (!ts) return "—";
    try {
      return new Date(ts * 1000).toISOString().replace("T", " ").replace(/\.\d+Z$/, " UTC");
    } catch (e) {
      return ago(ts);
    }
  }
  function typeName(t) {
    if (t === 2) return "EIP-1559";
    if (t === 1) return "EIP-2930";
    if (t === 3) return "EIP-4844";
    if (t === 0) return "legacy";
    return t == null || t === "" ? "—" : String(t);
  }
  function gweiLabel(n) {
    n = Number(n);
    if (!isFinite(n) || n <= 0) return "—";
    return n < 10 ? n.toFixed(2) + " GWEI" : n.toFixed(1) + " GWEI";
  }
  function meterEl(frac, heat, caption) {
    frac = Math.max(0, Math.min(1, Number(frac) || 0));
    var wrap = document.createElement("div");
    wrap.className = "meter-wrap";
    var svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("class", "meter");
    svg.setAttribute("viewBox", "0 0 156 156");
    var cx = 78, cy = 84, r = 58;
    var start = Math.PI * 0.75;
    var sweep = Math.PI * 1.5;
    function pt(a) { return [cx + r * Math.cos(a), cy + r * Math.sin(a)]; }
    function arc(t) {
      var a1 = start + sweep * t;
      var p0 = pt(start), p1 = pt(a1);
      var large = (sweep * t) > Math.PI ? 1 : 0;
      return "M " + p0[0].toFixed(2) + " " + p0[1].toFixed(2) + " A " + r + " " + r + " 0 " + large + " 1 " + p1[0].toFixed(2) + " " + p1[1].toFixed(2);
    }
    function path(d, stroke, width) {
      var p = document.createElementNS("http://www.w3.org/2000/svg", "path");
      p.setAttribute("d", d);
      p.setAttribute("fill", "none");
      p.setAttribute("stroke", stroke);
      p.setAttribute("stroke-width", String(width));
      p.setAttribute("stroke-linecap", "round");
      return p;
    }
    svg.appendChild(path(arc(1), "#2a3800", 10));
    if (frac > 0.002) svg.appendChild(path(arc(frac), "#c0f800", 10));
    var lab = document.createElementNS("http://www.w3.org/2000/svg", "text");
    lab.setAttribute("x", "78");
    lab.setAttribute("y", "82");
    lab.setAttribute("text-anchor", "middle");
    lab.setAttribute("fill", "#f2f3f4");
    lab.setAttribute("font-size", "22");
    lab.setAttribute("font-family", "Sora");
    lab.setAttribute("font-weight", "600");
    lab.textContent = Math.round(frac * 100) + "%";
    svg.appendChild(lab);
    var sub = document.createElementNS("http://www.w3.org/2000/svg", "text");
    sub.setAttribute("x", "78");
    sub.setAttribute("y", "102");
    sub.setAttribute("text-anchor", "middle");
    sub.setAttribute("fill", "#8aa090");
    sub.setAttribute("font-size", "11");
    sub.setAttribute("font-family", "Sora");
    sub.textContent = heat || "gas";
    svg.appendChild(sub);
    wrap.appendChild(svg);
    var right = document.createElement("div");
    right.appendChild(caption);
    wrap.appendChild(right);
    return wrap;
  }
  function flowEl(from, to, value, contract) {
    var f = document.createElement("div");
    f.className = "flow";
    var a = document.createElement("div");
    a.className = "flow-end";
    var la = document.createElement("div");
    la.className = "mono";
    la.textContent = "From";
    a.appendChild(la);
    a.appendChild(addrChip(from, "addr"));
    var mid = document.createElement("div");
    mid.className = "flow-mid";
    var mv = document.createElement("div");
    mv.className = "mono";
    mv.textContent = value || "";
    mid.appendChild(mv);
    mid.appendChild(document.createTextNode("→"));
    var b = document.createElement("div");
    b.className = "flow-end";
    var lb = document.createElement("div");
    lb.className = "mono";
    lb.textContent = contract && !to ? "Contract" : "To";
    b.appendChild(lb);
    b.appendChild(addrChip(to || contract || "", "addr"));
    f.appendChild(a);
    f.appendChild(mid);
    f.appendChild(b);
    return f;
  }
  function sheet(title, node) {
    return detailsPanel(title, node);
  }
  function detailsPanel(title, node) {
    var col = document.createElement("div");
    col.className = "details";
    if (title) {
      var h = document.createElement("h2");
      h.textContent = title;
      col.appendChild(h);
    }
    col.appendChild(node);
    return col;
  }
  function statGrid(rows) {
    var stats = document.createElement("div");
    stats.className = "stats";
    rows.forEach(function (s) {
      var n = document.createElement("div");
      n.className = "stat";
      n.setAttribute("data-k", s[0]);
      n.innerHTML = "<div class='mono'></div><b></b>";
      n.querySelector(".mono").textContent = s[0];
      n.querySelector("b").textContent = s[1];
      if (typeof s[2] === "function") {
        n.classList.add("go");
        n.onclick = s[2];
        n.title = s[3] || "Open";
      }
      stats.appendChild(n);
    });
    return stats;
  }
  function patchStatGrid(rows) {
    var box = view.querySelector(".stats");
    if (!box || box.children.length !== rows.length) {
      var neu = statGrid(rows);
      if (box) box.replaceWith(neu);
      else view.insertBefore(neu, view.firstChild);
      return;
    }
    rows.forEach(function (s, i) {
      var n = box.children[i];
      n.setAttribute("data-k", s[0]);
      var m = n.querySelector(".mono");
      var b = n.querySelector("b");
      if (m) m.textContent = s[0];
      if (b) {
        var next = String(s[1]);
        if (b.textContent !== next) {
          b.textContent = next;
          n.classList.remove("flash");
          void n.offsetWidth;
          n.classList.add("flash");
        }
      }
      if (typeof s[2] === "function") {
        n.classList.add("go");
        n.onclick = s[2];
        n.title = s[3] || "Open";
      }
    });
  }
  function patchSpark(values) {
    var sp = view.querySelector(".spark");
    if (!sp) return;
    var vals = (values || []).slice().reverse();
    var bars = sp.querySelectorAll("i");
    if (bars.length !== vals.length) {
      sp.replaceWith(sparkEl(values));
      return;
    }
    vals.forEach(function (v, i) {
      bars[i].style.height = Math.max(8, Math.round(Number(v) * 44)) + "px";
      bars[i].classList.toggle("on", v > 0.55);
    });
  }
  function gasCaption(gas) {
    var lanes = document.createElement("div");
    lanes.className = "lanes";
    [["slow", gas.slow], ["avg", gas.avg], ["fast", gas.fast]].forEach(function (lane, i) {
      var box = document.createElement("div");
      box.className = "lane" + (i === 1 ? " on" : "");
      var lab = document.createElement("div");
      lab.className = "mono";
      lab.textContent = lane[0];
      var b = document.createElement("b");
      b.textContent = gweiLabel(lane[1]);
      box.appendChild(lab);
      box.appendChild(b);
      lanes.appendChild(box);
    });
    var cap = document.createElement("div");
    cap.appendChild(lanes);
    var note = document.createElement("div");
    note.className = "meta";
    note.style.marginTop = "10px";
    note.textContent = "Relative to recent blocks  ·  " + (gas.heat || "cool") + "  ·  Robinhood Chain 4663";
    cap.appendChild(note);
    return cap;
  }
  function rowKey(rows, field) {
    return (rows || []).map(function (r) { return String(r && (r[field] || r.hash || r.tx || r.number || "") ); }).join(",");
  }
  function sparkEl(values) {
    var sp = document.createElement("div");
    sp.className = "spark";
    sp.title = "Gas used, last blocks";
    (values || []).slice().reverse().forEach(function (v) {
      var i = document.createElement("i");
      i.style.height = Math.max(8, Math.round(Number(v) * 44)) + "px";
      if (v > 0.55) i.classList.add("on");
      sp.appendChild(i);
    });
    return sp;
  }
  function crumb(parts) {
    var c = document.createElement("div");
    c.className = "crumb";
    var home = document.createElement("button");
    home.className = "btn-ghost";
    home.type = "button";
    home.textContent = "Overview";
    home.onclick = function () { goHead(); };
    c.appendChild(home);
    (parts || []).forEach(function (p) {
      var s = document.createElement("span");
      s.className = "meta";
      s.textContent = "/  " + p;
      c.appendChild(s);
    });
    var share = document.createElement("button");
    share.className = "copy";
    share.type = "button";
    share.textContent = "copy link";
    share.title = "Copy this page URL";
    share.onclick = function (e) {
      e.stopPropagation();
      copy(location.href);
    };
    c.appendChild(share);
    return c;
  }
  function kv(rows, wide) {
    var box = document.createElement("div");
    box.className = wide ? "kv wide" : "kv";
    rows.forEach(function (r) {
      if (r[2] === "section") {
        var sec = document.createElement("div");
        sec.className = "subhead";
        sec.textContent = r[0];
        box.appendChild(sec);
        return;
      }
      var val = r[1];
      var empty = val == null || val === "";
      var k = document.createElement("span");
      k.textContent = r[0];
      var v = document.createElement("b");
      if (r[2] === "node" && val && val.nodeType) {
        v.appendChild(val);
      } else if (empty) {
        v.textContent = "—";
        v.className = "meta";
      } else if (r[2] === "tx" || r[2] === "block" || r[2] === "addr" || r[2] === "token") {
        if (r[2] === "addr") v.appendChild(addrChip(val, "addr", false, r[5]));
        else v.appendChild(hashWrap(val, r[2], null, !!r[4]));
      } else if (r[3] === "age") {
        var hold = document.createElement("span");
        hold.appendChild(ageEl(val));
        hold.appendChild(document.createTextNode("  ·  " + when(val)));
        v.appendChild(hold);
      } else {
        v.textContent = String(val);
      }
      box.appendChild(k);
      box.appendChild(v);
    });
    return box;
  }
  function cursorParam(c) {
    if (c == null || c === "" || c === false) return "";
    return typeof c === "string" ? c : JSON.stringify(c);
  }
  function morePager(next, onMore, hasPrev, onPrev) {
    if (!next && !hasPrev) return null;
    var pager = document.createElement("div");
    pager.className = "pager";
    if (hasPrev && onPrev) {
      var p = document.createElement("button");
      p.className = "btn-ghost";
      p.type = "button";
      p.textContent = "Prev";
      p.setAttribute("data-prev", "1");
      p.onclick = onPrev;
      pager.appendChild(p);
    }
    if (next) {
      var b = document.createElement("button");
      b.className = "btn-ghost";
      b.type = "button";
      b.textContent = "More";
      b.setAttribute("data-more", "1");
      b.onclick = onMore;
      pager.appendChild(b);
    }
    return pager;
  }
  function setLive(ok, text, ts) {
    if (pip) pip.classList.toggle("on", !!ok);
    if (!live) return;
    live.textContent = "";
    var s = document.createElement("span");
    s.textContent = text || (ok ? "live" : "waiting");
    live.appendChild(s);
    if (ts) {
      live.appendChild(document.createTextNode("  ·  "));
      live.appendChild(ageEl(ts));
    }
  }
  function clearBusy() {
    if (view) view.removeAttribute("aria-busy");
  }
  function setTab(id) {
    var tabs = document.getElementById("tabs");
    if (!tabs) return;
    var buttons = tabs.querySelectorAll("button[data-tab]");
    var any = false;
    Array.prototype.forEach.call(buttons, function (b) {
      var on = id && b.getAttribute("data-tab") === id;
      b.classList.toggle("on", on);
      b.tabIndex = on ? 0 : -1;
      b.setAttribute("aria-selected", on ? "true" : "false");
      if (on) any = true;
    });
    if (!any && buttons[0]) buttons[0].tabIndex = 0;
  }
  function err(msg) {
    clearBusy();
    if (state.page === "head") setLive(false, "waiting");
    view.innerHTML = "";
    view.removeAttribute("data-scan");
    var p = document.createElement("div");
    p.className = "empty";
    if (msg === "index wait" || msg === "index offline") msg = WARM;
    if (msg === "rpc wait" || (msg && msg.indexOf("rpc wait") === 0)) msg = "RPC quiet. Retry stays in vapurr.";
    p.textContent = msg || "RPC quiet. Retry.";
    var b = document.createElement("button");
    b.className = "btn-ghost";
    b.type = "button";
    b.textContent = "Retry";
    b.onclick = route;
    view.appendChild(p);
    view.appendChild(b);
  }
  function skel(label) {
    view.innerHTML = "";
    view.removeAttribute("data-scan");
    view.setAttribute("aria-busy", "true");
    if (label) {
      var cap = document.createElement("div");
      cap.className = "meta";
      cap.textContent = label;
      view.appendChild(cap);
    }
    var stats = document.createElement("div");
    stats.className = "stats";
    for (var i = 0; i < 6; i++) {
      var n = document.createElement("div");
      n.className = "stat";
      n.innerHTML = "<div class='skel' style='width:42%'></div><div class='skel'></div>";
      stats.appendChild(n);
    }
    view.appendChild(stats);
    var cols = document.createElement("div");
    cols.className = "cols";
    for (var j = 0; j < 2; j++) {
      var col = document.createElement("div");
      col.className = "col";
      for (var k = 0; k < 6; k++) {
        var s = document.createElement("div");
        s.className = "skel";
        s.style.width = (38 + (k % 3) * 18) + "%";
        col.appendChild(s);
      }
      cols.appendChild(col);
    }
    view.appendChild(cols);
  }
  function emptyMsg(kind, source) {
    if (source && source !== "index") return WARM;
    if (kind === "token") return "Tokens show up as history fills in.";
    if (kind === "block") return "No blocks yet.";
    if (kind === "xfer" || kind === "transfer") return "No transfers yet.";
    if (kind === "event") return "No events in this window.";
    if (kind === "internal") return "No internal calls.";
    return "No transactions in this window.";
  }
  function gasLoad(b) {
    if (!b) return null;
    if (b.load != null && b.load !== "") return Number(b.load);
    if (b.gas_used != null && b.gas_limit) return Number(b.gas_used) / Math.max(1, Number(b.gas_limit));
    return null;
  }
  function uiError(d) {
    var e = d && d.error ? String(d.error) : "";
    if (/blockscout|https?:\/\//i.test(e)) return WARM;
    if (e === "index wait" || e === "index offline") return WARM;
    if (!e || e === "rpc wait" || e === "transport" || e === "decode") return "RPC quiet. Retry.";
    return e;
  }

  function paintHead(d) {
    if (d && d.loading) {
      skel();
      return false;
    }
    clearBusy();
    if (!d || (!d.ok && d.block == null && !d.stale)) {
      err(uiError(d) || "RPC quiet. Retry stays in vapurr.");
      return false;
    }
    var liveOk = !!(d.ok && !d.stale);
    setLive(liveOk, (liveOk ? "live  ·  blk " : "stale  ·  blk ") + comma(d.block), d.ts);
    var ix = d.index || {};
    var gas = d.gas || {};
    var headStats = [
      ["block", comma(d.block)],
      ["base fee", d.base_fee || "—"],
      ["gas price", d.gwei || "—"],
      ["gas used", comma(d.gas_used || 0)],
      ["block txs", comma(d.txs || 0)],
      ["chain txs", ix.total_txs ? comma(ix.total_txs) : "—"],
      ["l1", d.l1 ? comma(d.l1) : "—"]
    ];
    if (ix.total_addresses) headStats.push(["addresses", comma(ix.total_addresses)]);
    if (ix.txs_today) headStats.push(["txs today", comma(ix.txs_today)]);
    if (ix.avg_ms) headStats.push(["block time", msLabel(ix.avg_ms)]);
    if (d.eth_usd) headStats.push(["ETH", px(d.eth_usd)]);
    headStats.push(["liq tvl", usd(d.liq && d.liq.tvl_usd), function () { goLiq(); }, "Liquidity"]);
    headStats.push([liqVolLabel(d.liq), usd(d.liq && d.liq.vol24_usd), function () { goLiq(); }, "Liquidity"]);
    var frac = gas.relative != null ? gas.relative : (d.spark && d.spark[0]) || 0;
    var existing = view.getAttribute("data-scan") === "head" && view.querySelector(".stats") && view.querySelector(".cols");
    if (existing) {
      patchStatGrid(headStats);
      var meter = view.querySelector(".meter-wrap");
      if (meter) meter.replaceWith(meterEl(frac, gas.heat, gasCaption(gas)));
      else view.insertBefore(meterEl(frac, gas.heat, gasCaption(gas)), view.querySelector(".spark") || view.querySelector(".cols"));
      if (d.spark && d.spark.length) {
        if (view.querySelector(".spark")) patchSpark(d.spark);
        else view.insertBefore(sparkEl(d.spark), view.querySelector(".cols"));
      }
      var txk = rowKey(d.transactions, "hash");
      var blk = rowKey(d.blocks, "number");
      if (view.getAttribute("data-block") !== String(d.block) || view.getAttribute("data-txk") !== txk || view.getAttribute("data-blk") !== blk) {
        var cols = view.querySelector(".cols");
        var neu = document.createElement("div");
        neu.className = "cols stack";
        neu.appendChild(blockTable("Latest blocks", d.blocks || [], d.block_source || d.source));
        neu.appendChild(txTable("Latest transactions", d.transactions || [], d.tx_source || d.source));
        if (cols) cols.replaceWith(neu);
        else view.appendChild(neu);
        view.setAttribute("data-block", String(d.block));
        view.setAttribute("data-txk", txk);
        view.setAttribute("data-blk", blk);
      }
      return true;
    }
    view.innerHTML = "";
    view.setAttribute("data-scan", "head");
    view.setAttribute("data-block", String(d.block));
    view.setAttribute("data-txk", rowKey(d.transactions, "hash"));
    view.setAttribute("data-blk", rowKey(d.blocks, "number"));
    view.appendChild(statGrid(headStats));
    view.appendChild(meterEl(frac, gas.heat, gasCaption(gas)));
    if (d.spark && d.spark.length) view.appendChild(sparkEl(d.spark));
    var cols = document.createElement("div");
    cols.className = "cols stack";
    cols.appendChild(blockTable("Latest blocks", d.blocks || [], d.block_source || d.source));
    cols.appendChild(txTable("Latest transactions", d.transactions || [], d.tx_source || d.source));
    view.appendChild(cols);
    return true;
  }

  function emptyEl(text) {
    var e = document.createElement("div");
    e.className = "empty";
    e.textContent = text || WARM;
    return e;
  }

  function blockTable(title, rows, source) {
    var col = document.createElement("div");
    col.className = "col";
    if (title) {
      var h = document.createElement("h2");
      h.textContent = title + (source === "index" ? "  ·  history" : "");
      col.appendChild(h);
    }
    if (!rows.length) {
      col.appendChild(emptyEl(emptyMsg("block", source)));
      return col;
    }
    var t = document.createElement("table");
    t.className = "grid";
    t.innerHTML = "<thead><tr><th>Block</th><th>Age</th><th>Txn</th><th>Sequencer</th><th>Gas used</th><th>Base fee</th><th>L1</th></tr></thead>";
    var body = document.createElement("tbody");
    rows.forEach(function (b) {
      var tr = document.createElement("tr");
      var td0 = document.createElement("td");
      td0.appendChild(hashWrap(String(b.number), "block"));
      var td1 = document.createElement("td");
      td1.appendChild(ageEl(b.ts));
      var td2 = document.createElement("td");
      td2.textContent = comma(b.txs || 0);
      var tdSeq = document.createElement("td");
      if (b.miner) tdSeq.appendChild(addrChip(b.miner, "addr", false, "Sequencer"));
      else tdSeq.textContent = "—";
      var tdGas = document.createElement("td");
      tdGas.className = "meta";
      tdGas.textContent = b.gas_used != null && b.gas_used !== "" ? comma(b.gas_used) : "—";
      var tdFee = document.createElement("td");
      tdFee.className = "meta";
      tdFee.textContent = b.base_fee || (b.base_fee_n != null ? Number(b.base_fee_n).toFixed(2) + " GWEI" : "—");
      var tdL1 = document.createElement("td");
      tdL1.className = "meta";
      tdL1.textContent = b.l1 ? comma(b.l1) : "—";
      tr.appendChild(td0); tr.appendChild(td1); tr.appendChild(td2);
      tr.appendChild(tdSeq); tr.appendChild(tdGas); tr.appendChild(tdFee); tr.appendChild(tdL1);
      armRow(tr, function () { goBlock(b.number); });
      body.appendChild(tr);
    });
    t.appendChild(body);
    col.appendChild(t);
    return col;
  }

  function txTable(title, rows, source, emptyText) {
    var col = document.createElement("div");
    col.className = "col";
    if (title) {
      var h = document.createElement("h2");
      h.textContent = title + (source === "index" ? "  ·  history" : "");
      col.appendChild(h);
    }
    if (!rows.length) {
      col.appendChild(emptyEl(emptyText || emptyMsg("tx", source)));
      return col;
    }
    var t = document.createElement("table");
    t.className = "grid";
    t.innerHTML = "<thead><tr><th>Txn</th><th>Method</th><th>Block</th><th>From</th><th>To</th><th>Value</th><th>Gas price</th><th>Age</th></tr></thead>";
    var body = document.createElement("tbody");
    rows.forEach(function (x) {
      body.appendChild(txRow(x));
    });
    t.appendChild(body);
    col.appendChild(t);
    return col;
  }
  function txRow(x) {
    var tr = document.createElement("tr");
    var td0 = document.createElement("td");
    td0.appendChild(hashWrap(x.hash || x.tx, "tx", x));
    var tdM = document.createElement("td");
    var method = pillEl(methodLabel(x.method || x.event || x.symbol || "call"), x.status === 0 ? "bad" : "on");
    tdM.appendChild(method);
    var tdBlk = document.createElement("td");
    if (x.block != null && x.block !== "") tdBlk.appendChild(hashWrap(String(x.block), "block"));
    else tdBlk.textContent = "—";
    var tdFrom = document.createElement("td");
    if (x.from) tdFrom.appendChild(addrChip(x.from, "addr", false, x.from_pns || x.from_label));
    else tdFrom.textContent = "—";
    var tdTo = document.createElement("td");
    if (x.to) tdTo.appendChild(addrChip(x.to, "addr", false, x.to_pns || x.to_label));
    else tdTo.textContent = x.contract ? "create" : "—";
    var td2 = document.createElement("td");
    var shown = x.headline || x.value || x.amount || "";
    if (!hasNativeValue(shown) && x.amount) shown = x.amount;
    td2.textContent = shown || "0 ETH";
    var tdGas = document.createElement("td");
    tdGas.className = "meta";
    tdGas.textContent = x.gas_price || x.fee || "—";
    var tdAge = document.createElement("td");
    tdAge.appendChild(ageEl(x.ts));
    tr.appendChild(td0); tr.appendChild(tdM); tr.appendChild(tdBlk);
    tr.appendChild(tdFrom); tr.appendChild(tdTo); tr.appendChild(td2); tr.appendChild(tdGas); tr.appendChild(tdAge);
    armRow(tr, function () { goTx(x.hash || x.tx, x); });
    return tr;
  }
  function asTxRow(r) {
    r = r || {};
    return {
      hash: r.hash || r.tx,
      tx: r.tx || r.hash,
      from: r.from,
      from_label: r.from_label,
      to: r.to,
      to_label: r.to_label,
      value: r.headline || r.value || r.amount,
      headline: r.headline,
      amount: r.amount,
      method: r.method || r.symbol || r.type || r.event,
      event: r.event,
      symbol: r.symbol,
      ts: r.ts,
      status: r.status,
      block: r.block,
      gas_price: r.gas_price,
      fee: r.fee
    };
  }
  function hasNativeValue(v) {
    v = String(v == null ? "" : v).replace(/\s+/g, " ").trim();
    if (!v) return false;
    if (/^0+(\.0+)?( ETH)?$/i.test(v)) return false;
    return true;
  }
  function actionLine() {
    var line = document.createElement("div");
    line.className = "action-line";
    var i;
    for (i = 0; i < arguments.length; i++) {
      var bit = arguments[i];
      if (bit == null || bit === "") continue;
      if (typeof bit === "string") line.appendChild(document.createTextNode(bit));
      else line.appendChild(bit);
    }
    return line;
  }
  function transferActionLine(amount, from, to) {
    return actionLine(
      "Transfer " + amount + " from ",
      addrChip(from, "addr"),
      " to ",
      addrChip(to, "addr")
    );
  }
  function tokenActionAmount(t) {
    var amt = t.amount || t.value || "";
    var sym = t.usdg ? "USDG" : (t.symbol || "");
    var shown = String(amt);
    if (sym && shown.indexOf(sym) === -1) shown = shown ? shown + " " + sym : sym;
    return shown;
  }
  function txActionBody(d) {
    var box = document.createElement("div");
    var native = String(d.value || "").replace(/\s+/g, " ").trim();
    var hasNative = hasNativeValue(native);
    var xfers = d.transfers || [];
    if (hasNative) {
      box.appendChild(transferActionLine(native, d.from, d.to || d.contract || ""));
    }
    xfers.forEach(function (t) {
      box.appendChild(transferActionLine(tokenActionAmount(t), t.from, t.to));
    });
    if (!hasNative && !xfers.length) {
      box.appendChild(actionLine(
        "Interacted with ",
        addrChip(d.to || d.contract || "", "addr"),
        " ",
        pillEl(methodLabel(d.method || d.action || "call"))
      ));
    }
    return box;
  }
  function txActionSheet(d) {
    return detailsPanel("Transaction Action", txActionBody(d));
  }
  function tokenXferTable(rows, nft) {
    var t = document.createElement("table");
    t.className = "grid";
    t.innerHTML = nft
      ? "<thead><tr><th>NFT</th><th>From</th><th>To</th><th>Token ID</th></tr></thead>"
      : "<thead><tr><th>Token</th><th>From</th><th>To</th><th>Amount</th></tr></thead>";
    var body = document.createElement("tbody");
    (rows || []).forEach(function (x) {
      var tr = document.createElement("tr");
      var tdTok = document.createElement("td");
      var lab = document.createElement("div");
      lab.textContent = x.usdg ? "USDG" : (x.symbol || x.event || (nft ? "NFT" : "Token"));
      tdTok.appendChild(lab);
      if (x.token || x.address) {
        var c = document.createElement("div");
        c.className = "meta";
        c.appendChild(hashWrap(x.token || x.address, "token"));
        tdTok.appendChild(c);
      }
      var tdFrom = document.createElement("td");
      if (x.from) tdFrom.appendChild(addrChip(x.from, "addr"));
      else tdFrom.textContent = "—";
      var tdTo = document.createElement("td");
      if (x.to) tdTo.appendChild(addrChip(x.to, "addr"));
      else tdTo.textContent = "—";
      var tdAmt = document.createElement("td");
      tdAmt.textContent = nft
        ? (x.token_id || x.amount || "—")
        : (tokenActionAmount(x) || x.amount || x.value || "—");
      tr.appendChild(tdTok); tr.appendChild(tdFrom); tr.appendChild(tdTo); tr.appendChild(tdAmt);
      if (x.tx) armRow(tr, function () { goTx(x.tx, x); });
      body.appendChild(tr);
    });
    t.appendChild(body);
    return t;
  }

  function paintBlock(d) {
    clearBusy();
    view.innerHTML = "";
    if (!d || !d.ok) return err(uiError(d) || "block failed");
    state.head = d.head;
    state.id = d.number;
    view.appendChild(crumb(["block " + comma(d.number)]));
    var pager = document.createElement("div");
    pager.className = "pager";
    if (d.number > 0) {
      var prev = document.createElement("button");
      prev.className = "btn-ghost";
      prev.type = "button";
      prev.textContent = "←  " + comma(d.number - 1);
      prev.onclick = function () { goBlock(d.number - 1, 1, { pager: true }); };
      pager.appendChild(prev);
    }
    var atHead = !!(d.latest || (d.head != null && d.number >= d.head));
    if (!atHead) {
      var next = document.createElement("button");
      next.className = "btn-ghost";
      next.type = "button";
      next.textContent = comma(d.number + 1) + "  →";
      next.onclick = function () { goBlock(d.number + 1, 1, { pager: true }); };
      pager.appendChild(next);
    }
    view.appendChild(pager);
    var hero = document.createElement("div");
    hero.className = "hero";
    var left = document.createElement("div");
    var h = document.createElement("h1");
    h.textContent = "Block " + comma(d.number);
    left.appendChild(h);
    var pills = document.createElement("div");
    pills.className = "pills";
    pills.appendChild(pillEl((d.tx_count || 0) + " txs", "on"));
    if (d.latest) pills.appendChild(pillEl("head", "on"));
    left.appendChild(pills);
    hero.appendChild(left);
    view.appendChild(hero);
    view.appendChild(detailsPanel("Block Details", kv([
      ["Block Height", comma(d.number)],
      ["Timestamp", d.ts, null, "age"],
      ["Transactions", comma(d.tx_count || 0)],
      ["Hash", d.hash, "block", null, true],
      ["Parent Hash", d.parent, "block"],
      ["Sequencer", d.miner, "addr", null, false, "Sequencer"],
      ["Gas Used", comma(d.gas_used || 0) + (d.gas_limit ? " / " + comma(d.gas_limit) : "")],
      ["Base Fee Per Gas", d.base_fee],
      ["Size", d.size ? comma(d.size) + " bytes" : ""],
      ["L1 Block Number", d.l1 ? comma(d.l1) : ""],
      ["More Details", "", "section"],
      ["State Root", d.state_root],
      ["Transactions Root", d.tx_root],
      ["Receipts Root", d.receipts_root],
      ["Extra Data", d.extra]
    ], true)));
    view.appendChild(txTable("Transactions", d.txs || []));
    if (d.pages && d.pages > 1) {
      var tp = document.createElement("div");
      tp.className = "pager";
      if (d.page > 1) {
        var back = document.createElement("button");
        back.className = "btn-ghost";
        back.type = "button";
        back.textContent = "←  txs";
        back.onclick = function () { goBlock(d.number, d.page - 1); };
        tp.appendChild(back);
      }
      var lab = document.createElement("span");
      lab.className = "meta";
      lab.textContent = "page " + d.page + " / " + d.pages;
      tp.appendChild(lab);
      if (d.page < d.pages) {
        var fwd = document.createElement("button");
        fwd.className = "btn-ghost";
        fwd.type = "button";
        fwd.textContent = "txs  →";
        fwd.onclick = function () { goBlock(d.number, d.page + 1); };
        tp.appendChild(fwd);
      }
      view.appendChild(tp);
    }
  }

  function paintDecoded(dec) {
    if (!dec || (!dec.args && !dec.params && !dec.words)) return;
    var args = dec.params || dec.args || [];
    var rows = [["method", dec.name || dec.method || ""], ["selector", dec.selector || ""]];
    args.forEach(function (a) {
      if (!a) return;
      rows.push([a.name || a.kind || "arg", a.value, (a.type === "address" || a.kind === "address") ? "addr" : ""]);
    });
    if (args.length) view.appendChild(kv(rows));
    if (dec.words && dec.words.length && !args.length) {
      var pre = document.createElement("pre");
      pre.className = "hex";
      pre.textContent = (dec.words || []).join("\n") + (dec.leftover ? "\n" + dec.leftover : "");
      var lab = document.createElement("h2");
      lab.textContent = "Calldata";
      view.appendChild(lab);
      view.appendChild(pre);
    }
  }

  function paintLogs(logs) {
    var col = document.createElement("div");
    col.className = "col";
    var h2 = document.createElement("h2");
    h2.textContent = "Logs";
    col.appendChild(h2);
    logs.forEach(function (lg) {
      var box = document.createElement("div");
      box.className = "log";
      var head = document.createElement("div");
      head.className = "loghead";
      var ev = (lg.decoded && lg.decoded.name) || (lg.event && lg.event !== "log" ? lg.event : "log");
      head.textContent = ev + (lg.index != null && lg.index !== "" ? "  ·  #" + lg.index : "");
      box.appendChild(head);
      var rows = [
        ["emitter", lg.address || lg.token || "", "addr"],
        ["index", lg.index != null ? String(lg.index) : ""]
      ];
      var params = lg.decoded && (lg.decoded.params || lg.decoded.args);
      if (params && params.length) {
        params.forEach(function (a) {
          if (!a) return;
          rows.push([
            a.name || a.kind || "arg",
            a.value,
            (a.type === "address" || a.kind === "address") ? "addr" : ""
          ]);
        });
      } else {
        if (lg.from) rows.push(["from", lg.from, "addr"]);
        if (lg.to) rows.push(["to", lg.to, "addr"]);
        if (lg.spender) rows.push(["spender", lg.spender, "addr"]);
        if (lg.amount) rows.push(["amount", lg.amount]);
      }
      box.appendChild(kv(rows));
      if (lg.tx) {
        box.style.cursor = "pointer";
        box.onclick = function () { goTx(lg.tx); };
      }
      if (lg.topics && lg.topics.length) {
        (function (topics) {
          var tog = document.createElement("button");
          tog.className = "btn-ghost";
          tog.type = "button";
          tog.textContent = "Raw topics";
          var top = document.createElement("pre");
          top.className = "hex";
          top.style.display = "none";
          top.textContent = topics.map(function (t, i) { return "[" + i + "] " + t; }).join("\n");
          tog.onclick = function (e) {
            e.stopPropagation();
            var on = top.style.display === "none";
            top.style.display = on ? "block" : "none";
            tog.textContent = on ? "Hide topics" : "Raw topics";
          };
          box.appendChild(tog);
          box.appendChild(top);
        })(lg.topics);
      }
      if (lg.data && lg.data !== "0x" && lg.data !== "0x0") {
        (function (hex) {
          var tog = document.createElement("button");
          tog.className = "btn-ghost";
          tog.type = "button";
          tog.textContent = "Raw data";
          var data = document.createElement("pre");
          data.className = "hex";
          data.style.display = "none";
          data.textContent = hex;
          tog.onclick = function (e) {
            e.stopPropagation();
            var on = data.style.display === "none";
            data.style.display = on ? "block" : "none";
            tog.textContent = on ? "Hide data" : "Raw data";
          };
          box.appendChild(tog);
          box.appendChild(data);
        })(lg.data);
      }
      col.appendChild(box);
    });
    return col;
  }

  function paintTx(d) {
    if (d && d.tx && (d.kind === "tx" || (d.tx.hash && !d.hash))) d = d.tx;
    clearBusy();
    view.innerHTML = "";
    if (!d || !d.ok) {
      return err((uiError(d) || "tx failed") + (state.id ? "  ·  " + short(state.id, 8) : ""));
    }
    view.appendChild(crumb(["transaction"]));
    var pending = d.status !== 0 && d.status !== 1;
    var statusText = d.status === 1 ? "success" : d.status === 0 ? "reverted" : (d.partial ? "head" : "pending");
    var hash = d.hash || state.id || "";
    var hero = document.createElement("div");
    hero.className = "hero";
    var left = document.createElement("div");
    var h = document.createElement("h1");
    h.textContent = "Transaction Details";
    left.appendChild(h);
    var pills = document.createElement("div");
    pills.className = "pills";
    pills.appendChild(pillEl(statusText, d.status === 1 ? "on" : d.status === 0 ? "bad" : ""));
    pills.appendChild(pillEl(methodLabel(d.method || d.action || "call")));
    pills.appendChild(pillEl(d.type_name || typeName(d.type)));
    if (d.confirmations) pills.appendChild(pillEl(comma(d.confirmations) + " Block Confirmations", "on"));
    left.appendChild(pills);
    var src = document.createElement("div");
    src.className = "meta";
    src.textContent = (d.chain || "Robinhood Chain") + " · " + (d.chain_id || 4663);
    left.appendChild(src);
    hero.appendChild(left);
    view.appendChild(hero);
    var blockVal = pending || d.block == null || d.block === "" || Number(d.block) === 0 ? null : d.block;
    var blockCell = document.createElement("span");
    if (blockVal != null) {
      blockCell.appendChild(hashWrap(String(blockVal), "block"));
      if (d.confirmations) {
        blockCell.appendChild(document.createTextNode("  ·  " + comma(d.confirmations) + " Block Confirmations"));
      }
    } else {
      blockCell.textContent = pending ? "Pending" : "—";
    }
    var statusCell = document.createElement("span");
    statusCell.appendChild(pillEl(statusText, d.status === 1 ? "on" : d.status === 0 ? "bad" : ""));
    if (d.revert) {
      var rv = document.createElement("span");
      rv.className = "meta";
      rv.textContent = "  ·  " + d.revert;
      statusCell.appendChild(rv);
    }
    var gasLine = "—";
    if (d.gas_used != null) {
      gasLine = comma(d.gas_used) + " / " + comma(d.gas || 0);
      if (d.gas_pct != null) gasLine += " (" + Number(d.gas_pct).toFixed(2) + "%)";
    } else if (d.gas) {
      gasLine = comma(d.gas) + " limit";
    }
    var feeLine = d.fee || "—";
    var gp = d.effective_gas_price || d.gas_price;
    if (gp && gp !== "—") feeLine = (d.fee && d.fee !== "—" ? d.fee + "  ·  " : "") + gp;
    if (d.fee_usd) feeLine += "  ·  " + d.fee_usd;
    var inputBytes = d.input_bytes;
    if (inputBytes == null && d.input && d.input.length > 2) {
      inputBytes = Math.max(0, (d.input.length - 2) / 2);
    }
    var logCount = d.log_count != null ? d.log_count : ((d.logs || []).length);
    var toAddr = d.to || d.contract || "";
    var valueLine = "";
    if (hasNativeValue(d.value)) {
      valueLine = d.value;
      if (d.value_usd) valueLine += "  ·  " + d.value_usd;
    } else if (d.headline && d.headline !== d.value) {
      valueLine = d.headline;
    } else {
      valueLine = d.value || "0 ETH";
    }
    view.appendChild(detailsPanel("Transaction Details", kv([
      ["Transaction Hash", hash, "tx", null, true],
      ["Status", statusCell, "node"],
      ["Block", blockCell, "node"],
      ["Timestamp", d.ts, null, "age"],
      ["From", d.from, "addr", null, false, d.from_pns || d.from_label || pnsName(d.from) || knownLabel(d.from)],
      ["To", toAddr, toAddr ? "addr" : "", null, false, d.to_pns || d.to_label || pnsName(toAddr) || knownLabel(toAddr)],
      ["Transaction Action", txActionBody(d), "node"],
      ["Value", valueLine],
      ["Transaction Fee", feeLine],
      ["Burnt Fees", d.burnt],
      ["Txn Savings", d.savings],
      ["More Details", "", "section"],
      ["Gas Limit & Usage by Txn", gasLine],
      ["Gas Price", gp],
      ["Max Fee Per Gas", d.max_fee],
      ["Max Priority Fee", d.priority],
      ["Nonce", d.nonce != null && d.nonce !== "" ? comma(d.nonce) : ""],
      ["Position in Block", d.index != null && d.index !== "" ? comma(d.index) : ""],
      ["Transaction Type", d.type_name || typeName(d.type)],
      ["L1 Block Number", d.l1 ? comma(d.l1) : ""],
      ["L1 Gas Used", d.l1_gas != null && d.l1_gas !== "" ? comma(d.l1_gas) : ""],
      ["L1 Fees Paid", d.l1_fee],
      ["L1 Gas Price", d.l1_gas_price],
      ["Input Data", inputBytes != null ? comma(inputBytes) + " bytes" : ""],
      ["Logs", logCount != null ? comma(logCount) : ""],
      ["Internal Txns", d.index_loading && !(d.internal && d.internal.length) ? "" : ((d.internal && d.internal.length) ? comma(d.internal.length) : "")],
      ["Block Hash", d.block_hash, "block", null, true]
    ], true)));
    if (d.decoded && (d.decoded.args || d.decoded.params || d.decoded.words)) {
      var decHold = document.createElement("div");
      var args = d.decoded.params || d.decoded.args || [];
      var rows = [["Method", methodLabel(d.decoded.name || d.decoded.method || "")], ["Selector", d.decoded.selector || ""]];
      args.forEach(function (a) {
        if (!a) return;
        rows.push([a.name || a.kind || "arg", a.value, (a.type === "address" || a.kind === "address") ? "addr" : ""]);
      });
      decHold.appendChild(kv(rows, true));
      if (d.decoded.words && d.decoded.words.length && !args.length) {
        var preW = document.createElement("pre");
        preW.className = "hex";
        preW.textContent = (d.decoded.words || []).join("\n") + (d.decoded.leftover ? "\n" + d.decoded.leftover : "");
        decHold.appendChild(preW);
      }
      view.appendChild(detailsPanel("Input Data", decHold));
    }
    var xfers = d.transfers || [];
    var erc20 = xfers.filter(function (x) { return !isNftXfer(x); });
    var nfts = xfers.filter(isNftXfer);
    if (erc20.length) {
      view.appendChild(detailsPanel("ERC-20 Token Transfers (" + erc20.length + ")", tokenXferTable(erc20)));
    }
    if (nfts.length) {
      view.appendChild(detailsPanel("NFT Transfers (" + nfts.length + ")", tokenXferTable(nfts, true)));
    }
    var internals = d.internal || [];
    if (internals.length) {
      view.appendChild(detailsPanel(
        "Internal Transactions (" + internals.length + ")",
        txTable("", internals.map(asTxRow), "index")
      ));
    }
    if ((d.logs || []).length) view.appendChild(paintLogs(d.logs));
    else if (d.log_count > 0 || d.index_loading) {
      view.appendChild(emptyEl(WARM));
    }
    if (d.input && d.input.length > 4) {
      var col = document.createElement("div");
      col.className = "col sheet";
      var tools = document.createElement("div");
      tools.className = "pager";
      var pre = document.createElement("pre");
      pre.className = "hex";
      function showInput(mode) {
        if (mode === "utf8") pre.textContent = hexUtf8(d.input) || "(no utf-8)";
        else pre.textContent = d.input;
      }
      [["Original", "hex"], ["UTF-8", "utf8"]].forEach(function (pair) {
        var b = document.createElement("button");
        b.className = "btn-ghost";
        b.type = "button";
        b.textContent = pair[0];
        b.onclick = function () { showInput(pair[1]); };
        tools.appendChild(b);
      });
      showInput("hex");
      col.appendChild(tools);
      col.appendChild(pre);
      view.appendChild(detailsPanel("Input Data · raw", col));
    }
  }

  function paintAddr(d, tab) {
    clearBusy();
    view.innerHTML = "";
    if (!d || !d.ok) return err(uiError(d) || "address failed");
    view.appendChild(crumb(["address"]));
    var hero = document.createElement("div");
    hero.className = "hero";
    var left = document.createElement("div");
    var h = document.createElement("h1");
    h.appendChild(addrChip(d.address, "addr", false, d.pns || d.name || knownLabel(d.address)));
    left.appendChild(h);
    var pills = document.createElement("div");
    pills.className = "pills";
    pills.appendChild(pillEl(d.contract ? "contract" : "account", d.contract || d.verified ? "on" : ""));
    if (d.verified) pills.appendChild(pillEl("verified", "on"));
    if (d.pns) pills.appendChild(pillEl(d.pns, "on"));
    if (d.name && d.name !== d.pns) pills.appendChild(pillEl(d.name));
    left.appendChild(pills);
    hero.appendChild(left);
    view.appendChild(hero);
    view.appendChild(statGrid([
      ["ETH", d.eth || "0 ETH"],
      ["USDG", d.usdg || "0 USDG"],
      ["nonce", comma(d.nonce)],
      ["code", d.contract ? comma(d.code_bytes || 0) + " B" : "EOA"],
      ["history", d.indexed ? "full" : "last " + comma(d.span || 0)]
    ]));
    var accountRows = [
      ["address", d.address, "addr", null, true, d.pns || d.name || knownLabel(d.address)]
    ];
    if (d.pns) accountRows.push(["PNS", d.pns]);
    if (d.name && d.name !== d.pns) accountRows.push(["name", d.name]);
    accountRows.push(
      ["creator", d.creator || "", d.creator ? "addr" : ""],
      ["creation tx", d.creation_tx || "", d.creation_tx ? "tx" : ""]
    );
    view.appendChild(sheet("Account", kv(accountRows)));
    var tabs = document.createElement("div");
    tabs.className = "tabs";
    var body = document.createElement("div");
    var xfers = (d.token_transfers && d.token_transfers.length) ? d.token_transfers : (d.transfers || []);
    var events = d.events || d.logs || [];
    var panes = [
      ["txs", "Transactions", d.txs || [], d.txs_next, "tx"],
      ["tokens", "Tokens", pinUsdg(d.tokens || []), d.tokens_next, "token"],
      ["xfers", "Transfers", xfers, d.xfers_next, "xfer"],
      ["internal", "Internal", d.internal || [], d.internal_next, "internal"]
    ];
    if (d.contract) {
      panes.push(["events", "Events", events, d.events_next, "event"]);
      panes.push(["contract", "Contract", [d], null, ""]);
    }
    function addrMore(id, next) {
      return morePager(next, function () {
        goAddr(d.address, id, next, { pager: true, more: true });
      }, (state.stack || []).length > 0, function () {
        var popped = popStack(state.stack);
        goAddr(d.address, id, popped.cursor, { pager: true, prev: true, stack: popped.stack });
      });
    }
    function show(id) {
      Array.prototype.forEach.call(tabs.children, function (b) {
        b.classList.toggle("on", b.getAttribute("data-id") === id);
      });
      body.innerHTML = "";
      var pack = panes.filter(function (p) { return p[0] === id; })[0];
      if (!pack) return;
      var rows = pack[2];
      if (id === "events") {
        if (!rows.length) body.appendChild(emptyEl(emptyMsg("event", d.source || (d.indexed === false ? "rpc" : "index"))));
        else {
          if (d.source !== "index" || d.indexed === false) body.appendChild(emptyEl(WARM));
          body.appendChild(paintLogs(rows.map(function (r) {
          return {
            tx: r.hash || r.tx,
            from: r.from,
            to: r.to,
            amount: r.value || r.amount,
            event: r.event || r.symbol || "log",
            address: r.token || r.address,
            usdg: r.usdg,
            topics: r.topics,
            data: r.data,
            index: r.index,
            decoded: r.decoded
          };
        })));
        }
        var em = addrMore(id, pack[3]);
        if (em) body.appendChild(em);
        return;
      }
      if (id === "contract") {
        if (!d.contract) {
          body.appendChild(emptyEl("Not a contract."));
          return;
        }
        body.appendChild(kv([
          ["creator", d.creator || "", d.creator ? "addr" : ""],
          ["creation tx", d.creation_tx || "", d.creation_tx ? "tx" : ""],
          ["compiler", d.compiler || ""],
          ["optimization", d.optimization ? ("yes" + (d.optimization_runs ? " · " + comma(d.optimization_runs) : "")) : ""],
          ["license", d.license || ""],
          ["proxy", d.proxy || ""],
          ["implementation", d.implementation || "", d.implementation ? "addr" : ""],
          ["bytecode", d.code_bytes ? comma(d.code_bytes) + " bytes" : ""]
        ]));
        if (d.source) {
          var srcPre = document.createElement("pre");
          srcPre.className = "hex";
          srcPre.textContent = d.source;
          body.appendChild(detailsPanel("Source", srcPre));
        } else if (d.verified) {
          body.appendChild(emptyEl(d.loading ? WARM : "Verified, source not in this window."));
        } else if (d.loading) {
          body.appendChild(emptyEl(WARM));
        }
        if (d.abi) {
          var abiPre = document.createElement("pre");
          abiPre.className = "hex";
          try { abiPre.textContent = typeof d.abi === "string" ? d.abi : JSON.stringify(d.abi, null, 2); }
          catch (e) { abiPre.textContent = String(d.abi); }
          body.appendChild(detailsPanel("ABI", abiPre));
        }
        if (d.code) {
          var pre = document.createElement("pre");
          pre.className = "hex";
          pre.textContent = d.code;
          body.appendChild(detailsPanel("Bytecode", pre));
        }
        return;
      }
      if (id === "tokens") {
        if (!rows.length) {
          body.appendChild(emptyEl(d.indexed ? "No token balances." : WARM));
        } else {
          var col = document.createElement("div");
          col.className = "col";
          var t = document.createElement("table");
          t.className = "grid";
          t.innerHTML = "<thead><tr><th>Token</th><th>Contract</th><th>Amount</th><th>USD</th></tr></thead>";
          var tb = document.createElement("tbody");
          rows.forEach(function (tok) {
            var tr = document.createElement("tr");
            var tdTok = document.createElement("td");
            var lab = document.createElement("span");
            lab.textContent = tok.symbol || tok.name || short(tok.token || tok.address, 6);
            tdTok.appendChild(lab);
            if (tok.usdg) {
              tdTok.appendChild(document.createTextNode(" "));
              tdTok.appendChild(pillEl("unit", "on"));
            }
            var tdCtr = document.createElement("td");
            if (tok.token || tok.address) tdCtr.appendChild(hashWrap(tok.token || tok.address, "token"));
            else tdCtr.textContent = "—";
            var tdAmt = document.createElement("td");
            tdAmt.textContent = tok.amount || "";
            var tdUsd = document.createElement("td");
            tdUsd.className = "meta";
            tdUsd.textContent = tok.usd || "—";
            tr.appendChild(tdTok); tr.appendChild(tdCtr); tr.appendChild(tdAmt); tr.appendChild(tdUsd);
            armRow(tr, function () { goToken(tok.token || tok.address); });
            tb.appendChild(tr);
          });
          t.appendChild(tb);
          col.appendChild(t);
          body.appendChild(col);
        }
      } else {
        if ((d.source !== "index" || d.indexed === false) && rows.length) {
          body.appendChild(emptyEl(WARM));
        }
        body.appendChild(txTable(pack[1], rows.map(asTxRow), d.source, emptyMsg(pack[4] || "tx", d.source)));
      }
      var more = addrMore(id, pack[3]);
      if (more) body.appendChild(more);
    }
    panes.forEach(function (p) {
      var b = document.createElement("button");
      b.type = "button";
      b.setAttribute("data-id", p[0]);
      b.textContent = p[1] + (p[0] === "contract" ? "" : " · " + comma(p[2].length) + (p[3] ? "+" : ""));
      b.onclick = function () { goAddr(d.address, p[0], null, { pager: true }); };
      tabs.appendChild(b);
    });
    view.appendChild(tabs);
    view.appendChild(body);
    var start = tab || (panes[0][2].length ? "txs" : (panes[2][2].length ? "xfers" : "txs"));
    if (!panes.filter(function (p) { return p[0] === start; })[0]) start = "txs";
    show(start);
  }

  function paintToken(d) {
    clearBusy();
    view.innerHTML = "";
    if (!d || !d.ok) return err(uiError(d) || "index wait");
    view.appendChild(crumb(["token", d.symbol || ""]));
    var h = document.createElement("h1");
    h.textContent = d.symbol || "Token";
    view.appendChild(h);
    var pills = document.createElement("div");
    pills.className = "pills";
    pills.appendChild(pillEl(d.type || "ERC-20", d.usdg || d.verified ? "on" : ""));
    if (d.usdg) pills.appendChild(pillEl("unit of account", "on"));
    if (d.verified) pills.appendChild(pillEl("verified", "on"));
    view.appendChild(pills);
    if (d.name) {
      var nm = document.createElement("div");
      nm.className = "meta";
      nm.textContent = d.name;
      view.appendChild(nm);
    }
    var holderCount = tokenHolderCensus(d);
    var holderLabel = holderCount == null ? "—" : comma(holderCount);
    var market = [
      ["holders", holderLabel],
      ["supply", d.supply || "—"],
      ["decimals", d.decimals != null ? String(d.decimals) : "—"]
    ];
    if (d.price_usd) market.push(["price", px(d.price_usd)]);
    if (d.tvl_usd) market.push(["liq tvl", usd(d.tvl_usd), function () { goLiq(); }, "Liquidity"]);
    if (d.vol24_usd) market.push(["liq 24h", usd(d.vol24_usd), function () { goLiq(); }, "Liquidity"]);
    view.appendChild(statGrid(market));
    view.appendChild(sheet("Overview", kv([
      ["address", d.address, "addr", null, true],
      ["holders", holderCount == null ? "" : comma(holderCount)],
      ["supply", d.supply],
      ["decimals", d.decimals],
      ["price", d.price_usd ? px(d.price_usd) : ""],
      ["24h", d.change24 ? pct(d.change24) : ""],
      ["pools", d.degree != null && d.degree !== "" ? comma(d.degree) : ""]
    ])));
    if (d.liq) {
      var toMap = document.createElement("button");
      toMap.className = "btn-ghost";
      toMap.type = "button";
      toMap.textContent = "View on liquidity map";
      toMap.onclick = function () { goLiq(); };
      view.appendChild(toMap);
    }
    if ((d.pools || []).length) {
      view.appendChild(sheet("Pools", poolTable(d.pools, 8)));
    }
    var holderRows = d.holder_list || (Array.isArray(d.holders) ? d.holders : []);
    var xferRows = d.transfers || [];
    var tabs = document.createElement("div");
    tabs.className = "tabs";
    var body = document.createElement("div");
    var tokTab = state.tab === "holders" || state.tab === "transfers" ? state.tab : (holderRows.length ? "holders" : "transfers");
    function tokenMore(kind, next) {
      var isHolders = kind === "holders";
      var stack = isHolders ? (state.holderStack || []) : (state.xferStack || []);
      return morePager(next, function () {
        goToken(d.address, isHolders ? state.cursor : next, isHolders ? next : state.holders, {
          pager: true,
          tab: kind,
          moreHolders: isHolders,
          moreXfer: !isHolders
        });
      }, stack.length > 0, function () {
        var popped = popStack(stack);
        goToken(
          d.address,
          isHolders ? state.cursor : popped.cursor,
          isHolders ? popped.cursor : state.holders,
          {
            pager: true,
            tab: kind,
            prev: true,
            holderStack: isHolders ? popped.stack : state.holderStack,
            xferStack: isHolders ? state.xferStack : popped.stack
          }
        );
      });
    }
    function showTok(id) {
      Array.prototype.forEach.call(tabs.children, function (b) {
        b.classList.toggle("on", b.getAttribute("data-id") === id);
      });
      body.innerHTML = "";
      if (id === "holders") {
        if (!holderRows.length) {
          body.appendChild(emptyEl(d.loading || d.index === false || d.indexed === false ? WARM : "No holders in this window."));
        } else {
          var col = document.createElement("div");
          col.className = "col";
          var t = document.createElement("table");
          t.className = "grid";
          t.innerHTML = "<thead><tr><th>Holder</th><th>Amount</th><th>%</th></tr></thead>";
          var tb = document.createElement("tbody");
          holderRows.forEach(function (row) {
            var tr = document.createElement("tr");
            var td0 = document.createElement("td");
            td0.appendChild(addrChip(row.address, "addr"));
            if (row.usdg) td0.appendChild(pillEl("unit", "on"));
            var td1 = document.createElement("td");
            td1.textContent = row.amount || row.value || "";
            var tdP = document.createElement("td");
            tdP.className = "meta";
            tdP.textContent = row.pct != null && row.pct !== "" ? Number(row.pct).toFixed(2) + "%" : "—";
            tr.appendChild(td0); tr.appendChild(td1); tr.appendChild(tdP);
            armRow(tr, function () { goAddr(row.address); });
            tb.appendChild(tr);
          });
          t.appendChild(tb);
          col.appendChild(t);
          body.appendChild(col);
        }
        var hp = tokenMore("holders", d.holders_next);
        if (hp) body.appendChild(hp);
        return;
      }
      if ((d.source !== "index" || d.index === false) && xferRows.length) {
        body.appendChild(emptyEl(WARM));
      }
      body.appendChild(txTable("", xferRows.map(function (t) {
        return { hash: t.tx, from: t.from, to: t.to, value: t.amount, method: t.symbol || "Transfer", ts: t.ts };
      }), d.source, emptyMsg("xfer", d.source)));
      var tp = tokenMore("transfers", d.transfers_next || d.next);
      if (tp) body.appendChild(tp);
    }
    [["holders", "Holders" + (holderCount != null ? " · " + comma(holderCount) : (d.holders_next ? " · " + comma(holderRows.length) + "+" : ""))],
     ["transfers", "Transfers · " + comma(xferRows.length) + ((d.transfers_next || d.next) ? "+" : "")]].forEach(function (p) {
      var b = document.createElement("button");
      b.type = "button";
      b.setAttribute("data-id", p[0]);
      b.textContent = p[1];
      b.onclick = function () {
        goToken(d.address, state.cursor, state.holders, { tab: p[0], pager: true });
      };
      tabs.appendChild(b);
    });
    view.appendChild(tabs);
    view.appendChild(body);
    showTok(tokTab);
  }

  function paintList(title, rows, kind, d) {
    clearBusy();
    view.innerHTML = "";
    var h = document.createElement("h1");
    h.textContent = title;
    view.appendChild(h);
    var source = d && d.source;
    var empty = emptyMsg(kind === "token" ? "token" : kind === "block" ? "block" : "tx", source);
    var degraded = source !== "index" || (d && d.index === false);
    if (degraded && rows.length && (kind === "block" || kind === "tx")) {
      view.appendChild(emptyEl(WARM));
    }
    if (kind === "token") {
      rows = pinUsdg(rows);
      if (rows.length > 48) rows = rows.slice(0, 48);
    }
    if (kind === "block") {
      if (!rows.length) view.appendChild(emptyEl(empty));
      else view.appendChild(blockTable("", rows, source));
    } else if (kind === "token") {
      var col = document.createElement("div");
      col.className = "col";
      if (tokenWarm(rows, d)) {
        col.appendChild(emptyEl(emptyMsg("token", source === "rpc" ? "rpc" : "index wait")));
      } else if (!rows.length) {
        col.appendChild(emptyEl(empty));
      } else {
        var hasPx = rows.some(function (tok) { return tok.price_usd || tok.tvl_usd; });
        var hasHold = rows.some(function (tok) { return tokenHolderCensus(tok) != null; });
        var heads = ["Token"];
        if (hasPx) heads.push("Price", "TVL");
        if (hasHold) heads.push("Holders");
        if (!hasPx) heads.push("Supply");
        var t = document.createElement("table");
        t.className = "grid";
        t.innerHTML = "<thead><tr>" + heads.map(function (h) { return "<th>" + h + "</th>"; }).join("") + "</tr></thead>";
        var tb = document.createElement("tbody");
        rows.forEach(function (tok) {
          var tr = document.createElement("tr");
          var td0 = document.createElement("td");
          if (tok.address) td0.appendChild(hashWrap(tok.address, "token"));
          var lab = document.createElement("div");
          lab.className = "meta";
          lab.textContent = tok.symbol || tok.name || "token";
          td0.appendChild(lab);
          if (tok.usdg) td0.appendChild(pillEl("unit", "on"));
          tr.appendChild(td0);
          if (hasPx) {
            var tdP = document.createElement("td");
            tdP.textContent = px(tok.price_usd);
            var tdT = document.createElement("td");
            tdT.textContent = usd(tok.tvl_usd);
            tr.appendChild(tdP); tr.appendChild(tdT);
          }
          if (hasHold) {
            var tdH = document.createElement("td");
            var census = tokenHolderCensus(tok);
            tdH.textContent = census == null ? "—" : comma(census);
            tr.appendChild(tdH);
          }
          if (!hasPx) {
            var td2 = document.createElement("td");
            td2.className = "meta";
            td2.textContent = tok.supply || "";
            tr.appendChild(td2);
          }
          armRow(tr, function () { goToken(tok.address); });
          tb.appendChild(tr);
        });
        t.appendChild(tb);
        col.appendChild(t);
      }
      view.appendChild(col);
    } else {
      if (!rows.length) view.appendChild(emptyEl(empty));
      else view.appendChild(txTable("", rows, source));
    }
    var more = morePager(d && d.next, function () {
      if (kind === "block") goBlocks(d.next, { more: true, pager: true });
      else if (kind === "token") goTokens(d.next, { more: true, pager: true });
      else goTxs(d.next, { more: true, pager: true });
    }, (state.stack || []).length > 0, function () {
      var popped = popStack(state.stack);
      if (kind === "block") goBlocks(popped.cursor, { prev: true, pager: true, stack: popped.stack });
      else if (kind === "token") goTokens(popped.cursor, { prev: true, pager: true, stack: popped.stack });
      else goTxs(popped.cursor, { prev: true, pager: true, stack: popped.stack });
    });
    if (more) view.appendChild(more);
  }

  function paintSearch(items, q) {
    clearBusy();
    view.innerHTML = "";
    var h = document.createElement("h1");
    h.textContent = "Search";
    view.appendChild(h);
    if (q) {
      var m = document.createElement("div");
      m.className = "meta";
      m.textContent = q;
      view.appendChild(m);
    }
    var col = document.createElement("div");
    col.className = "col";
    if (!items.length) {
      col.appendChild(emptyEl("Nothing matched. Try a 0x hash, address, or block height."));
    } else {
      var t = document.createElement("table");
      t.className = "grid";
      t.innerHTML = "<thead><tr><th>Hit</th><th>Name</th><th>Kind</th></tr></thead>";
      var tb = document.createElement("tbody");
      items.forEach(function (it) {
        var tr = document.createElement("tr");
        var td0 = document.createElement("td");
        var blockId = it.block != null && it.block !== "" ? it.block : it.block_number;
        var kind = it.kind && String(it.kind).indexOf("token") >= 0 && it.address ? "token"
          : it.address ? "addr"
          : it.tx ? "tx"
          : (blockId != null && blockId !== "" ? "block" : "");
        var hit = kind === "token" || kind === "addr" ? it.address
          : kind === "tx" ? it.tx
          : kind === "block" ? String(blockId) : "";
        if (hit) td0.appendChild(hashWrap(hit, kind));
        else td0.textContent = it.symbol || short(it.address || it.tx || it.block, 6);
        var td1 = document.createElement("td");
        td1.textContent = it.name || it.symbol || "";
        var td2 = document.createElement("td");
        td2.className = "meta";
        td2.textContent = it.kind || "";
        tr.appendChild(td0); tr.appendChild(td1); tr.appendChild(td2);
        armRow(tr, function () {
          if (kind === "token") goToken(it.address);
          else if (kind === "addr") goAddr(it.address);
          else if (kind === "tx") goTx(it.tx);
          else if (kind === "block") goBlock(blockId);
        });
        tb.appendChild(tr);
      });
      t.appendChild(tb);
      col.appendChild(t);
    }
    view.appendChild(col);
  }

  function qsCursor(cursor, extra) {
    var p = [];
    var c = cursorParam(cursor);
    if (c) p.push("cursor=" + encodeURIComponent(c));
    if (extra) {
      Object.keys(extra).forEach(function (k) {
        if (extra[k] != null && extra[k] !== "") p.push(k + "=" + encodeURIComponent(extra[k]));
      });
    }
    return p.length ? "?" + p.join("&") : "";
  }

  function goHead() {
    stop();
    state = { page: "head" };
    setHash("#/");
    dropQ();
    setTab("head");
    skel();
    var retried = false;
    function tick() {
      api("head").then(function (d) {
        if (state.page !== "head") return;
        if (d && d.loading) {
          if (!retried) {
            retried = true;
            skel();
            poll = setTimeout(tick, 200);
            return;
          }
          return err((d && d.error) || "RPC quiet. Retry stays in vapurr.");
        }
        if (!paintHead(d)) return;
        poll = setInterval(function () {
          api("head").then(function (d2) {
            if (state.page !== "head") return;
            if (d2 && !d2.loading) paintHead(d2);
          }).catch(function () {});
        }, 4000);
      }).catch(function () {
        if (state.page !== "head") return;
        if (!retried) {
          retried = true;
          poll = setTimeout(tick, 200);
          return;
        }
        err("RPC quiet. Retry stays in vapurr.");
      });
    }
    tick();
  }
  function goBlock(id, page, opts) {
    opts = opts || {};
    var prev = { page: state.page, id: state.id, head: state.head, txpage: state.txpage };
    stop();
    state = { page: "block", id: id, txpage: page, head: prev.head };
    var hash = "#/block/" + id + (page && Number(page) > 1 ? "?page=" + page : "");
    setHash(hash);
    dropQ();
    setTab("blocks");
    if (!opts.pager) skel("block  " + id);
    api(scanApi("block", id, page && Number(page) > 1 ? { page: String(page) } : null)).then(function (d) {
      if (!d || !d.ok) {
        if (opts.pager) {
          state = prev;
          if (prev.page === "block") setHash("#/block/" + prev.id);
          toast((d && d.error) === "block not found" ? "at head" : ((d && d.error) || "block not found"));
          return;
        }
        return err(uiError(d) || "block failed");
      }
      paintBlock(d);
    }).catch(function () {
      if (opts.pager) {
        state = prev;
        toast("waiting");
        return;
      }
      err("RPC quiet. Retry stays in vapurr.");
    });
  }
  function goTx(h, preview) {
    h = normHash(h);
    stop();
    state = { page: "tx", id: h };
    setHash("#/tx/" + h);
    dropQ();
    setTab("txs");
    if (!/^0x[0-9a-f]{64}$/.test(h)) return err("bad tx hash");
    if (preview && (preview.hash || preview.from || preview.to || preview.value)) {
      try {
        paintTx({
          ok: true,
          partial: true,
          hash: preview.hash || h,
          from: preview.from,
          to: preview.to,
          value: preview.value,
          method: preview.method,
          block: preview.block,
          ts: preview.ts,
          status: preview.status == null ? 2 : preview.status,
          chain: "Robinhood Chain",
          chain_id: 4663
        });
      } catch (e) {
        skel("tx  " + short(h, 10));
      }
    } else {
      skel("tx  " + short(h, 10));
    }
    var retried = 0;
    function tick() {
      api("tx/" + encodeURIComponent(h)).then(function (d) {
        if (state.page !== "tx" || state.id !== h) return;
        if (d && d.ok) {
          try { paintTx(d); }
          catch (e) { err((e && e.message) || "tx paint failed"); }
          if ((d.loading || d.index_loading) && retried < 8) {
            retried += 1;
            poll = setTimeout(tick, 400);
          }
          return;
        }
        if (preview && (preview.hash || preview.from || preview.to)) {
          toast(uiError(d) || "waiting");
          return;
        }
        err((uiError(d) || "tx failed") + "  ·  " + short(h, 8));
      }).catch(function () {
        if (state.page !== "tx" || state.id !== h) return;
        if (!(preview && (preview.hash || preview.from))) err("RPC quiet. Retry.");
      });
    }
    tick();
  }
  function goAddr(a, tab, cursor, opts) {
    opts = opts || {};
    var stack = opts.stack;
    if (stack == null) {
      if (opts.more) stack = pushStack(state.page === "addr" && state.id === a ? state.stack : [], state.cursor);
      else if (opts.prev) stack = state.stack || [];
      else stack = [];
    }
    stop();
    state = { page: "addr", id: a, tab: tab, cursor: cursor, stack: stack };
    var extra = {};
    if (tab) extra.tab = tab;
    setHash("#/addr/" + a + qsCursor(cursor, extra));
    dropQ();
    setTab("");
    if (!opts.pager) skel("addr  " + short(a, 8));
    var retried = 0;
    function tick() {
      api(scanApi("addr", a, { tab: tab, cursor: cursorParam(cursor) })).then(function (d) {
        if (state.page !== "addr" || state.id !== a) return;
        paintAddr(d, tab);
        if (d && d.loading && retried < 8) {
          retried += 1;
          poll = setTimeout(tick, 400);
        }
      }).catch(function () {
        if (state.page !== "addr" || state.id !== a) return;
        err("RPC quiet. Retry.");
      });
    }
    tick();
  }
  function goToken(a, cursor, holders, opts) {
    opts = opts || {};
    var same = state.page === "token" && state.id === a;
    var xferStack = opts.xferStack;
    var holderStack = opts.holderStack;
    if (xferStack == null) {
      if (opts.moreXfer) xferStack = pushStack(same ? state.xferStack : [], state.cursor);
      else if (opts.prev) xferStack = state.xferStack || [];
      else xferStack = same && !opts.moreHolders ? (state.xferStack || []) : [];
    }
    if (holderStack == null) {
      if (opts.moreHolders) holderStack = pushStack(same ? state.holderStack : [], state.holders);
      else if (opts.prev) holderStack = state.holderStack || [];
      else holderStack = same && !opts.moreXfer ? (state.holderStack || []) : [];
    }
    stop();
    state = {
      page: "token",
      id: a,
      cursor: cursor,
      holders: holders,
      tab: opts.tab || (same ? state.tab : ""),
      xferStack: xferStack,
      holderStack: holderStack
    };
    var extra = {};
    if (holders) extra.holders = cursorParam(holders);
    if (state.tab) extra.tab = state.tab;
    setHash("#/token/" + a + qsCursor(cursor, extra));
    dropQ();
    setTab("tokens");
    if (!opts.pager) skel();
    var retried = 0;
    function tick() {
      api(scanApi("token", a, { cursor: cursorParam(cursor), holders: cursorParam(holders) })).then(function (d) {
        if (state.page !== "token" || state.id !== a) return;
        paintToken(d);
        if (d && d.loading && retried < 8) {
          retried += 1;
          poll = setTimeout(tick, 350);
        }
      }).catch(function () {
        if (state.page !== "token" || state.id !== a) return;
        err("index wait");
      });
    }
    tick();
  }
  function goBlocks(cursor, opts) {
    opts = opts || {};
    var stack = opts.stack;
    if (stack == null) {
      if (opts.more) stack = pushStack(state.page === "blocks" ? state.stack : [], state.cursor);
      else if (opts.prev) stack = state.stack || [];
      else stack = [];
    }
    stop();
    state = { page: "blocks", cursor: cursor, stack: stack };
    setHash("#/blocks" + qsCursor(cursor));
    dropQ();
    setTab("blocks");
    if (!opts.pager) skel();
    api(scanApi("blocks", null, cursor ? { cursor: cursorParam(cursor) } : null)).then(function (d) {
      paintList("Blocks", (d && d.blocks) || [], "block", d);
    }).catch(function () { err("index wait"); });
  }
  function goTxs(cursor, opts) {
    opts = opts || {};
    var stack = opts.stack;
    if (stack == null) {
      if (opts.more) stack = pushStack(state.page === "txs" ? state.stack : [], state.cursor);
      else if (opts.prev) stack = state.stack || [];
      else stack = [];
    }
    stop();
    state = { page: "txs", cursor: cursor, stack: stack };
    setHash("#/txs" + qsCursor(cursor));
    dropQ();
    setTab("txs");
    if (!opts.pager) skel();
    api(scanApi("txs", null, cursor ? { cursor: cursorParam(cursor) } : null)).then(function (d) {
      paintList("Transactions", (d && d.transactions) || [], "tx", d);
    }).catch(function () { err("index wait"); });
  }
  function goTokens(cursor, opts) {
    opts = opts || {};
    var stack = opts.stack;
    if (stack == null) {
      if (opts.more) stack = pushStack(state.page === "tokens" ? state.stack : [], state.cursor);
      else if (opts.prev) stack = state.stack || [];
      else stack = [];
    }
    stop();
    state = { page: "tokens", cursor: cursor, stack: stack };
    setHash("#/tokens" + qsCursor(cursor));
    dropQ();
    setTab("tokens");
    if (!opts.pager) skel();
    api(scanApi("tokens", null, cursor ? { cursor: cursorParam(cursor) } : null)).then(function (d) {
      paintList("Tokens", (d && d.tokens) || [], "token", d);
    }).catch(function () { err("index wait"); });
  }
  function usd(n) {
    n = Number(n || 0);
    if (!isFinite(n) || n <= 0) return "—";
    if (n >= 1e9) return "$" + (n / 1e9).toFixed(2) + "B";
    if (n >= 1e6) return "$" + (n / 1e6).toFixed(2) + "M";
    if (n >= 1e3) return "$" + (n / 1e3).toFixed(1) + "K";
    return "$" + n.toFixed(0);
  }
  function px(n) {
    n = Number(n || 0);
    if (!isFinite(n) || n <= 0) return "—";
    if (n >= 1000) return usd(n);
    if (n >= 1) return "$" + n.toFixed(2);
    if (n >= 0.01) return "$" + n.toFixed(4);
    return "$" + n.toPrecision(3);
  }
  function pct(n) {
    n = Number(n);
    if (!isFinite(n) || n === 0) return "—";
    return (n > 0 ? "+" : "") + n.toFixed(Math.abs(n) >= 10 ? 1 : 2) + "%";
  }
  function prettySrc(s) {
    if (s === "rhc-rpc" || s === "rhc-liq") return "Scan";
    return s || "—";
  }
  function cssVar(name, fallback) {
    try {
      var v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
      return v || fallback;
    } catch (e) {
      return fallback;
    }
  }
  function liqTheme() {
    return {
      lime: cssVar("--color-lime", "#c0f800"),
      forest: cssVar("--color-forest", "#2a3800"),
      voidc: cssVar("--color-void", "#0e0e0e"),
      snow: cssVar("--color-snow", "#f2f3f4"),
      muted: cssVar("--color-muted", "#8aa090")
    };
  }
  function isHubAddr(a) {
    a = String(a || "").toLowerCase();
    return a === "0x5fc5360d0400a0fd4f2af552add042d716f1d168"
      || a === "0x0bd7d308f8e1639fab988df18a8011f41eacad73";
  }
  function poolFocus(p) {
    var b = (p && p.base) || {};
    var q = (p && p.quote) || {};
    if (b.address && !isHubAddr(b.address)) return b.address;
    if (q.address && !isHubAddr(q.address)) return q.address;
    return b.address || q.address;
  }
  function hasGraph(d) {
    return !!(d && d.graph && d.graph.nodes && d.graph.nodes.length);
  }
  function chgNode(n) {
    var s = document.createElement("span");
    var v = Number(n);
    s.textContent = pct(v);
    if (v > 0) s.className = "chg up";
    else if (v < 0) s.className = "chg dn";
    else s.className = "meta";
    return s;
  }
  function liqAge(ts) {
    var s = Math.max(0, Math.floor(Date.now() / 1000 - Number(ts || 0)));
    if (s < 4) return "live now";
    if (s < 60) return "live  ·  " + s + "s";
    return "live  ·  " + Math.floor(s / 60) + "m";
  }
  function liqVolLabel(st) {
    st = st || {};
    var w = Number(st.vol_window_sec || 0);
    if (w >= 20 * 3600) return "liq 24h";
    if (w >= 5 * 3600) return "liq " + Math.round(w / 3600) + "h";
    if (w >= 60) return "liq " + Math.round(w / 60) + "m";
    if (Number(st.vol24_usd) > 0) return "liq vol";
    return "liq vol";
  }
  function volLabel(st) {
    st = st || {};
    var w = Number(st.vol_window_sec || 0);
    if (w >= 20 * 3600) return "24h vol";
    if (w >= 5 * 3600) return Math.round(w / 3600) + "h vol";
    if (w >= 60) return Math.round(w / 60) + "m vol";
    return "vol";
  }
  function liqTape(d) {
    var st = (d && d.stats) || d || {};
    if (st.tape === "vol" || Number(st.vol24_usd) > 0) return "vol";
    return "tvl";
  }
  function liqStatRows(d) {
    var st = d.stats || {};
    var rows = [
      ["TVL", usd(st.tvl_usd)],
      [volLabel(st), usd(st.vol24_usd)],
      ["pools", comma(st.pools || 0)],
      ["tokens", comma(st.tokens || 0)]
    ];
    if (st.txns24) rows.push(["swaps", comma(st.txns24)]);
    if (Number(st.vol1_usd) > 0) rows.splice(2, 0, ["1h vol", usd(st.vol1_usd)]);
    return rows;
  }
  function histSpark(spark) {
    var vals = (spark || []).map(function (x) { return Number((x && x.tvl_usd) || x || 0); });
    var max = 1;
    vals.forEach(function (v) { if (v > max) max = v; });
    return vals.map(function (v) { return v / max; });
  }
  function shareBars(rows, key, title) {
    var box = document.createElement("div");
    box.className = "col";
    var h = document.createElement("h2");
    h.textContent = title;
    box.appendChild(h);
    box.id = "liq-dex-col";
    var wrap = document.createElement("div");
    wrap.className = "liq-shares";
    var max = 0;
    (rows || []).forEach(function (r) {
      var n = Number(r[key] || 0);
      if (n > max) max = n;
    });
    (rows || []).slice(0, 6).forEach(function (r) {
      var row = document.createElement("div");
      row.className = "liq-share";
      var lab = document.createElement("span");
      lab.textContent = r.id || "dex";
      var bar = document.createElement("div");
      bar.className = "bar";
      var i = document.createElement("i");
      i.style.width = (max ? Math.max(4, Math.round(Number(r[key] || 0) / max * 100)) : 0) + "%";
      bar.appendChild(i);
      var val = document.createElement("b");
      val.textContent = usd(r[key]);
      row.appendChild(lab); row.appendChild(bar); row.appendChild(val);
      wrap.appendChild(row);
    });
    if (!wrap.children.length) wrap.appendChild(emptyEl("No dex split yet."));
    box.appendChild(wrap);
    return box;
  }
  function flowBar(st) {
    var box = document.createElement("div");
    box.id = "liq-flow";
    var buys = Number(st.buys24 || 0);
    var sells = Number(st.sells24 || 0);
    var tot = buys + sells;
    if (!tot) {
      box.className = "meta";
      box.textContent = "Swap tape filling from chain…";
      return box;
    }
    box.className = "flow-split";
    var left = document.createElement("div");
    left.className = "buy";
    left.textContent = comma(buys) + " buys";
    var mid = document.createElement("div");
    mid.className = "bar";
    var ib = document.createElement("i");
    ib.className = "buy";
    ib.style.width = (tot ? Math.round(buys / tot * 100) : 50) + "%";
    var is_ = document.createElement("i");
    is_.className = "sell";
    is_.style.width = (tot ? Math.round(sells / tot * 100) : 50) + "%";
    mid.appendChild(ib); mid.appendChild(is_);
    var right = document.createElement("div");
    right.className = "sell";
    right.textContent = comma(sells) + " sells";
    box.appendChild(left); box.appendChild(mid); box.appendChild(right);
    return box;
  }
  function moversTable(rows, title) {
    var col = document.createElement("div");
    col.className = "col";
    col.id = "liq-movers";
    var h = document.createElement("h2");
    h.textContent = title || "By TVL";
    col.appendChild(h);
    if (!rows || !rows.length) {
      col.appendChild(emptyEl("Waiting on prints."));
      return col;
    }
    var t = document.createElement("table");
    t.className = "grid";
    t.innerHTML = "<thead><tr><th>Token</th><th>Price</th><th>24h</th><th>Vol</th></tr></thead>";
    var tb = document.createElement("tbody");
    rows.slice(0, 8).forEach(function (tok) {
      var tr = document.createElement("tr");
      var td0 = document.createElement("td");
      td0.textContent = tok.symbol || short(tok.address, 4);
      var td1 = document.createElement("td");
      td1.textContent = px(tok.price_usd);
      var td2 = document.createElement("td");
      td2.appendChild(chgNode(tok.change24));
      var td3 = document.createElement("td");
      td3.className = "meta";
      td3.textContent = usd(tok.vol24_usd);
      tr.appendChild(td0); tr.appendChild(td1); tr.appendChild(td2); tr.appendChild(td3);
      armRow(tr, function () { if (tok.address) goToken(tok.address); });
      tb.appendChild(tr);
    });
    t.appendChild(tb);
    col.appendChild(t);
    return col;
  }
  function setLiqLive(d) {
    var el = document.getElementById("liq-live");
    if (el) el.textContent = liqAge(d.ts);
    setLive(true, "liq  ·  " + comma((d.stats && d.stats.pools) || 0) + " pools", d.ts);
  }
  var liqSvg = null;
  var liqData = null;
  var liqFilter = { q: "", dex: "", node: "" };
  function killLiqNet() {
    liqSvg = null;
  }
  function onLiqResize() {
    if (!liqSvg) return;
    try {
      liqSvg.removeAttribute("style");
    } catch (e) {}
  }
  function goGas() {
    stop();
    state = { page: "gas" };
    setHash("#/gas");
    dropQ();
    setTab("gas");
    skel();
    api("gas").then(paintGas).catch(function () { err("RPC quiet. Retry."); });
  }
  function paintGas(d) {
    clearBusy();
    if (!d || !d.ok) return err((d && d.error) || "RPC quiet. Retry.");
    var rows = [
      ["block", comma(d.block)],
      ["base", d.base_fee || "—"],
      ["price", d.gwei || "—"],
      ["gas used", comma(d.gas_used || 0)],
      ["heat", d.heat || "—"]
    ];
    var frac = d.relative != null ? d.relative : (d.spark && d.spark[0]) || 0;
    var capNote = "Suggested lanes from base fee and eth_gasPrice. Meter is this block vs the last 12.";
    function gasPageCaption() {
      var cap = gasCaption(d);
      cap.querySelector(".meta").textContent = capNote;
      return cap;
    }
    var existing = view.getAttribute("data-scan") === "gas" && view.querySelector(".stats") && view.querySelector(".meter-wrap");
    if (existing) {
      patchStatGrid(rows);
      var meter = view.querySelector(".meter-wrap");
      if (meter) meter.replaceWith(meterEl(frac, d.heat, gasPageCaption()));
      if (d.spark && d.spark.length) {
        if (view.querySelector(".spark")) patchSpark(d.spark);
        else view.appendChild(sparkEl(d.spark));
      }
      return;
    }
    view.innerHTML = "";
    view.setAttribute("data-scan", "gas");
    var h = document.createElement("h1");
    h.textContent = "Gas";
    view.appendChild(h);
    view.appendChild(statGrid(rows));
    view.appendChild(meterEl(frac, d.heat, gasPageCaption()));
    if (d.spark && d.spark.length) view.appendChild(sparkEl(d.spark));
  }
  function goLiq() {
    stop();
    state = { page: "liq" };
    setHash("#/liq");
    dropQ();
    setTab("liq");
    paintLiqWarm();
    var tries = 0;
    var liveOk = false;
    function tick() {
      api("liq").then(function (d) {
        if (state.page !== "liq") return;
        if (d && d.loading && !hasGraph(d)) {
          tries += 1;
          setLive(false, "loading market map");
          if (tries === 1) paintLiqWarm();
          return;
        }
        paintLiq(d);
        if (d && d.ok && !liveOk) {
          liveOk = true;
          if (poll) { clearInterval(poll); poll = 0; }
          poll = setInterval(tick, 12000);
        }
      }).catch(function () {
        if (state.page !== "liq") return;
        tries += 1;
        if (tries > 8) err("liquidity wait");
      });
    }
    tick();
    poll = setInterval(tick, 2500);
  }
  function paintLiqWarm() {
    killLiqNet();
    clearBusy();
    view.innerHTML = "";
    view.setAttribute("aria-busy", "true");
    var h = document.createElement("h1");
    h.textContent = "Liquidity";
    view.appendChild(h);
    var cap = document.createElement("div");
    cap.className = "meta";
    cap.textContent = "Tokens are nodes. Pools are edges. Pulling the Robinhood book…";
    view.appendChild(cap);
    var stage = document.createElement("div");
    stage.className = "liq-stage warm";
    stage.appendChild(emptyEl("Pulling Robinhood pools…"));
    view.appendChild(stage);
  }
  function paintLiqEmpty(msg) {
    killLiqNet();
    clearBusy();
    view.innerHTML = "";
    var h = document.createElement("h1");
    h.textContent = "Liquidity";
    view.appendChild(h);
    view.appendChild(emptyEl(msg || "No pools on the market map yet."));
    var b = document.createElement("button");
    b.className = "btn-ghost";
    b.type = "button";
    b.textContent = "Retry";
    b.onclick = goLiq;
    view.appendChild(b);
  }
  function poolMatches(p) {
    var q = (liqFilter.q || "").toLowerCase();
    var dex = liqFilter.dex || "";
    var node = (liqFilter.node || "").toLowerCase();
    if (dex && String(p.dex || "") !== dex) return false;
    if (node) {
      var ba = ((p.base || {}).address || "").toLowerCase();
      var qa = ((p.quote || {}).address || "").toLowerCase();
      if (ba !== node && qa !== node) return false;
    }
    if (!q) return true;
    var blob = ((p.name || "") + " " + (p.dex || "") + " " + ((p.base || {}).symbol || "") + " " + ((p.quote || {}).symbol || "")).toLowerCase();
    return blob.indexOf(q) >= 0;
  }
  function tokMatches(t) {
    var q = (liqFilter.q || "").toLowerCase();
    var node = (liqFilter.node || "").toLowerCase();
    if (node && String(t.address || "").toLowerCase() !== node) {
      var linked = (liqData.pools || []).some(function (p) {
        return poolMatches(p) && (
          ((p.base || {}).address || "").toLowerCase() === String(t.address || "").toLowerCase()
          || ((p.quote || {}).address || "").toLowerCase() === String(t.address || "").toLowerCase()
        );
      });
      if (!linked) return false;
    }
    if (!q) return true;
    return ((t.symbol || "") + " " + (t.address || "")).toLowerCase().indexOf(q) >= 0;
  }
  function poolRow(p) {
    var tr = document.createElement("tr");
    var td0 = document.createElement("td");
    var base = p.base || {};
    var quote = p.quote || {};
    if (base.address) td0.appendChild(hashWrap(base.address, "token"));
    else td0.appendChild(document.createTextNode(base.symbol || p.name || "pool"));
    td0.appendChild(document.createTextNode(" / "));
    if (quote.address) td0.appendChild(hashWrap(quote.address, "token"));
    else td0.appendChild(document.createTextNode(quote.symbol || ""));
    if (p.fee) {
      var fee = document.createElement("div");
      fee.className = "meta";
      fee.textContent = p.fee;
      td0.appendChild(fee);
    }
    var td1 = document.createElement("td");
    td1.className = "meta";
    td1.textContent = p.dex || "";
    var td2 = document.createElement("td");
    td2.textContent = usd(p.reserve_usd);
    var td3 = document.createElement("td");
    td3.className = "meta";
    td3.textContent = usd(p.vol24_usd);
    var td4 = document.createElement("td");
    td4.appendChild(chgNode(p.change24));
    tr.appendChild(td0); tr.appendChild(td1); tr.appendChild(td2); tr.appendChild(td3); tr.appendChild(td4);
    var focus = poolFocus(p);
    armRow(tr, function () { if (focus) goToken(focus); });
    return tr;
  }
  function poolTable(pools, limit) {
    var t = document.createElement("table");
    t.className = "grid";
    t.innerHTML = "<thead><tr><th>Pool</th><th>Dex</th><th>TVL</th><th>24h</th><th></th></tr></thead>";
    var body = document.createElement("tbody");
    (pools || []).slice(0, limit || 24).forEach(function (p) {
      body.appendChild(poolRow(p));
    });
    t.appendChild(body);
    if (!body.children.length) {
      var wrap = document.createElement("div");
      wrap.appendChild(emptyEl("No pools in this filter."));
      return wrap;
    }
    return t;
  }
  function dimLiqGraph() {
    var keep = {};
    var node = (liqFilter.node || "").toLowerCase();
    (liqData && liqData.pools || []).forEach(function (p) {
      if (!poolMatches(p)) return;
      var ba = ((p.base || {}).address || "").toLowerCase();
      var qa = ((p.quote || {}).address || "").toLowerCase();
      if (ba) keep[ba] = true;
      if (qa) keep[qa] = true;
    });
    function hiddenId(id) {
      id = String(id || "").toLowerCase();
      if (node && id !== node && !keep[id]) return true;
      if (!node && (liqFilter.q || liqFilter.dex) && !keep[id]) return true;
      return false;
    }
    if (!liqSvg) return;
    Array.prototype.forEach.call(liqSvg.querySelectorAll(".liq-n"), function (el) {
      el.classList.toggle("is-dim", hiddenId(el.getAttribute("data-id")));
    });
    Array.prototype.forEach.call(liqSvg.querySelectorAll(".liq-e"), function (el) {
      var a = (el.getAttribute("data-from") || "").toLowerCase();
      var b = (el.getAttribute("data-to") || "").toLowerCase();
      var hide = false;
      if (node && a !== node && b !== node) hide = true;
      if (!node && (liqFilter.q || liqFilter.dex) && (!keep[a] || !keep[b])) hide = true;
      el.classList.toggle("is-dim", hide);
    });
  }
  var liqRefill = null;
  function paintLiq(d) {
    if (!d || d.error === "unknown") {
      return err("Liquidity map isn't in this build.");
    }
    if (!d.ok && !hasGraph(d)) {
      if (d.loading) return paintLiqWarm();
      if (d.error === "empty") return paintLiqEmpty("No pools on Robinhood Chain yet.");
      return paintLiqEmpty((d && d.error) || "Market map offline. Retry.");
    }
    if (view.getAttribute("data-scan") === "liq" && view.querySelector("#liq-graph")) {
      liqData = d;
      setLiqLive(d);
      patchStatGrid(liqStatRows(d));
      var age = document.getElementById("liq-live");
      if (age) age.textContent = liqAge(d.ts);
      var sparkHold = document.getElementById("liq-spark");
      var hs = histSpark(d.spark);
      if (sparkHold) {
        sparkHold.innerHTML = "";
        if (hs.length) {
          var sp = sparkEl(hs.slice().reverse());
          sp.title = "TVL prints";
          sparkHold.appendChild(sp);
        }
      }
      var flow = document.getElementById("liq-flow");
      if (flow) flow.replaceWith(flowBar(d.stats || {}));
      var movers = document.getElementById("liq-movers");
      if (movers) movers.replaceWith(moversTable(d.movers || [], liqTape(d) === "vol" ? "By volume" : "By TVL"));
      var dexcol = document.getElementById("liq-dex-col");
      if (dexcol) dexcol.replaceWith(shareBars(d.dexes || [], liqTape(d) === "vol" ? "vol24_usd" : "tvl_usd", liqTape(d) === "vol" ? "Dex vol" : "Dex TVL"));
      if (typeof liqRefill === "function") liqRefill();
      return;
    }
    killLiqNet();
    clearBusy();
    view.innerHTML = "";
    view.setAttribute("data-scan", "liq");
    liqData = d;
    liqFilter = { q: "", dex: "", node: "" };
    var head = document.createElement("div");
    head.className = "liq-head";
    var h = document.createElement("h1");
    h.textContent = "Liquidity";
    head.appendChild(h);
    var src = document.createElement("div");
    src.className = "liq-live";
    var pip = document.createElement("span");
    pip.className = "pip";
    var age = document.createElement("span");
    age.id = "liq-live";
    age.textContent = liqAge(d.ts);
    src.appendChild(pip);
    src.appendChild(age);
    head.appendChild(src);
    view.appendChild(head);
    view.appendChild(statGrid(liqStatRows(d)));
    setLiqLive(d);
    var sparkHold = document.createElement("div");
    sparkHold.id = "liq-spark";
    var hs = histSpark(d.spark);
    if (hs.length) {
      var sp = sparkEl(hs.slice().reverse());
      sp.title = "TVL prints";
      sparkHold.appendChild(sp);
    }
    view.appendChild(sparkHold);
    view.appendChild(flowBar(d.stats || {}));
    var analytics = document.createElement("div");
    analytics.className = "liq-cols";
    var dexcol = shareBars(d.dexes || [], liqTape(d) === "vol" ? "vol24_usd" : "tvl_usd", liqTape(d) === "vol" ? "Dex vol" : "Dex TVL");
    dexcol.id = "liq-dex-col";
    analytics.appendChild(dexcol);
    analytics.appendChild(moversTable(d.movers || [], liqTape(d) === "vol" ? "By volume" : "By TVL"));
    view.appendChild(analytics);
    var chips = document.createElement("div");
    chips.className = "chips";
    function dexBtn(id, label, on) {
      var b = document.createElement("button");
      b.type = "button";
      b.textContent = label;
      if (on) b.classList.add("on");
      b.onclick = function () {
        liqFilter.dex = id;
        Array.prototype.forEach.call(chips.querySelectorAll("button"), function (x) {
          x.classList.toggle("on", x === b);
        });
        refill();
        dimLiqGraph();
      };
      return b;
    }
    chips.appendChild(dexBtn("", "All", true));
    (d.dexes || []).slice(0, 8).forEach(function (dx) {
      chips.appendChild(dexBtn(dx.id, (dx.id || "dex") + " · " + comma(dx.pools || 0), false));
    });
    view.appendChild(chips);
    var filt = document.createElement("div");
    filt.className = "liq-filter";
    var field = document.createElement("input");
    field.className = "field";
    field.placeholder = "Filter pair, dex, token";
    field.addEventListener("input", function () {
      liqFilter.q = field.value.trim();
      refill();
      dimLiqGraph();
    });
    filt.appendChild(field);
    var clear = document.createElement("button");
    clear.className = "btn-ghost";
    clear.type = "button";
    clear.textContent = "Clear";
    clear.onclick = function () {
      liqFilter = { q: "", dex: "", node: "" };
      field.value = "";
      Array.prototype.forEach.call(chips.querySelectorAll("button"), function (x, i) {
        x.classList.toggle("on", i === 0);
      });
      if (card) card.classList.remove("on");
      refill();
      dimLiqGraph();
    };
    filt.appendChild(clear);
    view.appendChild(filt);
    var stage = document.createElement("div");
    stage.className = "liq-stage";
    var overlay = document.createElement("div");
    overlay.className = "liq-overlay";
    var legend = document.createElement("div");
    legend.className = "liq-legend";
    legend.innerHTML = "<span><i class='hub'></i> USDG / WETH</span><span><i class='token'></i> Tokens</span><span><i class='edge'></i> Pools</span>";
    var tools = document.createElement("div");
    tools.className = "liq-tools";
    var fit = document.createElement("button");
    fit.className = "btn-ghost";
    fit.type = "button";
    fit.textContent = "Fit";
    fit.onclick = function () { onLiqResize(); };
    tools.appendChild(fit);
    overlay.appendChild(legend);
    overlay.appendChild(tools);
    var canvas = document.createElement("div");
    canvas.id = "liq-graph";
    var card = document.createElement("div");
    card.className = "liq-card";
    stage.appendChild(overlay);
    stage.appendChild(canvas);
    stage.appendChild(card);
    view.appendChild(stage);
    var note = document.createElement("p");
    note.className = "meta";
    note.style.margin = "0 0 14px";
    note.textContent = "Robinhood RPC. Nodes are tokens, edges are pools. USDG and WETH hub in lime. Click to pin, double-click opens the token.";
    view.appendChild(note);
    var cols = document.createElement("div");
    cols.className = "liq-cols";
    var poolCol = document.createElement("div");
    poolCol.className = "col";
    var ph = document.createElement("h2");
    ph.textContent = "Pools";
    poolCol.appendChild(ph);
    var poolHold = document.createElement("div");
    poolCol.appendChild(poolHold);
    var tokCol = document.createElement("div");
    tokCol.className = "col";
    var th = document.createElement("h2");
    th.textContent = "Tokens";
    tokCol.appendChild(th);
    var tokHold = document.createElement("div");
    tokCol.appendChild(tokHold);
    cols.appendChild(poolCol);
    cols.appendChild(tokCol);
    view.appendChild(cols);
    function showCard(id) {
      var tok = ((liqData && liqData.tokens) || []).filter(function (t) {
        return String(t.address || "").toLowerCase() === String(id || "").toLowerCase();
      })[0];
      if (!tok || !tok.address) { card.classList.remove("on"); return; }
      card.innerHTML = "";
      var sym = document.createElement("div");
      sym.className = "sym";
      sym.textContent = tok.symbol || short(tok.address, 6);
      card.appendChild(sym);
      var row = document.createElement("div");
      row.className = "row";
      row.textContent = px(tok.price_usd) + "  ·  TVL " + usd(tok.tvl_usd) + "  ·  24h " + usd(tok.vol24_usd);
      card.appendChild(row);
      var open = document.createElement("button");
      open.className = "btn-ghost";
      open.type = "button";
      open.style.marginTop = "8px";
      open.textContent = "Open token";
      open.onclick = function (e) {
        e.stopPropagation();
        goToken(tok.address);
      };
      card.appendChild(open);
      card.classList.add("on");
    }
    function refill() {
      poolHold.innerHTML = "";
      tokHold.innerHTML = "";
      var snap = liqData || d;
      var pools = (snap.pools || []).filter(poolMatches).slice(0, 32);
      if (!pools.length) poolHold.appendChild(emptyEl("No pools in this filter."));
      else poolHold.appendChild(poolTable(pools, 32));
      var toks = (snap.tokens || []).filter(tokMatches).slice(0, 24);
      if (!toks.length) {
        tokHold.appendChild(emptyEl("No tokens in this filter."));
      } else {
        var tt = document.createElement("table");
        tt.className = "grid";
        tt.innerHTML = "<thead><tr><th>Token</th><th>Price</th><th>TVL</th></tr></thead>";
        var tb = document.createElement("tbody");
        toks.forEach(function (tok) {
          var tr = document.createElement("tr");
          var td0 = document.createElement("td");
          td0.appendChild(hashWrap(tok.address, "token"));
          var lab = document.createElement("div");
          lab.className = "meta";
          lab.textContent = tok.symbol || "token";
          if (tok.hub) lab.textContent += "  ·  hub";
          td0.appendChild(lab);
          var td1 = document.createElement("td");
          td1.textContent = px(tok.price_usd);
          var td2 = document.createElement("td");
          td2.textContent = usd(tok.tvl_usd);
          tr.appendChild(td0); tr.appendChild(td1); tr.appendChild(td2);
          armRow(tr, function () { goToken(tok.address); });
          tb.appendChild(tr);
        });
        tt.appendChild(tb);
        tokHold.appendChild(tt);
      }
    }
    refill();
    liqRefill = refill;
    requestAnimationFrame(function () {
      drawLiqGraph(canvas, d.graph || { nodes: [], edges: [] }, {
        onNode: function (id) {
          id = String(id || "");
          var cur = String(liqFilter.node || "").toLowerCase();
          if (id && cur === id.toLowerCase()) {
            liqFilter.node = "";
            card.classList.remove("on");
          } else {
            liqFilter.node = id;
            showCard(id);
          }
          try { refill(); } catch (err) {}
          try { dimLiqGraph(); } catch (err) {}
        }
      });
    });
  }
  function drawLiqGraph(container, g, hooks) {
    hooks = hooks || {};
    container.innerHTML = "";
    liqSvg = null;
    var nodes = ((g && g.nodes) || []).slice();
    var edges = ((g && g.edges) || []).slice();
    nodes.sort(function (a, b) {
      var ah = a.hub || isHubAddr(a.id) ? 1 : 0;
      var bh = b.hub || isHubAddr(b.id) ? 1 : 0;
      if (bh !== ah) return bh - ah;
      return Number(b.tvl || 0) - Number(a.tvl || 0);
    });
    if (nodes.length > 48) nodes = nodes.slice(0, 48);
    var keep = {};
    nodes.forEach(function (n) { keep[String(n.id || "").toLowerCase()] = true; });
    edges = edges.filter(function (e) {
      return keep[String(e.from || "").toLowerCase()] && keep[String(e.to || "").toLowerCase()];
    });
    edges.sort(function (a, b) { return Number(b.reserve || 0) - Number(a.reserve || 0); });
    if (edges.length > 72) edges = edges.slice(0, 72);
    if (!nodes.length) {
      container.appendChild(emptyEl("No pools yet."));
      return;
    }
    var t = liqTheme();
    var maxTvl = 1;
    nodes.forEach(function (n) {
      var v = Number(n.tvl || 0);
      if (v > maxTvl) maxTvl = v;
    });
    function isHubNode(n) {
      return isHubAddr(n.id) || n.hub || n.kind === "eth";
    }
    function bubbleR(tvl) {
      var x = Math.sqrt(Math.max(tvl, 0) / maxTvl);
      return 3.2 + x * 46;
    }
    function nodeFill(n) {
      if (isHubNode(n)) return t.lime;
      return t.muted;
    }
    var w = 960;
    var h = 560;
    var svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 " + w + " " + h);
    svg.setAttribute("preserveAspectRatio", "xMidYMid meet");
    svg.setAttribute("width", "100%");
    svg.setAttribute("height", "100%");
    svg.setAttribute("aria-label", "Robinhood Chain liquidity map");
    var pos = {};
    var hubs = [];
    var rest = [];
    nodes.forEach(function (n) {
      if (isHubNode(n)) hubs.push(n);
      else rest.push(n);
    });
    rest.sort(function (a, b) { return Number(b.tvl || 0) - Number(a.tvl || 0); });
    var cx = w / 2;
    var cy = h / 2;
    hubs.forEach(function (n, i) {
      var a = -Math.PI / 2 + (i * (Math.PI * 2 / Math.max(hubs.length, 1)));
      pos[n.id] = { x: cx + Math.cos(a) * 70, y: cy + Math.sin(a) * 52, n: n };
    });
    var rim = Math.min(cx, cy) - 36;
    rest.forEach(function (n, i) {
      var golden = Math.PI * (3 - Math.sqrt(5));
      var a = i * golden;
      var frac = rest.length < 2 ? 0 : i / (rest.length - 1);
      var wealth = Math.sqrt(Number(n.tvl || 0) / maxTvl);
      var r = 88 + frac * (rim - 88) * 0.92 + (1 - wealth) * 28;
      r = Math.min(r, rim);
      pos[n.id] = { x: cx + Math.cos(a) * r, y: cy + Math.sin(a) * r * 0.78, n: n };
    });
    edges.forEach(function (e) {
      var a = pos[e.from];
      var b = pos[e.to];
      if (!a || !b) return;
      var res = Number(e.reserve || 0);
      var line = document.createElementNS("http://www.w3.org/2000/svg", "line");
      line.setAttribute("class", "liq-e");
      line.setAttribute("x1", a.x.toFixed(1));
      line.setAttribute("y1", a.y.toFixed(1));
      line.setAttribute("x2", b.x.toFixed(1));
      line.setAttribute("y2", b.y.toFixed(1));
      line.setAttribute("stroke", t.lime);
      var thick = 0.6 + Math.min(7, Math.sqrt(Math.max(res, 0) / Math.max(maxTvl, 1)) * 8);
      line.setAttribute("stroke-width", String(thick));
      line.setAttribute("stroke-opacity", res >= 10000 ? "0.42" : "0.18");
      line.setAttribute("data-from", e.from || "");
      line.setAttribute("data-to", e.to || "");
      var et = document.createElementNS("http://www.w3.org/2000/svg", "title");
      et.textContent = (e.name || "") + " · " + usd(e.reserve) + " · 24h " + usd(e.vol24);
      line.appendChild(et);
      svg.appendChild(line);
    });
    nodes.forEach(function (n) {
      var p = pos[n.id];
      if (!p) return;
      var tvl = Number(n.tvl || 0);
      var r = bubbleR(tvl);
      var hub = isHubNode(n);
      var fill = nodeFill(n);
      var gEl = document.createElementNS("http://www.w3.org/2000/svg", "g");
      gEl.setAttribute("class", "liq-n" + (hub ? " hub" : "") + (n.kind === "stable" ? " stable" : ""));
      gEl.setAttribute("data-id", n.id);
      gEl.style.cursor = "pointer";
      if (hub) {
        var aura = document.createElementNS("http://www.w3.org/2000/svg", "circle");
        aura.setAttribute("class", "liq-aura");
        aura.setAttribute("cx", p.x.toFixed(1));
        aura.setAttribute("cy", p.y.toFixed(1));
        aura.setAttribute("r", (r * 1.85).toFixed(1));
        aura.setAttribute("fill", "rgba(192,248,0,0.18)");
        aura.setAttribute("pointer-events", "none");
        gEl.appendChild(aura);
      }
      var c = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      c.setAttribute("class", "liq-core");
      c.setAttribute("cx", p.x.toFixed(1));
      c.setAttribute("cy", p.y.toFixed(1));
      c.setAttribute("r", r.toFixed(1));
      c.setAttribute("fill", fill);
      c.setAttribute("stroke", hub ? t.lime : t.forest);
      c.setAttribute("stroke-width", hub ? "2" : "1.2");
      gEl.appendChild(c);
      var labeled = hub || tvl >= maxTvl * 0.04 || tvl >= 80000;
      if (labeled) {
        var lab = document.createElementNS("http://www.w3.org/2000/svg", "text");
        lab.setAttribute("x", p.x.toFixed(1));
        lab.setAttribute("y", (p.y + r + 12).toFixed(1));
        lab.setAttribute("text-anchor", "middle");
        lab.setAttribute("fill", t.snow);
        lab.setAttribute("font-size", r > 22 ? "12" : "10");
        lab.setAttribute("font-family", "Sora, sans-serif");
        lab.setAttribute("font-weight", hub ? "600" : "400");
        lab.textContent = n.label || n.symbol || "";
        gEl.appendChild(lab);
      }
      var title = document.createElementNS("http://www.w3.org/2000/svg", "title");
      title.textContent = (n.label || "") + " · " + px(n.price) + " · " + usd(tvl);
      gEl.appendChild(title);
      function eatClick(ev) {
        if (ev.preventDefault) ev.preventDefault();
        if (ev.stopPropagation) ev.stopPropagation();
        if (ev.stopImmediatePropagation) ev.stopImmediatePropagation();
      }
      gEl.addEventListener("click", function (ev) {
        eatClick(ev);
        try {
          if (hooks.onNode) hooks.onNode(n.id);
        } catch (err) {}
      });
      gEl.addEventListener("dblclick", function (ev) {
        eatClick(ev);
        var addr = String(n.id || n.address || "");
        if (!/^0x[0-9a-fA-F]{40}$/.test(addr)) return;
        try { goToken(addr); } catch (err) {}
      });
      svg.appendChild(gEl);
    });
    container.appendChild(svg);
    liqSvg = svg;
    dimLiqGraph();
  }
  function search(q) {
    stop();
    q = String(q || "").trim();
    if (!q) return goHead();
    var hood = q.replace(/^@/, "");
    if (/\.hood$/i.test(hood)) {
      state = { page: "search", q: q };
      setHash("#/search/" + encodeURIComponent(q));
      dropQ();
      skel();
      fetch("/zzzmail/api/pns/resolve/" + encodeURIComponent(hood)).then(function (r) { return r.json(); }).then(function (v) {
        var rec = (v && (v.lookup || v.record)) || {};
        var addr = rec.addr || rec.owner;
        if (addr) return goAddr(addr);
        err("unknown PNS name");
      }).catch(function () { err("PNS quiet. Retry."); });
      return;
    }
    state = { page: "search", q: q };
    setHash("#/search/" + encodeURIComponent(q));
    dropQ();
    skel();
    api("search/" + encodeURIComponent(q)).then(function (d) {
      dropQ();
      if (!d || !d.ok) return err(uiError(d) || "not found");
      if (d.kind === "block") goBlock(d.id);
      else if (d.kind === "tx") goTx(d.id);
      else if (d.kind === "addr") goAddr(d.id);
      else if (d.kind === "token") goToken(d.id);
      else if (d.kind === "search") { state = { page: "search", q: q }; setHash("#/search/" + encodeURIComponent(q)); setTab(""); paintSearch(d.items || [], q); }
      else goHead();
    }).catch(function () { err("RPC quiet. Retry."); });
  }
  function stop() {
    if (poll) {
      clearInterval(poll);
      clearTimeout(poll);
      poll = 0;
    }
    killLiqNet();
  }
  function ensureLive() {
    if (liveTick) return;
    function ping() {
      if (state.page === "head") return;
      api("head").then(function (d) {
        if (!d || d.loading) {
          if (!pip || !pip.classList.contains("on")) setLive(false, "RPC quiet");
          return;
        }
        if (d.ok && !d.stale) setLive(true, "live  ·  blk " + comma(d.block), d.ts);
        else if (d.block != null) setLive(false, "stale  ·  blk " + comma(d.block), d.ts);
        else setLive(false, d.error === "rpc wait" ? "waiting" : (d.error || "waiting"));
        if (state.page === "gas" && d.gas) paintGas(d.gas);
      }).catch(function () {
        setLive(false, "RPC quiet");
      });
    }
    ping();
    liveTick = setInterval(ping, 4000);
  }
  function goNamedTab(tab) {
    tab = String(tab || "").toLowerCase();
    if (tab === "gas" || tab === "gwei") return goGas();
    if (tab === "blocks") return goBlocks();
    if (tab === "txs" || tab === "transactions") return goTxs();
    if (tab === "tokens") return goTokens();
    if (tab === "liq" || tab === "liquidity") return goLiq();
    if (tab === "head" || tab === "overview") return goHead();
    return goHead();
  }
  function route() {
    var searchParams = new URLSearchParams(location.search);
    var q = searchParams.get("q");
    var tabQ = searchParams.get("tab");
    var rawHash = (location.hash || "").replace(/^#\/?/, "");
    if (rawHash) dropQ();
    if (q && !rawHash) { input.value = q; search(q); return; }
    if (!rawHash && tabQ) return goNamedTab(tabQ);
    var raw = (location.hash || "").replace(/^#\/?/, "");
    var qi = raw.indexOf("?");
    var path = qi >= 0 ? raw.slice(0, qi) : raw;
    var params = new URLSearchParams(qi >= 0 ? raw.slice(qi + 1) : "");
    var cursor = params.get("cursor");
    var page = params.get("page");
    var tab = params.get("tab");
    if (path.indexOf("search/") === 0) {
      var sq = path.slice(7);
      try { sq = decodeURIComponent(sq); } catch (e) {}
      input.value = sq;
      return search(sq);
    }
    if (path === "search") {
      var sq2 = params.get("q") || "";
      input.value = sq2;
      return search(sq2);
    }
    if (path.indexOf("block/") === 0) return goBlock(path.slice(6), page);
    if (path.indexOf("tx/") === 0) {
      var th = path.slice(3);
      try { th = decodeURIComponent(th); } catch (e) {}
      return goTx(th);
    }
    if (path.indexOf("addr/") === 0) {
      var aa = path.slice(5);
      try { aa = decodeURIComponent(aa); } catch (e) {}
      return goAddr(aa, tab, cursor);
    }
    if (path.indexOf("token/") === 0) {
      var ta = path.slice(6);
      try { ta = decodeURIComponent(ta); } catch (e) {}
      return goToken(ta, cursor, params.get("holders"), { tab: params.get("tab") });
    }
    if (path === "blocks") return goBlocks(cursor);
    if (path === "txs") return goTxs(cursor);
    if (path === "tokens") return goTokens(cursor);
    if (path === "liq" || path === "liquidity") return goLiq();
    if (path === "gas") return goGas();
    if (/^liq-/i.test(path)) {
      if (state.page === "liq") return;
      return goLiq();
    }
    goHead();
  }

  var sug = document.getElementById("suggest");
  var sugTimer = 0;
  function hideSuggest() {
    sugIx = -1;
    if (sug) { sug.classList.remove("on"); sug.hidden = true; sug.innerHTML = ""; }
  }
  function moveSuggest(dir) {
    if (!sug) return;
    var buttons = sug.querySelectorAll("button");
    if (!buttons.length) return;
    sugIx = (sugIx + dir + buttons.length) % buttons.length;
    Array.prototype.forEach.call(buttons, function (b, i) {
      b.classList.toggle("on", i === sugIx);
    });
  }
  function showSuggest(items) {
    if (!sug) return;
    sugIx = -1;
    sug.innerHTML = "";
    if (!items || !items.length) { hideSuggest(); return; }
    items.slice(0, 6).forEach(function (it) {
      var b = document.createElement("button");
      b.type = "button";
      var label = it.label || it.symbol || it.name || it.address || it.tx || it.id || "";
      b.textContent = (it.kind ? it.kind + "  ·  " : "") + label;
      b.onclick = function () {
        hideSuggest();
        if (it.kind === "block") goBlock(it.id || it.block);
        else if (it.kind === "addr" || it.kind === "address") goAddr(it.id || it.address);
        else if (it.kind === "hash" || it.kind === "tx") search(it.id || it.tx || label);
        else if (it.kind && String(it.kind).indexOf("token") >= 0 && (it.address || it.id)) goToken(it.address || it.id);
        else search(label);
      };
      sug.appendChild(b);
    });
    sug.hidden = false;
    sug.classList.add("on");
  }
  document.getElementById("go").addEventListener("submit", function (e) {
    e.preventDefault();
    var hit = sug && sug.classList.contains("on") && sug.querySelector("button.on");
    if (hit) {
      hit.click();
      return;
    }
    hideSuggest();
    search(input.value);
  });
  input.addEventListener("input", function () {
    var q = input.value.trim();
    clearTimeout(sugTimer);
    if (q.length < 2) { hideSuggest(); return; }
    sugTimer = setTimeout(function () {
      api("suggest/" + encodeURIComponent(q)).then(function (d) {
        showSuggest((d && d.items) || []);
      }).catch(function () {});
    }, 160);
  });
  input.addEventListener("blur", function () {
    setTimeout(hideSuggest, 180);
  });
  document.getElementById("tabs").addEventListener("click", function (e) {
    var b = e.target.closest("button");
    if (!b) return;
    var tab = b.getAttribute("data-tab");
    if (tab === "head") goHead();
    else if (tab === "blocks") goBlocks();
    else if (tab === "txs") goTxs();
    else if (tab === "tokens") goTokens();
    else if (tab === "gas") goGas();
    else if (tab === "liq") goLiq();
  });
  document.getElementById("tabs").addEventListener("keydown", function (e) {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    var buttons = Array.prototype.slice.call(this.querySelectorAll("button[data-tab]"));
    if (!buttons.length) return;
    var i = buttons.indexOf(document.activeElement);
    if (i < 0) i = buttons.findIndex(function (b) { return b.classList.contains("on"); });
    if (i < 0) i = 0;
    i = (i + (e.key === "ArrowRight" ? 1 : -1) + buttons.length) % buttons.length;
    e.preventDefault();
    e.stopPropagation();
    buttons[i].focus();
    buttons[i].click();
  });
  document.addEventListener("keydown", function (e) {
    var typing = isField(e.target) || isField(document.activeElement);
    if (e.key === "/" && !typing && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      input.focus();
      input.select();
      return;
    }
    if (sug && sug.classList.contains("on") && document.activeElement === input) {
      if (e.key === "ArrowDown") { e.preventDefault(); moveSuggest(1); return; }
      if (e.key === "ArrowUp") { e.preventDefault(); moveSuggest(-1); return; }
      if (e.key === "Enter") {
        var hit = sug.querySelector("button.on");
        if (hit) { e.preventDefault(); hit.click(); return; }
      }
      if (e.key === "Escape") { e.preventDefault(); hideSuggest(); return; }
    }
    if (e.key === "Escape") {
      hideSuggest();
      if (isField(document.activeElement)) { document.activeElement.blur(); return; }
      if (state.page !== "head") goHead();
    }
    if (typing) return;
    if (e.key === "?" && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      toast("/ search   m more   [ prev   esc overview");
      return;
    }
    if (e.key === "m" || e.key === "]") {
      var moreBtn = document.querySelector(".pager [data-more]");
      if (moreBtn && !moreBtn.disabled) {
        e.preventDefault();
        moreBtn.click();
        return;
      }
    }
    if (e.key === "[") {
      var prevBtn = document.querySelector(".pager [data-prev]");
      if (prevBtn && !prevBtn.disabled) {
        e.preventDefault();
        prevBtn.click();
        return;
      }
    }
    var tabsEl = document.getElementById("tabs");
    var inChromeTabs = tabsEl && tabsEl.contains(document.activeElement);
    if (state.page === "block" && !inChromeTabs) {
      var n = Number(state.id);
      if (e.key === "n" || e.key === "ArrowRight") {
        e.preventDefault();
        if (!isFinite(n)) return;
        if (state.head != null && n >= Number(state.head)) { toast("at head"); return; }
        goBlock(n + 1, 1, { pager: true });
      }
      if ((e.key === "p" || e.key === "ArrowLeft") && isFinite(n) && n > 0) {
        e.preventDefault();
        goBlock(n - 1, 1, { pager: true });
      }
    }
  });
  window.addEventListener("hashchange", function () {
    if (ignoreHash) { ignoreHash = false; return; }
    route();
  });
  ensureLive();
  loadPns();
  if (!clock) clock = setInterval(tickAges, 1000);
  route();
})();
