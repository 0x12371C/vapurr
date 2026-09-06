(function (g) {
  var NATIVE = "0x0000000000000000000000000000000000000000";
  var USDG = "0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168";
  var AVAX_USDC = "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E";
  var SWAP_SYMS = {
    ETH: 1, VAPURR: 1, WGV: 1, PUSD: 1, USDG: 1, WETH: 1,
    GV: 1, SPUSD: 1, MUSDG: 1, EETH: 1, ENVDA: 1, EAMD: 1, EAMZN: 1, ETSLA: 1, ENFLX: 1, EPLTR: 1,
    NVDA: 1, TSLA: 1, HOOD: 1, PLTR: 1, MSFT: 1,
    AAPL: 1, AMZN: 1, GOOGL: 1, META: 1, AMD: 1,
    COIN: 1, SPY: 1, QQQ: 1, INTC: 1, ORCL: 1, NFLX: 1
  };
  function swapList(list) {
    return (list || []).filter(function (t) {
      var s = String(t.symbol || "").toUpperCase().replace(/^\$/, "");
      return !!SWAP_SYMS[s];
    });
  }
  function prettySym(s) {
    var u = String(s || "").replace(/^\$/, "").toUpperCase();
    if (u === "VAPURR" || u === "PUSD") return "$" + u;
    return s || "";
  }
  function api(path, signal) {
    return fetch("/route/api/" + path, signal ? { signal: signal } : {}).then(function (r) {
      if (!r.ok) throw new Error("router " + r.status);
      return r.json();
    });
  }
  function qs(obj) {
    return Object.keys(obj)
      .filter(function (k) { return obj[k] != null && obj[k] !== ""; })
      .map(function (k) { return encodeURIComponent(k) + "=" + encodeURIComponent(obj[k]); })
      .join("&");
  }
  function byId(id) { return document.getElementById(id); }
  function tokenKey(t) {
    if (!t) return "";
    return (t.chain_id || "") + ":" + String(t.address || "").toLowerCase();
  }
  function isSelect(el) { return el && el.tagName === "SELECT"; }

  g.bootRoute = function (opts) {
    var mode = opts.mode === "bridge" ? "bridge" : "swap";
    var seed = null;
    try { seed = JSON.parse((byId("route-catalog") || {}).textContent || "null"); } catch (_) {}
    var tokens = [];
    var chains = [];
    var quote = null;
    var expires = 0;
    var expiryTimer = 0;
    var pendingChain = 0;
    var approving = false;
    var timer = 0;
    var seq = 0;
    var ac = null;
    var fromAddress = "";
    var snap = null;
    var sending = false;
    var waitHash = "";
    var fromPick = null;
    var toPick = null;
    var pickWhich = "from";
    var tokList = [];
    window.__setWallet = function (s) {
      if (!s || !Array.isArray(s.assets)) return;
      var previousChain = snap && snap.chain_id;
      snap = s;
      var next = s.address || "";
      var changed = next !== fromAddress || previousChain !== s.chain_id;
      fromAddress = next;
      if (mode !== "bridge") applySwapList();
      paintBals();
      if (sending && s.tx && s.tx !== waitHash && Number(s.tx_chain_id) === pendingChain) {
        sending = false;
        if (vapurr.finishTx) vapurr.finishTx(s.tx_status === "confirmed", {
          tx: s.tx, txUrl: s.tx_url, tx_status: s.tx_status, tx_chain_id: s.tx_chain_id,
          title: mode === "bridge" && !approving ? "Bridge source transaction" : undefined,
          lede: mode === "bridge" && !approving ? "Destination receipt is not yet verified. Do not submit again." : undefined
        });
        debounce();
      }
      if (changed) debounce();
    };
    window.__walletErr = function (msg) {
      sending = false;
      if (vapurr.pendingTx && vapurr.finishTx) vapurr.finishTx(false, { error: msg || "not sent" });
    };
    try { vapurr.send({ cmd: "wallet" }); } catch (e) {}

    function chainTokens(cid) {
      return tokens.filter(function (t) { return Number(t.chain_id) === Number(cid); });
    }
    function activeChain() {
      if (mode === "bridge") {
        var el = byId("from-chain");
        return el ? Number(el.value) : 4663;
      }
      if (snap && snap.chain_id) return Number(snap.chain_id);
      return 46630;
    }
    function swapTokens() {
      var cid = activeChain();
      var list = tokens.filter(function (t) { return Number(t.chain_id) === cid; });
      var out = swapList(list);
      var rank = { WGV: 0, PUSD: 1, VAPURR: 2, ETH: 3 };
      out.sort(function (a, b) {
        var sa = String(a.symbol || "").toUpperCase().replace(/^\$/, "");
        var sb = String(b.symbol || "").toUpperCase().replace(/^\$/, "");
        var ra = rank[sa] != null ? rank[sa] : 9;
        var rb = rank[sb] != null ? rank[sb] : 9;
        if (ra !== rb) return ra - rb;
        return sa.localeCompare(sb);
      });
      return out;
    }
    function fillSelect(sel, list, pick) {
      if (!sel || !isSelect(sel)) return;
      sel.innerHTML = "";
      list.forEach(function (t) {
        var o = document.createElement("option");
        o.value = tokenKey(t);
        o.textContent = prettySym(t.symbol) + (t.native ? " · gas" : "");
        sel.appendChild(o);
      });
      if (pick) {
        for (var i = 0; i < sel.options.length; i++) {
          if (sel.options[i].value === pick) { sel.selectedIndex = i; break; }
        }
      }
    }
    function fillChains(sel, pick) {
      sel.innerHTML = "";
      chains.forEach(function (c) {
        var o = document.createElement("option");
        o.value = String(c.id);
        o.textContent = c.name;
        sel.appendChild(o);
      });
      if (pick) sel.value = String(pick);
    }
    function selected(sel) {
      if (!sel || !isSelect(sel)) return null;
      var key = sel.value;
      return tokens.filter(function (t) { return tokenKey(t) === key; })[0];
    }
    function current(which) {
      if (which === "from") {
        var el = byId("from-tok");
        return isSelect(el) ? selected(el) : fromPick;
      }
      var el2 = byId("to-tok");
      return isSelect(el2) ? selected(el2) : toPick;
    }
    function paintTokBtn(btn, t) {
      if (!btn || isSelect(btn) || !t) return;
      btn.innerHTML = "";
      var ico = g.tokenIconEl
        ? tokenIconEl(String(t.symbol || "").toLowerCase(), t.symbol)
        : null;
      if (ico) btn.appendChild(ico);
      var lab = document.createElement("span");
      lab.className = "tok-sym";
      lab.textContent = prettySym(t.symbol);
      btn.appendChild(lab);
      btn.dataset.key = tokenKey(t);
    }
    function pickInList(list, key, symbol) {
      var i;
      if (key) {
        for (i = 0; i < list.length; i++) if (tokenKey(list[i]) === key) return list[i];
      }
      if (symbol) {
        var s = String(symbol).toUpperCase().replace(/^\$/, "");
        for (i = 0; i < list.length; i++) {
          if (String(list[i].symbol || "").toUpperCase().replace(/^\$/, "") === s) return list[i];
        }
      }
      return null;
    }
    function applySwapList() {
      var list = swapTokens();
      var network = byId("route-network");
      var info = chains.filter(function (c) { return Number(c.id) === activeChain(); })[0];
      if (network) network.textContent = info ? info.name : "Chain " + activeChain();
      tokList = list;
      var fromEl = byId("from-tok");
      var toEl = byId("to-tok");
      if (isSelect(fromEl)) {
        fillSelect(fromEl, list, fromEl.value);
        fillSelect(toEl, list, toEl.value);
        return;
      }
      fromPick = pickInList(list, fromPick && tokenKey(fromPick), fromPick && fromPick.symbol)
        || pickInList(list, "", "WGV")
        || pickInList(list, "", "ETH")
        || list[0]
        || null;
      toPick = pickInList(list, toPick && tokenKey(toPick), toPick && toPick.symbol)
        || pickInList(list, "", "PUSD")
        || pickInList(list, "", "VAPURR")
        || list[1]
        || list[0]
        || null;
      if (fromPick && toPick && tokenKey(fromPick) === tokenKey(toPick) && list.length > 1) {
        toPick = list[0] === fromPick ? list[1] : list[0];
      }
      paintTokBtn(fromEl, fromPick);
      paintTokBtn(toEl, toPick);
    }
    function openTokSheet(which) {
      var sheet = byId("tok-sheet");
      var box = byId("tok-list");
      if (!sheet || !box) return;
      pickWhich = which;
      var cur = current(which);
      box.innerHTML = "";
      tokList.forEach(function (t) {
        var b = document.createElement("button");
        b.type = "button";
        b.className = "tok-row" + (cur && tokenKey(cur) === tokenKey(t) ? " on" : "");
        var ico = g.tokenIconEl
          ? tokenIconEl(String(t.symbol || "").toLowerCase(), t.symbol)
          : document.createElement("span");
        var grow = document.createElement("span");
        grow.className = "grow";
        grow.innerHTML = "<b></b><span class='hint'></span>";
        grow.querySelector("b").textContent = prettySym(t.symbol);
        var bal = tokBal(t);
        grow.querySelector(".hint").textContent = t.native ? "Network gas" :
          String(t.address).slice(0, 6) + "…" + String(t.address).slice(-4);
        if (bal && bal.amount !== "0") grow.querySelector(".hint").textContent += " · " + bal.amount;
        b.appendChild(ico);
        b.appendChild(grow);
        b.onclick = function () {
          if (which === "from") fromPick = t;
          else toPick = t;
          paintTokBtn(byId(which === "from" ? "from-tok" : "to-tok"), t);
          sheet.hidden = true;
          paintBals();
          debounce();
        };
        box.appendChild(b);
      });
      sheet.hidden = false;
    }
    function esc(s) {
      return String(s || "")
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");
    }
    function simState(q) {
      var s = (q && q.sim) || {};
      if (s.ok) return "ok";
      if (s.ran) return "fail";
      if (q && (q.simulated === false || q.estimate || q.payable === false)) return "fail";
      return "wait";
    }
    function traceHtmlFrom(nodes) {
      return (nodes || []).map(function (n, i) {
        var st = n.state || "wait";
        var node =
          "<div class='tnode " + esc(st) + " " + esc(n.kind || "") + "'>" +
            "<span class='dot'></span>" +
            "<span class='tl'>" + esc(n.label) + "</span>" +
            "<span class='tv'>" + esc(n.value) + "</span>" +
          "</div>";
        if (i + 1 < nodes.length) node += "<div class='tline " + esc(st) + "' aria-hidden='true'></div>";
        return node;
      }).join("");
    }
    function idleTrace() {
      var ft = current("from");
      var tt = current("to");
      return [
        { state: "held", kind: "in", label: "You pay", value: ft ? prettySym(ft.symbol) : "—" },
        { state: "held", kind: "swap", label: "Route", value: "—" },
        { state: "held", kind: "out", label: "Estimated receipt", value: tt ? prettySym(tt.symbol) : "—" }
      ];
    }
    function paintIdle(msg) {
      quote = null;
      expires = 0;
      clearTimeout(expiryTimer);
      var box = byId("route");
      var rec = byId("receive");
      var go = byId("go");
      var toAmt = byId("to-amt");
      var hopChip = byId("chip-hops");
      var simChip = byId("chip-sim");
      if (rec) { rec.hidden = true; rec.textContent = ""; }
      if (toAmt) toAmt.textContent = "0";
      if (hopChip) hopChip.textContent = "—";
      if (simChip) { simChip.textContent = "idle"; simChip.removeAttribute("data-state"); }
      if (box) {
        box.innerHTML =
          "<div class='sim-board' data-state='wait'>" +
            "<div class='sim-head'><span class='pip'></span><b>SIM</b><span>enter an amount</span></div>" +
            "<div class='trace'>" + traceHtmlFrom(idleTrace()) + "</div>" +
          "</div>" +
          "<div class='row'><span>Min received</span><b>—</b></div>" +
          "<div class='row'><span>Impact</span><b>—</b></div>" +
          "<div class='row'><span>Refund</span><b>None</b></div>" +
          "<div class='row'><span>Fee</span><span>Shown with quote</span></div>" +
          "<div class='note'>" + esc(msg || "Enter an amount. We simulate on this device before you sign.") + "</div>";
      }
      if (go) {
        if (go._slideHold) {
          go._slideHold.setDisabled(true);
          go._slideHold.setLabel(mode === "bridge" ? "Bridge" : "Swap");
        } else {
          go.disabled = true;
          go.textContent = mode === "bridge" ? "Bridge" : "Swap";
        }
      }
    }
    function paintQuote(q) {
      quote = q;
      var box = byId("route");
      var rec = byId("receive");
      var go = byId("go");
      if (!q || !q.ok) {
        paintIdle((q && q.error) || "Enter an amount.");
        return;
      }
      if (rec) {
        rec.hidden = false;
        rec.textContent = (q.to_display || "") + " " + prettySym(q.to_symbol);
      }
      var toAmt = byId("to-amt");
      if (toAmt) toAmt.textContent = q.to_display || "0";
      var hopChip = byId("chip-hops");
      if (hopChip) {
        var n = (q.hops || []).length;
        hopChip.textContent = n ? (n + (n === 1 ? " hop" : " hops")) : "—";
      }
      var state = simState(q);
      var simChip = byId("chip-sim");
      if (simChip) {
        simChip.setAttribute("data-state", state);
        simChip.textContent = state === "ok" ? "OK" : (state === "fail" ? "REVERT" : "WAIT");
      }
      var sim = q.sim || {};
      var refund = q.refund || {};
      var refundLine = "Not included";
      var refChip = byId("chip-refund");
      if (refChip) {
        var rb = (refund.bps != null ? refund.bps : q.refund_bps);
        refChip.textContent = "None";
      }
      var sink = (q.fee_sink && q.fee_sink.label) || "Provider fees included; gas separate";
      var gasBits = [];
      if (sim.gas) gasBits.push(Number(sim.gas).toLocaleString() + " gas");
      if (sim.gas_eth) gasBits.push(sim.gas_eth + " ETH");
      else if (q.gas_usd) gasBits.push("router " + q.gas_usd);
      var eta = q.duration ? (q.duration + "s") : "";
      var trace = q.trace && q.trace.length ? q.trace : idleTrace();
      var alts = q.routes || [];
      var altHtml = alts.map(function (r) {
        var mark = r.sim_ok ? "ok" : (r.sim_ran ? "fail" : "wait");
        return "<div class='alt'><span data-state='" + mark + "'>" + esc(r.tool) + "</span><b>" + esc(r.net_display) + " " + esc(prettySym(q.to_symbol)) + "</b></div>";
      }).join("");
      var note = q.note || "";
      var rpcLab = sim.rpc ? sim.rpc.replace(/^https:\/\//, "").replace(/\/$/, "") : "";
      var best = q.best || {};
      var bestLine = best.of ? ("Best of " + best.of) : "";
      box.innerHTML =
        (bestLine ? "<div class='best' id='q-best'></div>" : "") +
        "<div class='sim-board' data-state='" + state + "'>" +
          "<div class='sim-head'><span class='pip'></span><b id='sim-lab'></b><span id='sim-meta'></span></div>" +
          "<div class='trace' id='q-trace'></div>" +
          "<div class='sim-err' id='sim-err' hidden></div>" +
        "</div>" +
        "<div class='row'><span>Min received</span><b id='q-min'></b></div>" +
        "<div class='row'><span>Impact</span><b id='q-impact'></b></div>" +
        "<div class='row'><span>Refund</span><b id='q-refund'></b></div>" +
        "<div class='row'><span>Fee</span><span id='q-fee'></span></div>" +
        (altHtml ? "<div class='alts-lab'>Other routers</div><div class='alts' id='q-alts'></div>" : "") +
        "<div class='note' id='q-note'></div>";
      byId("sim-lab").textContent = state === "ok" ? "SIMULATED" : (state === "fail" ? "REVERT" : "SIMULATING");
      var meta = [];
      if (gasBits.length) meta.push(gasBits.join(" · "));
      if (q.ms) meta.push(q.ms + " ms");
      if (eta) meta.push(eta);
      if (rpcLab) meta.push(rpcLab);
      byId("sim-meta").textContent = meta.join(" · ") || "this device";
      byId("q-trace").innerHTML = traceHtmlFrom(trace);
      var err = byId("sim-err");
      if (sim.revert && state !== "ok") {
        err.hidden = false;
        err.textContent = sim.revert;
      }
      if (bestLine) {
        var extra = (best.extra_display && best.extra_display !== "0")
          ? ("  ·  +" + best.extra_display + " " + prettySym(q.to_symbol || ""))
          : "";
        var ref = "";
        byId("q-best").textContent = bestLine + extra + ref;
      }
      byId("q-min").textContent = (q.to_min_display || q.to_display) + " " + prettySym(q.to_symbol || "")
        + (q.slippage ? "  ·  " + q.slippage + " slip" : "");
      var imp = byId("q-impact");
      if (imp) imp.textContent = q.impact || "—";
      var house = q.tool === "house";
      if (house) {
        byId("q-refund").textContent = "none";
        byId("q-fee").textContent = sink;
        if (hopChip) hopChip.textContent = "house";
      } else {
        byId("q-refund").textContent = refundLine;
        byId("q-fee").textContent = sink;
      }
      byId("q-note").textContent = note;
      if (altHtml) byId("q-alts").innerHTML = altHtml;
      var canApprove = !!(q.needs_approve && q.approve && q.approve.to && q.approve.data && q.approve.execution_id);
      var goLab = canApprove && !q.payable
        ? ("Approve " + prettySym(q.from_symbol))
        : (mode === "bridge" ? "Bridge" : "Swap");
      if (go._slideHold) {
        go._slideHold.setDisabled(sending || !((q.payable && q.execution_id) || canApprove));
        go._slideHold.setLabel(goLab);
      } else {
        go.disabled = sending || !((q.payable && q.execution_id) || canApprove);
        go.textContent = goLab;
      }
    }
    function tokBal(tok) {
      if (!snap || !tok || Number(snap.chain_id) !== Number(tok.chain_id)) return null;
      if (tok.native || String(tok.address).replace(/0x/, "") === "") {
        return { amount: snap.eth || "0", symbol: "ETH" };
      }
      var assets = snap.assets || [];
      var hit = assets.filter(function (a) {
        return a && a.token && String(a.token).toLowerCase() === String(tok.address).toLowerCase();
      })[0];
      if (hit) return { amount: hit.amount || "0", symbol: hit.symbol || tok.symbol };
      if (tok.symbol === "USDG") return { amount: snap.usdg || "0", symbol: "USDG" };
      if (tok.symbol === "PUSD") return { amount: snap.pusd || "0", symbol: "PUSD" };
      if (tok.symbol === "VAPURR") return { amount: snap.vapurr || "0", symbol: "VAPURR" };
      if (tok.symbol === "WETH") return { amount: snap.weth || "0", symbol: "WETH" };
      return { amount: "0", symbol: tok.symbol };
    }
    function paintBals() {
      var fb = byId("from-bal");
      var tb = byId("to-bal");
      var ft = current("from");
      var tt = current("to");
      var a = tokBal(ft);
      var b = tokBal(tt);
      if (fb) fb.textContent = a ? (a.amount + " " + prettySym(a.symbol)) : "";
      if (tb) tb.textContent = b ? (b.amount + " " + prettySym(b.symbol)) : "";
    }
    function requestKey() {
      return [fromAddress, activeChain(), tokenKey(current("from")), tokenKey(current("to")), (byId("amt").value || "").trim()].join("|");
    }
    function invalidate() {
      ++seq;
      if (ac) ac.abort();
      quote = null; expires = 0; clearTimeout(expiryTimer);
      var go = byId("go");
      if (go._slideHold) go._slideHold.setDisabled(true); else go.disabled = true;
    }
    function request() {
      invalidate();
      var key = requestKey();
      var started = Date.now();
      var fromTok = current("from");
      var toTok = current("to");
      var amt = (byId("amt").value || "").trim();
      if (!fromTok || !toTok || !amt || Number(amt) <= 0) {
        paintIdle("Enter an amount. We simulate on this device before you sign.");
        return;
      }
      var id = ++seq;
      if (ac) try { ac.abort(); } catch (e) {}
      ac = typeof AbortController !== "undefined" ? new AbortController() : null;
      var controller = ac;
      var timeout = setTimeout(function () { if (controller) controller.abort(); }, 15000);
      byId("route").innerHTML =
        "<div class='sim-board' data-state='wait'>" +
          "<div class='sim-head'><span class='pip'></span><b>SIMULATING</b><span>house book</span></div>" +
          "<div class='trace skel'><span></span><span></span><span></span><span></span></div>" +
        "</div>";
      var fromChain = mode === "bridge" ? Number(byId("from-chain").value) : fromTok.chain_id;
      var toChain = mode === "bridge" ? Number(byId("to-chain").value) : toTok.chain_id;
      api("quote?" + qs({
        fromChain: fromChain,
        toChain: toChain,
        fromToken: fromTok.address,
        toToken: toTok.address,
        fromSymbol: fromTok.symbol,
        toSymbol: toTok.symbol,
        fromDecimals: fromTok.decimals,
        toDecimals: toTok.decimals,
        amount: amt,
        fromAddress: fromAddress
      }), ac && ac.signal).then(function (q) {
        clearTimeout(timeout);
        if (id !== seq || key !== requestKey()) return;
        paintQuote(q);
        if (!quote) return;
        expires = started + Math.min(Number(q.expires_in_ms) || 0, 40000);
        if (expires <= Date.now()) { paintIdle("Quote expired. Enter an amount to refresh."); return; }
        expiryTimer = setTimeout(function () { invalidate(); paintIdle("Quote expired. Edit the amount to refresh."); }, expires - Date.now());
      }).catch(function (e) {
        clearTimeout(timeout);
        if (id !== seq) return;
        paintIdle(e && e.name === "AbortError" ? "Quote timed out." : "Router wait.");
      });
    }
    function debounce() {
      invalidate();
      clearTimeout(timer);
      timer = setTimeout(request, 140);
    }
    function syncTokenLists() {
      if (mode === "bridge") {
        fillSelect(byId("from-tok"), chainTokens(byId("from-chain").value), byId("from-tok").value);
        fillSelect(byId("to-tok"), chainTokens(byId("to-chain").value), byId("to-tok").value);
      } else {
        applySwapList();
      }
    }
    function flip() {
      if (mode === "bridge") {
        var a = byId("from-chain").value;
        byId("from-chain").value = byId("to-chain").value;
        byId("to-chain").value = a;
        syncTokenLists();
        var fa = byId("from-tok").value;
        var ta = byId("to-tok").value;
        fillSelect(byId("from-tok"), chainTokens(byId("from-chain").value), ta);
        fillSelect(byId("to-tok"), chainTokens(byId("to-chain").value), fa);
      } else {
        var tmp = fromPick;
        fromPick = toPick;
        toPick = tmp;
        paintTokBtn(byId("from-tok"), fromPick);
        paintTokBtn(byId("to-tok"), toPick);
      }
      paintBals();
      debounce();
    }

    byId("amt").addEventListener("input", debounce);
    if (isSelect(byId("from-tok"))) {
      byId("from-tok").addEventListener("change", function () { paintBals(); debounce(); });
      byId("to-tok").addEventListener("change", function () { paintBals(); debounce(); });
    } else {
      byId("from-tok").addEventListener("click", function () { openTokSheet("from"); });
      byId("to-tok").addEventListener("click", function () { openTokSheet("to"); });
    }
    if (byId("from-chain")) byId("from-chain").addEventListener("change", function () { syncTokenLists(); debounce(); });
    if (byId("to-chain")) byId("to-chain").addEventListener("change", function () { syncTokenLists(); debounce(); });
    byId("flip").onclick = flip;
    if (byId("from-max")) {
      byId("from-max").onclick = function () {
        var ft = current("from");
        var a = tokBal(ft);
        if (!a || !a.amount || a.amount === "0") return;
        byId("amt").value = a.amount;
        debounce();
      };
    }
    if (byId("tok-dim")) byId("tok-dim").onclick = function () { byId("tok-sheet").hidden = true; };
    function doGo() {
      if (sending) return;
      if (!vapurr.beginTx) return;
      if (!quote || !quote.ok || Date.now() >= expires) { debounce(); return; }
      var reviewed = quote, reviewedSeq = seq, key = requestKey();
      function stillReviewed() {
        if (quote === reviewed && seq === reviewedSeq && key === requestKey() && Date.now() < expires) return true;
        sending = false;
        if (vapurr.finishTx) vapurr.finishTx(false, { error: "Quote changed or expired. Review a fresh quote." });
        debounce(); return false;
      }
      if (reviewed.needs_approve && reviewed.approve && !reviewed.payable) {
        var ap = reviewed.approve;
        if (!ap.execution_id) return;
        var apChain = Number(ap.chainId || reviewed.from_chain || 0);
        vapurr.beginTx({
          title: "Approve " + prettySym(reviewed.from_symbol),
          kicker: "This device signs",
          lede: "Approve only this amount. The swap requires a fresh quote afterward.",
          rows: [
            { k: "Token", v: prettySym(reviewed.from_symbol) },
            { k: "Spender", v: ap.spender || ap.to },
            { k: "You pay", v: (reviewed.from_display || "") + " " + prettySym(reviewed.from_symbol || "") }
          ],
          confirmLabel: "Sign approve",
          doneTitle: "Approved",
          failTitle: "Not approved",
          explorer: (apChain === 4663 || apChain === 46630) ? "" : (snap && snap.explorer)
        }).then(function (ok) {
          if (!ok || !stillReviewed()) return;
          sending = true; approving = true; pendingChain = apChain;
          waitHash = (snap && snap.tx) || "";
          vapurr.send({
            cmd: "wallet-exec",
            route_id: ap.execution_id,
            to: ap.to,
            data: ap.data,
            value: ap.value || "0x0",
            chain_id: apChain,
            gas: 80000
          });
        });
        return;
      }
      if (!reviewed.payable || !reviewed.execution_id) return;
      var tx = reviewed.tx || {};
      if (!tx.to || !tx.data) return;
      var chain = Number(tx.chainId || reviewed.from_chain || 0);
      vapurr.beginTx({
        title: mode === "bridge" ? "Bridge" : "Swap",
        kicker: "This device signs",
        lede: mode === "bridge" ? "Source transaction simulated. Destination settlement is separate." : "Review this simulated route before signing.",
        rows: [
          { k: "You pay", v: (reviewed.from_display || "") + " " + prettySym(reviewed.from_symbol || "") },
          { k: "Estimated receipt", v: (reviewed.to_display || "") + " " + prettySym(reviewed.to_symbol || "") },
          { k: "Min", v: (reviewed.to_min_display || "") + " " + prettySym(reviewed.to_symbol || "") },
          { k: "Network", v: String(chain) },
          { k: "Router", v: tx.to },
          { k: "Native value (wei)", v: String(BigInt(tx.value || "0x0")) }
        ],
        confirmLabel: "Sign and send",
        doneTitle: "Sent",
        failTitle: "Not sent",
        explorer: (chain === 4663 || chain === 46630) ? "" : (snap && snap.explorer)
      }).then(function (ok) {
        if (!ok || !stillReviewed()) return;
        sending = true; approving = false; pendingChain = chain;
        waitHash = (snap && snap.tx) || "";
        vapurr.send({
          cmd: "wallet-exec",
          route_id: reviewed.execution_id,
          to: tx.to,
          data: tx.data,
          value: tx.value || "0x0",
          chain_id: chain,
          gas: (reviewed.sim && reviewed.sim.gas) || 0
        });
      });
    }
    if (vapurr.bindSlideHold) {
      vapurr.bindSlideHold(byId("go"), { onConfirm: doGo, label: mode === "bridge" ? "Bridge" : "Swap" });
    } else {
      byId("go").onclick = doGo;
    }
    paintIdle();
    function applyCatalog(d) {
      if (!d || !Array.isArray(d.tokens) || !d.tokens.length || !Array.isArray(d.chains)) throw new Error("Empty token catalog");
      tokens = d.tokens || [];
      chains = d.chains || [];
      var params = new URLSearchParams(location.search);
      if (mode === "bridge") {
        fillChains(byId("from-chain"), 43114);
        fillChains(byId("to-chain"), 4663);
        fillSelect(byId("from-tok"), chainTokens(43114), "43114:" + AVAX_USDC.toLowerCase());
        fillSelect(byId("to-tok"), chainTokens(4663), "4663:" + USDG.toLowerCase());
      } else {
        applySwapList();
        var toPickParam = params.get("toToken");
        var toSym = params.get("toSymbol");
        if (toPickParam || toSym) {
          var hit = pickInList(tokList, "", toSym) || tokList.filter(function (t) {
            return toPickParam && String(t.address).toLowerCase() === toPickParam.toLowerCase();
          })[0];
          if (hit) {
            toPick = hit;
            paintTokBtn(byId("to-tok"), toPick);
          }
        }
      }
      paintBals();
      paintIdle();
      if ((byId("amt").value || "").trim()) request();
    }
    function refreshTokens() {
      var controller = typeof AbortController !== "undefined" ? new AbortController() : null;
      var timeout = setTimeout(function () { if (controller) controller.abort(); }, 8000);
      return api("tokens", controller && controller.signal).then(function (d) {
        invalidate(); applyCatalog(d);
      }).catch(function () {
        if (mode !== "bridge") applySwapList();
        if (!tokens.length) paintIdle("Token catalog unavailable. Tap ↻ to retry.");
      }).finally(function () { clearTimeout(timeout); });
    }
    if (seed && Array.isArray(seed.tokens) && seed.tokens.length) applyCatalog(seed);
    if (byId("refresh-tokens")) byId("refresh-tokens").onclick = refreshTokens;
    refreshTokens();
  };
})(window);
