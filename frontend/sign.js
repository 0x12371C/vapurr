(function (g) {
  var root = null;
  var mode = "review";
  var resolveFn = null;
  var slideApi = null;

  function el(tag, cls, text) {
    var n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text != null) n.textContent = text;
    return n;
  }

  function prefersReduce() {
    try { return !!(g.matchMedia && g.matchMedia("(prefers-reduced-motion: reduce)").matches); }
    catch (e) { return false; }
  }

  /* Shared slide / hold confirm — used by sign sheet + route #go */
  g.vapurr = g.vapurr || {};
  g.vapurr.bindSlideHold = function (btn, opts) {
    opts = opts || {};
    if (!btn || btn._slideHold) return btn._slideHold;
    var reduce = prefersReduce();
    var threshold = opts.threshold != null ? opts.threshold : 0.88;
    var holdMs = opts.holdMs != null ? opts.holdMs : (reduce ? 520 : 780);
    var onConfirm = typeof opts.onConfirm === "function" ? opts.onConfirm : function () {};
    var filling = false;
    var dragging = false;
    var armed = false;
    var startX = 0;
    var startP = 0;
    var progress = 0;
    var holdTimer = 0;
    var holdT0 = 0;
    var ptrId = null;

    if (!btn.querySelector(".sh-lab")) {
      var labText = (btn.textContent || opts.label || "Confirm").trim();
      btn.textContent = "";
      btn.classList.add("slide-hold");
      btn.innerHTML =
        '<span class="sh-fill" aria-hidden="true"></span>' +
        '<span class="sh-knob" aria-hidden="true"><svg class="sh-chev" viewBox="0 0 24 24" fill="none"><path d="M9 6l6 6-6 6" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/></svg></span>' +
        '<span class="sh-lab"></span>' +
        '<span class="sh-hint" aria-hidden="true"></span>';
      btn.querySelector(".sh-lab").textContent = labText;
    }
    btn.classList.add("slide-hold");
    var fill = btn.querySelector(".sh-fill");
    var knob = btn.querySelector(".sh-knob");
    var lab = btn.querySelector(".sh-lab");
    var hint = btn.querySelector(".sh-hint");
    if (hint) hint.textContent = reduce ? "hold" : "slide or hold";

    function trackW() {
      var pad = 6;
      var kw = knob ? knob.offsetWidth : 40;
      return Math.max(1, btn.clientWidth - kw - pad * 2);
    }
    function paint() {
      var p = Math.max(0, Math.min(1, progress));
      if (fill) fill.style.transform = "scaleX(" + p + ")";
      if (knob) knob.style.transform = "translate3d(" + (p * trackW()) + "px,0,0)";
      btn.style.setProperty("--sh-p", String(p));
      btn.classList.toggle("sh-hot", p > 0.08);
      btn.classList.toggle("sh-ready", p >= threshold);
      if (hint) {
        if (btn.disabled) hint.textContent = "";
        else if (p >= threshold) hint.textContent = "release";
        else hint.textContent = reduce ? "hold" : "slide or hold";
      }
    }
    function setProgress(p) {
      progress = Math.max(0, Math.min(1, p));
      paint();
    }
    function reset() {
      clearInterval(holdTimer);
      holdTimer = 0;
      filling = false;
      dragging = false;
      armed = false;
      ptrId = null;
      btn.classList.remove("sh-active", "sh-done");
      setProgress(0);
    }
    var fired = false;
    function fire() {
      if (btn.disabled || fired) { reset(); return; }
      fired = true;
      clearInterval(holdTimer);
      holdTimer = 0;
      filling = false;
      armed = false;
      dragging = false;
      ptrId = null;
      setProgress(1);
      btn.classList.add("sh-done");
      btn.classList.remove("sh-active");
      try { onConfirm(); } finally {
        setTimeout(function () {
          btn.classList.remove("sh-done");
          fired = false;
          reset();
        }, reduce ? 80 : 220);
      }
    }
    function startHold() {
      if (btn.disabled || filling) return;
      filling = true;
      holdT0 = performance.now();
      btn.classList.add("sh-active");
      clearInterval(holdTimer);
      holdTimer = setInterval(function () {
        var t = (performance.now() - holdT0) / holdMs;
        setProgress(Math.min(1, t));
        if (t >= 1) {
          clearInterval(holdTimer);
          holdTimer = 0;
          fire();
        }
      }, reduce ? 40 : 16);
    }
    function onPtrDown(e) {
      if (btn.disabled || (e.button != null && e.button !== 0)) return;
      if (btn.classList.contains("sh-tap")) return;
      e.preventDefault();
      try { btn.setPointerCapture(e.pointerId); } catch (err) {}
      ptrId = e.pointerId;
      startX = e.clientX;
      startP = progress;
      dragging = false;
      armed = true;
      btn.classList.add("sh-active");
      startHold();
    }
    function onPtrMove(e) {
      if (!armed || (ptrId != null && e.pointerId !== ptrId)) return;
      if (reduce) return;
      var dx = e.clientX - startX;
      if (Math.abs(dx) > 6) {
        dragging = true;
        clearInterval(holdTimer);
        holdTimer = 0;
        filling = false;
        setProgress(startP + dx / trackW());
      }
    }
    function onPtrUp(e) {
      if (!armed || (ptrId != null && e.pointerId !== ptrId)) return;
      try { btn.releasePointerCapture(e.pointerId); } catch (err) {}
      var p = progress;
      clearInterval(holdTimer);
      holdTimer = 0;
      filling = false;
      armed = false;
      dragging = false;
      ptrId = null;
      btn.classList.remove("sh-active");
      if (p >= threshold) fire();
      else reset();
    }
    function onPtrCancel() {
      if (!armed) return;
      reset();
    }
    function onKeyDown(e) {
      if (btn.disabled || btn.classList.contains("sh-tap")) return;
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        if (e.repeat) return;
        startHold();
      }
    }
    function onKeyUp(e) {
      if (btn.classList.contains("sh-tap")) return;
      if (e.key === "Enter" || e.key === " ") {
        if (progress >= threshold) fire();
        else reset();
      }
    }

    btn.addEventListener("pointerdown", onPtrDown);
    btn.addEventListener("pointermove", onPtrMove);
    btn.addEventListener("pointerup", onPtrUp);
    btn.addEventListener("pointercancel", onPtrCancel);
    btn.addEventListener("lostpointercapture", onPtrCancel);
    btn.addEventListener("keydown", onKeyDown);
    btn.addEventListener("keyup", onKeyUp);
    btn.addEventListener("contextmenu", function (e) { e.preventDefault(); });

    var api = {
      setLabel: function (t) {
        if (lab) lab.textContent = t;
        btn.setAttribute("aria-label", t);
      },
      setDisabled: function (d) {
        btn.disabled = !!d;
        if (d) reset();
        else paint();
      },
      reset: reset,
      el: btn
    };
    btn._slideHold = api;
    paint();
    return api;
  };

  function ensureInk(panel) {
    var ink = panel.querySelector(".vapurr-sign-ink");
    if (ink) return ink;
    ink = el("div", "vapurr-sign-ink");
    ink.hidden = true;
    ink.setAttribute("aria-hidden", "true");
    ink.innerHTML =
      '<div class="vsi-stage">' +
        '<svg class="vsi-pen" viewBox="0 0 48 48" fill="none" aria-hidden="true">' +
          '<path d="M34.5 6.5l7 7-22 22-9.2 2.2 2.2-9.2 22-22z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>' +
          '<path class="vsi-nib" d="M12.5 35.5l-2.2 9.2 9.2-2.2" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/>' +
        '</svg>' +
        '<svg class="vsi-stroke" viewBox="0 0 280 72" fill="none" aria-hidden="true">' +
          '<path class="vsi-path" d="M18 48 C 48 18, 72 58, 98 34 S 148 12, 172 40 S 222 62, 262 28" ' +
            'stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>' +
          '<path class="vsi-hash" d="M40 60h28M78 60h22M118 60h34M170 60h26M214 60h30" ' +
            'stroke="currentColor" stroke-width="1.4" stroke-linecap="round" opacity="0.45"/>' +
        '</svg>' +
        '<div class="vsi-bits" aria-hidden="true">' +
          '<i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>' +
        '</div>' +
      '</div>' +
      '<div class="vsi-cap mono">Signing on this device</div>';
    var lede = panel.querySelector("#vs-lede");
    if (lede && lede.parentNode) lede.parentNode.insertBefore(ink, lede.nextSibling);
    else panel.appendChild(ink);
    return ink;
  }

  function setInk(on) {
    if (!root) return;
    var panel = root.querySelector(".vapurr-sign-panel");
    if (!panel) return;
    var ink = ensureInk(panel);
    ink.hidden = !on;
    ink.classList.toggle("on", !!on);
    root.classList.toggle("is-signing", !!on);
    if (on && !prefersReduce()) {
      ink.classList.remove("vsi-replay");
      void ink.offsetWidth;
      ink.classList.add("vsi-replay");
    }
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
      '<button class="btn slide-hold" type="button" id="vs-go" aria-label="Sign">' +
        '<span class="sh-fill" aria-hidden="true"></span>' +
        '<span class="sh-knob" aria-hidden="true"><svg class="sh-chev" viewBox="0 0 24 24" fill="none"><path d="M9 6l6 6-6 6" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/></svg></span>' +
        '<span class="sh-lab">Sign</span>' +
        '<span class="sh-hint" aria-hidden="true">slide or hold</span>' +
      '</button>' +
      '<button class="btn vapurr-sign-ghost" type="button" id="vs-no">Reject</button>' +
      "</div>";
    document.body.appendChild(root);
    root.querySelector(".vapurr-sign-dim").onclick = reject;
    root.querySelector("#vs-no").onclick = function () {
      if (mode === "done") close(true);
      else reject();
    };
    slideApi = g.vapurr.bindSlideHold(root.querySelector("#vs-go"), {
      onConfirm: accept
    });
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
    if (!slideApi) slideApi = g.vapurr.bindSlideHold(go, { onConfirm: accept });
    if (mode === "done") {
      setInk(false);
      go.classList.add("sh-tap");
      slideApi.setLabel(spec.txUrl ? "Open explorer" : "Done");
      slideApi.setDisabled(false);
      go.onclick = accept;
      no.textContent = spec.txUrl ? "Done" : "Close";
    } else if (mode === "wait") {
      setInk(true);
      go.classList.remove("sh-tap");
      go.onclick = null;
      slideApi.setLabel("Signing…");
      slideApi.setDisabled(true);
      no.textContent = "Hide";
    } else {
      setInk(false);
      go.classList.remove("sh-tap");
      go.onclick = null;
      slideApi.setLabel(spec.confirmLabel || "Sign and send");
      slideApi.setDisabled(false);
      slideApi.reset();
      no.textContent = "Reject";
    }
    root._spec = spec;
  }

  function close(ok) {
    if (!root) return;
    setInk(false);
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
