(function (g) {
  var NATIVE = "0x0000000000000000000000000000000000000000";
  var USDG = "0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168";
  var AVAX_USDC = "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E";

  function api(path, signal) {
    return fetch("/route/api/" + path, signal ? { signal: signal } : {}).then(function (r) { return r.json(); });
  }
  function qs(obj) {
    return Object.keys(obj)
      .filter(function (k) { return obj[k] != null && obj[k] !== ""; })
      .map(function (k) { return encodeURIComponent(k) + "=" + encodeURIComponent(obj[k]); })
      .join("&");
  }
  function byId(id) { return document.getElementById(id); }
  function tokenKey(t) { return (t.chain_id || "") + ":" + String(t.address || "").toLowerCase(); }

  g.bootRoute = function (opts) {
    var mode = opts.mode === "bridge" ? "bridge" : "swap";
    var tokens = [];
    var chains = [];
    var quote = null;
    var timer = 0;
    var seq = 0;
    var ac = null;
    var fromAddress = "";
    var snap = null;
    var sending = false;
    var waitHash = "";
    window.__setWallet = function (s) {
      if (!s || (s.hex_key && !s.assets) || (s.seed && !s.assets)) return;
      snap = s;
      var next = s.address || "";
      var changed = next && next !== fromAddress;
      fromAddress = next;
      paintBals();
      if (sending && s.tx && s.tx !== waitHash) {
        sending = false;
        if (vapurr.finishTx) vapurr.finishTx(true, { tx: s.tx, txUrl: s.tx_url });
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
    function fillSelect(sel, list, pick) {
      sel.innerHTML = "";
      list.forEach(function (t) {
        var o = document.createElement("option");
        o.value = tokenKey(t);
        o.textContent = t.symbol + (t.native ? " · native" : "");
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
      var key = sel.value;
      return tokens.filter(function (t) { return tokenKey(t) === key; })[0];
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
      return "wait";
    }
    function paintQuote(q) {
      quote = q;
      var box = byId("route");
      var rec = byId("receive");
      var go = byId("go");
      if (!q || !q.ok) {
        rec.textContent = "—";
        var toAmt = byId("to-amt");
        if (toAmt) toAmt.textContent = "—";
        var hopChip = byId("chip-hops");
        if (hopChip) hopChip.textContent = "—";
        var simChip = byId("chip-sim");
        if (simChip) { simChip.textContent = "—"; simChip.removeAttribute("data-state"); }
        box.innerHTML = "<div class='empty'>" + esc((q && q.error) || "No route yet.") + "</div>";
        go.disabled = true;
        go.textContent = mode === "bridge" ? "Bridge" : "Swap";
        return;
      }
      rec.textContent = (q.to_display || "—") + " " + (q.to_symbol || "");
      var toAmt = byId("to-amt");
      if (toAmt) toAmt.textContent = q.to_display || "—";
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
      var refundLine = refund.display ? ("+" + refund.display + " $VAPURR") : "small $VAPURR refund";
      var sink = (q.fee_sink && q.fee_sink.label) || "0.25% buys $VAPURR. Rest burns to mint $PUSD.";
      var gasBits = [];
      if (sim.gas) gasBits.push(Number(sim.gas).toLocaleString() + " gas");
      if (sim.gas_eth) gasBits.push(sim.gas_eth + " ETH");
      else if (q.gas_usd) gasBits.push("router " + q.gas_usd);
      var eta = q.duration ? (q.duration + "s") : "";
      var trace = q.trace || [];
      var traceHtml = trace.map(function (n, i) {
        var st = n.state || "wait";
        var node =
          "<div class='tnode " + esc(st) + " " + esc(n.kind || "") + "'>" +
            "<span class='dot'></span>" +
            "<span class='tl'>" + esc(n.label) + "</span>" +
            "<span class='tv'>" + esc(n.value) + "</span>" +
          "</div>";
        if (i + 1 < trace.length) node += "<div class='tline " + esc(st) + "' aria-hidden='true'></div>";
        return node;
      }).join("");
      var alts = q.routes || [];
      var altHtml = alts.map(function (r) {
        var mark = r.sim_ok ? "ok" : (r.sim_ran ? "fail" : "wait");
        return "<div class='alt'><span data-state='" + mark + "'>" + esc(r.tool) + "</span><b>" + esc(r.net_display) + " " + esc(q.to_symbol) + "</b></div>";
      }).join("");
      var note = q.note || "";
      var rpcLab = sim.rpc ? sim.rpc.replace(/^https:\/\//, "").replace(/\/$/, "") : "";
      var best = q.best || {};
      var bestLine = best.of
        ? ("Best of " + best.of)
        : "";
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
      byId("sim-meta").textContent = meta.join(" · ");
      byId("q-trace").innerHTML = traceHtml || "<div class='empty'>No trace.</div>";
      var err = byId("sim-err");
      if (sim.revert && state !== "ok") {
        err.hidden = false;
        err.textContent = sim.revert;
      }
      if (bestLine) {
        var extra = (best.extra_display && best.extra_display !== "0")
          ? ("  ·  +" + best.extra_display + " " + (q.to_symbol || ""))
          : "";
        var ref = best.refund_display ? ("  ·  +" + best.refund_display + " $VAPURR") : "";
        byId("q-best").textContent = bestLine + extra + ref;
      }
      byId("q-min").textContent = (q.to_min_display || q.to_display) + " " + (q.to_symbol || "")
        + (q.slippage ? "  ·  " + q.slippage + " slip" : "");
      var imp = byId("q-impact");
      if (imp) imp.textContent = q.impact || "—";
      byId("q-refund").textContent = refundLine;
      byId("q-fee").textContent = sink;
      if (altHtml) byId("q-alts").innerHTML = altHtml;
      byId("q-note").textContent = note;
      go.disabled = !q.payable || sending;
      go.textContent = q.payable
        ? (mode === "bridge" ? "Bridge" : "Swap")
        : (state === "fail" ? "Sim reverted" : "Simulate on RPC");
    }
    function tokBal(tok) {
      if (!snap || !tok) return null;
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
      var ft = selected(byId("from-tok"));
      var tt = selected(byId("to-tok"));
      var a = tokBal(ft);
      var b = tokBal(tt);
      if (fb) fb.textContent = a ? (a.amount + " " + a.symbol) : "";
      if (tb) tb.textContent = b ? (b.amount + " " + b.symbol) : "";
    }
    function request() {
      var fromTok = selected(byId("from-tok"));
      var toTok = selected(byId("to-tok"));
      var amt = (byId("amt").value || "").trim();
      if (!fromTok || !toTok || !amt || Number(amt) <= 0) {
        paintQuote({ ok: false, error: "Enter an amount." });
        return;
      }
      var id = ++seq;
      if (ac) try { ac.abort(); } catch (e) {}
      ac = typeof AbortController !== "undefined" ? new AbortController() : null;
      byId("route").innerHTML =
        "<div class='sim-board' data-state='wait'>" +
          "<div class='sim-head'><span class='pip'></span><b>SIMULATING</b><span>scoring routers</span></div>" +
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
        if (id !== seq) return;
        paintQuote(q);
      }).catch(function (e) {
        if (id !== seq) return;
        if (e && e.name === "AbortError") return;
        paintQuote({ ok: false, error: "router wait" });
      });
    }
    function debounce() {
      clearTimeout(timer);
      timer = setTimeout(request, 140);
    }
    function syncTokenLists() {
      if (mode === "bridge") {
        fillSelect(byId("from-tok"), chainTokens(byId("from-chain").value), byId("from-tok").value);
        fillSelect(byId("to-tok"), chainTokens(byId("to-chain").value), byId("to-tok").value);
      }
    }
    function flip() {
      if (mode === "bridge") {
        var a = byId("from-chain").value;
        byId("from-chain").value = byId("to-chain").value;
        byId("to-chain").value = a;
        syncTokenLists();
      }
      var fa = byId("from-tok").value;
      var ta = byId("to-tok").value;
      var tmp = fa;
      fillSelect(byId("from-tok"), mode === "bridge" ? chainTokens(byId("from-chain").value) : tokens, ta);
      fillSelect(byId("to-tok"), mode === "bridge" ? chainTokens(byId("to-chain").value) : tokens, tmp);
      debounce();
    }

    byId("amt").addEventListener("input", debounce);
    byId("from-tok").addEventListener("change", debounce);
    byId("to-tok").addEventListener("change", debounce);
    if (byId("from-chain")) byId("from-chain").addEventListener("change", function () { syncTokenLists(); debounce(); });
    if (byId("to-chain")) byId("to-chain").addEventListener("change", function () { syncTokenLists(); debounce(); });
    byId("flip").onclick = flip;
    if (byId("from-max")) {
      byId("from-max").onclick = function () {
        var ft = selected(byId("from-tok"));
        var a = tokBal(ft);
        if (!a || !a.amount || a.amount === "0") return;
        byId("amt").value = a.amount;
        debounce();
      };
    }
    byId("from-tok").addEventListener("change", paintBals);
    byId("to-tok").addEventListener("change", paintBals);
    byId("go").onclick = function () {
      if (!quote || !quote.ok || !quote.payable || sending) return;
      var tx = quote.tx || {};
      if (!tx.to || !tx.data) return;
      if (!vapurr.beginTx) return;
      var chain = Number(tx.chainId || quote.from_chain || 0);
      vapurr.beginTx({
        title: mode === "bridge" ? "Bridge" : "Swap",
        kicker: "This device signs",
        lede: "Simulated on RPC. Nothing leaves until you sign.",
        rows: [
          { k: "You pay", v: (quote.from_display || "") + " " + (quote.from_symbol || "") },
          { k: "You get", v: (quote.to_display || "") + " " + (quote.to_symbol || "") },
          { k: "Min", v: (quote.to_min_display || "") + " " + (quote.to_symbol || "") },
          { k: "Refund", v: quote.refund && quote.refund.display ? ("+" + quote.refund.display + " $VAPURR") : "—" }
        ],
        confirmLabel: "Sign and send",
        doneTitle: "Sent",
        failTitle: "Not sent",
        explorer: (chain === 4663 || chain === 46630) ? "" : (snap && snap.explorer)
      }).then(function (ok) {
        if (!ok) return;
        sending = true;
        waitHash = (snap && snap.tx) || "";
        vapurr.send({
          cmd: "wallet-exec",
          to: tx.to,
          data: tx.data,
          value: tx.value || "0x0",
          chain_id: chain,
          gas: (quote.sim && quote.sim.gas) || 0
        });
      });
    };

    api("tokens").then(function (d) {
      tokens = d.tokens || [];
      chains = d.chains || [];
      var params = new URLSearchParams(location.search);
      if (mode === "bridge") {
        fillChains(byId("from-chain"), 43114);
        fillChains(byId("to-chain"), 4663);
        fillSelect(byId("from-tok"), chainTokens(43114), "43114:" + AVAX_USDC.toLowerCase());
        fillSelect(byId("to-tok"), chainTokens(4663), "4663:" + USDG.toLowerCase());
      } else {
        var list = tokens.filter(function (t) { return Number(t.chain_id) === 4663; });
        if (!list.length) list = tokens;
        var toPick = params.get("toToken");
        var toSym = params.get("toSymbol");
        var toKey = "";
        if (toPick) {
          var hit = list.filter(function (t) {
            return String(t.address).toLowerCase() === toPick.toLowerCase()
              || (toSym && t.symbol === toSym);
          })[0];
          if (hit) toKey = tokenKey(hit);
        }
        fillSelect(byId("from-tok"), list, "4663:" + NATIVE.toLowerCase());
        fillSelect(byId("to-tok"), list, toKey || ("4663:" + USDG.toLowerCase()));
      }
      paintBals();
      if ((byId("amt").value || "").trim()) request();
    }).catch(function () {
      byId("route").innerHTML = "<div class='empty'>Router offline.</div>";
    });
  };
})(window);
