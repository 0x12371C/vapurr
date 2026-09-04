(function (g) {
  var NATIVE = "0x0000000000000000000000000000000000000000";
  var USDG = "0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168";
  var AVAX_USDC = "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E";

  function api(path) {
    return fetch("/route/api/" + path).then(function (r) { return r.json(); });
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
        box.innerHTML = "<div class='empty'>" + ((q && q.error) || "No route yet.") + "</div>";
        go.disabled = true;
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
      var hops = (q.hops || []).map(function (h) { return h.name || h.tool; }).join(" → ");
      var feeLine = "vapurr scoops 0.25%";
      if (q.fee_usd) feeLine += "  ·  " + q.fee_usd;
      var gas = q.gas_usd ? "Gas " + q.gas_usd : "";
      var eta = q.duration ? (" · " + q.duration + "s") : "";
      var note = q.note || (q.estimate ? "Estimate. LI.FI had no live path." : (q.provider || "LI.FI"));
      box.innerHTML =
        "<div class='row'><span>Min received</span><b id='q-min'></b></div>" +
        "<div class='row'><span>Fee</span><span id='q-fee'></span></div>" +
        "<div class='row'><span>Route</span><span class='hops' id='q-hops'></span></div>" +
        "<div class='note' id='q-note'></div>";
      byId("q-min").textContent = (q.to_min_display || q.to_display) + " " + (q.to_symbol || "");
      byId("q-fee").textContent = feeLine + (gas ? "  ·  " + gas : "") + eta;
      byId("q-hops").textContent = hops || "—";
      byId("q-note").textContent = note;
      go.disabled = false;
      go.textContent = q.tx ? (mode === "bridge" ? "Bridge" : "Swap") : "Copy route";
    }
    function request() {
      var fromTok = selected(byId("from-tok"));
      var toTok = selected(byId("to-tok"));
      var amt = (byId("amt").value || "").trim();
      if (!fromTok || !toTok || !amt || Number(amt) <= 0) {
        paintQuote({ ok: false, error: "Enter an amount." });
        return;
      }
      byId("route").innerHTML = "<div class='empty'>Routing…</div>";
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
        amount: amt
      })).then(paintQuote).catch(function () {
        paintQuote({ ok: false, error: "router wait" });
      });
    }
    function debounce() {
      clearTimeout(timer);
      timer = setTimeout(request, 280);
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
    byId("go").onclick = function () {
      if (!quote || !quote.ok) return;
      if (quote.tx && quote.tx.data) {
        var blob = JSON.stringify(quote.tx, null, 2);
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(blob).catch(function () {});
        }
        vapurr.go("vapurr://wallet");
        return;
      }
      vapurr.go("vapurr://wallet");
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
      if ((byId("amt").value || "").trim()) request();
    }).catch(function () {
      byId("route").innerHTML = "<div class='empty'>Router offline.</div>";
    });
  };
})(window);
