(function (g) {
  var root = null;
  var mode = "review";
  var resolveFn = null;

  function el(tag, cls, text) {
    var n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text != null) n.textContent = text;
    return n;
  }

  function ensure() {
    if (root) return root;
    root = el("div", "vapurr-sign");
    root.hidden = true;
    root.innerHTML =
      '<div class="vapurr-sign-dim"></div>' +
      '<div class="vapurr-sign-panel" role="dialog" aria-modal="true">' +
      '<div class="vapurr-sign-grab"></div>' +
      '<div class="mono" id="vs-kicker">This device</div>' +
      '<h2 id="vs-title">Sign</h2>' +
      '<p id="vs-lede"></p>' +
      '<div id="vs-rows"></div>' +
      '<div class="err" id="vs-err" hidden></div>' +
      '<button class="btn" type="button" id="vs-go">Sign</button>' +
      '<button class="btn vapurr-sign-ghost" type="button" id="vs-no">Reject</button>' +
      "</div>";
    document.body.appendChild(root);
    root.querySelector(".vapurr-sign-dim").onclick = reject;
    root.querySelector("#vs-no").onclick = function () {
      if (mode === "done") close(true);
      else reject();
    };
    root.querySelector("#vs-go").onclick = accept;
    document.addEventListener("keydown", function (e) {
      if (e.key === "Escape" && root && !root.hidden) {
        e.preventDefault();
        if (mode === "done") close(true);
        else reject();
      }
    });
    return root;
  }

  function row(k, v) {
    var r = el("div", "vapurr-sign-row");
    r.appendChild(el("span", "", k));
    var b = el("b", "", v);
    r.appendChild(b);
    return r;
  }

  function show(spec) {
    ensure();
    mode = spec.mode || "review";
    root.hidden = false;
    document.getElementById("vs-kicker").textContent = spec.kicker || "This device signs";
    document.getElementById("vs-title").textContent = spec.title || "Sign";
    document.getElementById("vs-lede").textContent = spec.lede || "";
    var box = document.getElementById("vs-rows");
    box.innerHTML = "";
    (spec.rows || []).forEach(function (x) {
      if (x && x.k) box.appendChild(row(x.k, x.v || "—"));
    });
    var err = document.getElementById("vs-err");
    if (spec.error) {
      err.hidden = false;
      err.textContent = spec.error;
    } else {
      err.hidden = true;
      err.textContent = "";
    }
    var go = document.getElementById("vs-go");
    var no = document.getElementById("vs-no");
    if (mode === "done") {
      go.textContent = spec.txUrl ? "Open explorer" : "Done";
      go.disabled = false;
      no.textContent = spec.txUrl ? "Done" : "Close";
    } else if (mode === "wait") {
      go.textContent = "Waiting on chain…";
      go.disabled = true;
      no.textContent = "Hide";
    } else {
      go.textContent = spec.confirmLabel || "Sign and send";
      go.disabled = false;
      no.textContent = "Reject";
    }
    root._spec = spec;
  }

  function close(ok) {
    if (!root) return;
    root.hidden = true;
    var fn = resolveFn;
    resolveFn = null;
    if (fn) fn(ok);
  }

  function reject() {
    if (mode === "wait") {
      root.hidden = true;
      return;
    }
    close(false);
  }

  function accept() {
    var spec = root && root._spec;
    if (mode === "done") {
      if (spec && spec.txUrl && g.vapurr && g.vapurr.go) g.vapurr.go(spec.txUrl);
      close(true);
      return;
    }
    if (mode === "wait") return;
    close(true);
  }

  g.vapurr = g.vapurr || {};
  g.vapurr.pendingTx = null;

  g.vapurr.txUrl = function (tx, explorer) {
    tx = String(tx || "");
    if (!tx) return "";
    var base = String(explorer || "https://explorer.testnet.chain.robinhood.com").replace(/\/$/, "");
    return base + "/tx/" + tx;
  };

  g.vapurr.reviewTx = function (spec) {
    spec = spec || {};
    spec.mode = "review";
    spec.lede = spec.lede || "Read it. Then sign. You get a hash or it failed.";
    return new Promise(function (resolve) {
      resolveFn = resolve;
      show(spec);
    });
  };

  g.vapurr.waitTx = function (spec) {
    spec = spec || {};
    spec.mode = "wait";
    spec.lede = spec.lede || "Broadcasting. Do not close this.";
    show(spec);
  };

  g.vapurr.showTx = function (spec) {
    spec = spec || {};
    spec.mode = "done";
    if (spec.ok) {
      spec.title = spec.title || "Sent";
      spec.lede = spec.lede || "On chain. Hash below.";
      spec.rows = (spec.rows || []).slice();
      if (spec.tx) spec.rows.push({ k: "Tx", v: spec.tx });
    } else {
      spec.title = spec.title || "Not sent";
      spec.lede = spec.lede || "Nothing was broadcast, or the chain rejected it.";
    }
    return new Promise(function (resolve) {
      resolveFn = resolve;
      show(spec);
    });
  };

  g.vapurr.beginTx = function (spec) {
    spec = spec || {};
    if (g.vapurr.pendingTx) return Promise.resolve(false);
    return g.vapurr.reviewTx(spec).then(function (ok) {
      if (!ok) return false;
      g.vapurr.pendingTx = spec;
      g.vapurr.waitTx({
        title: spec.title,
        kicker: spec.kicker,
        rows: spec.rows,
        explorer: spec.explorer
      });
      return true;
    });
  };

  g.vapurr.finishTx = function (ok, extra) {
    extra = extra || {};
    var spec = g.vapurr.pendingTx || {};
    g.vapurr.pendingTx = null;
    var tx = extra.tx || "";
    var url = extra.txUrl || extra.tx_url || "";
    if (!url && tx) url = g.vapurr.txUrl(tx, spec.explorer || extra.explorer);
    var already = !!extra.already && !tx;
    if (tx && extra.tx_status !== "confirmed" && extra.tx_status !== "reverted") {
      var chain = Number(extra.tx_chain_id || 46630);
      var attempts = 0;
      function checkReceipt() {
        if (++attempts > 120) return;
        fetch("/wallet/api/transaction/" + chain + "/" + encodeURIComponent(tx), { cache: "no-store" })
          .then(function (r) { return r.json(); }).then(function (res) {
            if (res.tx_status === "confirmed" || res.tx_status === "reverted") {
              g.vapurr.showTx({ ok: res.tx_status === "confirmed", title: res.tx_status === "confirmed" ? (spec.doneTitle || "Confirmed") : "Transaction reverted", tx: tx, txUrl: url, rows: spec.rows || [], lede: res.tx_status === "confirmed" ? "Confirmed on chain." : "The chain reverted this transaction. Network gas may have been spent." });
            } else { setTimeout(checkReceipt, 5000); }
          }).catch(function () { setTimeout(checkReceipt, 5000); });
      }
      setTimeout(checkReceipt, 3000);
      return g.vapurr.showTx({ ok: false, title: "Pending confirmation", tx: tx, txUrl: url, rows: spec.rows || [], lede: "Submitted to the network. Payment is not confirmed. Do not send it again while its receipt is pending." });
    }
    if (extra.tx_status === "reverted") {
      ok = false;
      extra.error = "The chain reverted this transaction. Network gas may have been spent.";
    }
    return g.vapurr.showTx({
      ok: !!ok,
      title: extra.title || (ok ? (already ? "Already on chain" : (spec.doneTitle || "Sent")) : (spec.failTitle || "Not sent")),
      tx: tx,
      txUrl: url,
      error: ok ? "" : (extra.error || extra.msg || "failed"),
      rows: spec.rows || extra.rows || [],
      lede: extra.lede || (ok && already ? "This device already owns it. No new tx." : undefined),
      explorer: spec.explorer
    });
  };

  g.vapurr.signAndWait = function (spec, job) {
    return g.vapurr.beginTx(spec).then(function (go) {
      if (!go) return { rejected: true };
      return Promise.resolve()
        .then(job)
        .then(function (res) {
          res = res || {};
          var rec = res.receipt || {};
          var tx = res.tx || rec.tx || "";
          var url = res.tx_url || res.txUrl || rec.tx_url || "";
          var err = "";
          if (res.ok === false) err = res.error || res.msg || "failed";
          var already = !!(res.already || rec.already);
          var onchain = !!(res.onchain || rec.onchain);
          if (!err && !tx && !already && onchain) already = true;
          var ok = !err && !!(tx || already);
          if (!ok && !err) err = "No hash. Nothing was broadcast.";
          return g.vapurr.finishTx(ok, {
            tx: tx,
            txUrl: url,
            error: err,
            already: already
          }).then(function () { return res; });
        }, function (e) {
          return g.vapurr.finishTx(false, {
            error: String((e && e.message) || e || "failed")
          }).then(function () {
            return { ok: false, error: String((e && e.message) || e || "failed") };
          });
        });
    });
  };
})(window);
