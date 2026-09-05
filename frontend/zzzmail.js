(function () {
  var KEY = "vapurr-zzzmail-v1";
  var OLD_KEY = "vapurr-zmail-v1";
  var app = document.getElementById("app");
  var convos = document.getElementById("convos");
  var threadEl = document.getElementById("thread");
  var thead = document.getElementById("thead");
  var tava = document.getElementById("tava");
  var thandle = document.getElementById("thandle");
  var msgs = document.getElementById("msgs");
  var body = document.getElementById("body");
  var go = document.getElementById("go");
  var tobar = document.getElementById("tobar");
  var to = document.getElementById("to");
  var q = document.getElementById("q");
  var toastEl = document.getElementById("toast");
  var composing = false;
  var active = null;
  var filter = "";
  var sending = false;
  var quote = { label: "0.25¢ $PUSD", gasless: true, note: "voucher · 0 ETH · body is an IPFS CID" };
  var me = { handle: "", address: "", x25519: "" };

  function load() {
    try {
      var raw = localStorage.getItem(KEY);
      if (!raw) {
        raw = localStorage.getItem(OLD_KEY);
        if (raw) {
          localStorage.setItem(KEY, raw);
        }
      }
      var data = raw ? JSON.parse(raw) : { threads: [] };
      if (!data || !Array.isArray(data.threads)) return { threads: [] };
      return data;
    } catch (e) {
      return { threads: [] };
    }
  }
  function save(data) {
    try { localStorage.setItem(KEY, JSON.stringify(data)); } catch (e) {}
  }
  function store() {
    return load();
  }
  function uid() {
    return "m" + Date.now().toString(36) + Math.random().toString(36).slice(2, 7);
  }
  function parseTo(s) {
    s = String(s || "").trim().replace(/^@+/, "").trim();
    if (!s) return null;
    if (/^0x[0-9a-fA-F]{40}$/.test(s)) {
      return { kind: "address", key: s.toLowerCase() };
    }
    if (/^0x/i.test(s)) return null;
    var h = s.toLowerCase();
    if (/\.hood$/.test(h)) {
      var label = h.slice(0, -5);
      if (!/^[a-z0-9]([a-z0-9-]{1,30}[a-z0-9])?$/.test(label) || label.length < 3) return null;
      return { kind: "hood", key: label + ".hood" };
    }
    if (!/^[a-z0-9._-]{3,32}$/.test(h) || h.indexOf(".hood") >= 0) return null;
    return { kind: "handle", key: h };
  }
  function at(key) {
    key = String(key || "");
    if (/^0x[0-9a-f]{40}$/.test(key)) return "@" + key.slice(0, 6) + "…" + key.slice(-4);
    return "@" + key;
  }
  function letter(key) {
    key = String(key || "");
    if (/^0x[0-9a-f]{40}$/.test(key)) return key.slice(2, 4).toUpperCase();
    return (key.charAt(0) || "?").toUpperCase();
  }
  function ago(ts) {
    var d = Math.max(0, Date.now() - Number(ts || 0));
    if (d < 45000) return "now";
    if (d < 3600000) return Math.floor(d / 60000) + "m";
    if (d < 86400000) return Math.floor(d / 3600000) + "h";
    return Math.floor(d / 86400000) + "d";
  }
  function dayLabel(ts) {
    var dt = new Date(ts);
    var now = new Date();
    if (dt.toDateString() === now.toDateString()) return "Today";
    var y = new Date(now);
    y.setDate(now.getDate() - 1);
    if (dt.toDateString() === y.toDateString()) return "Yesterday";
    return dt.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }
  function toast(msg) {
    toastEl.textContent = msg;
    toastEl.classList.add("on");
    clearTimeout(toast._t);
    toast._t = setTimeout(function () { toastEl.classList.remove("on"); }, 1600);
  }
  function lastOf(t) {
    var m = (t.messages || [])[(t.messages || []).length - 1];
    return m || null;
  }
  function sorted(data) {
    return (data.threads || []).slice().sort(function (a, b) {
      var am = lastOf(a); var bm = lastOf(b);
      return Number((bm && bm.ts) || 0) - Number((am && am.ts) || 0);
    });
  }
  function findThread(data, handle) {
    handle = String(handle || "").toLowerCase();
    for (var i = 0; i < data.threads.length; i++) {
      if (data.threads[i].handle === handle) return data.threads[i];
    }
    return null;
  }
  function setHash(h) {
    var next = h ? "#/" + h : "#/";
    if (location.hash !== next) location.hash = next;
  }

  function paintList() {
    var data = store();
    var rows = sorted(data);
    if (filter) {
      var f = filter.toLowerCase();
      rows = rows.filter(function (t) {
        var last = lastOf(t);
        return t.handle.indexOf(f) >= 0
          || at(t.handle).toLowerCase().indexOf(f) >= 0
          || (last && String(last.text || "").toLowerCase().indexOf(f) >= 0);
      });
    }
    convos.innerHTML = "";
    if (!rows.length) {
      var empty = document.createElement("div");
      empty.className = "list-empty";
      empty.textContent = filter ? "No threads match." : "Quiet in here. Compose starts a thread.";
      convos.appendChild(empty);
      return;
    }
    rows.forEach(function (t) {
      var last = lastOf(t);
      var b = document.createElement("button");
      b.type = "button";
      b.className = "convo" + (active === t.handle ? " on" : "");
      b.setAttribute("role", "option");
      b.setAttribute("aria-selected", active === t.handle ? "true" : "false");
      var av = document.createElement("div");
      av.className = "ava";
      av.textContent = letter(t.handle);
      var mid = document.createElement("div");
      var who = document.createElement("div");
      who.className = "who";
      who.textContent = at(t.handle);
      who.title = t.handle.indexOf("0x") === 0 ? t.handle : at(t.handle);
      var prev = document.createElement("div");
      prev.className = "prev";
      prev.textContent = last ? last.text : "No messages";
      mid.appendChild(who);
      mid.appendChild(prev);
      var when = document.createElement("div");
      when.className = "when";
      when.textContent = last ? ago(last.ts) : "";
      b.appendChild(av);
      b.appendChild(mid);
      b.appendChild(when);
      b.onclick = function () { openThread(t.handle, false); };
      convos.appendChild(b);
    });
  }

  function groupKind(list, i) {
    var m = list[i];
    var prev = list[i - 1];
    var next = list[i + 1];
    var samePrev = prev && prev.me === m.me && Math.abs(m.ts - prev.ts) < 120000;
    var sameNext = next && next.me === m.me && Math.abs(next.ts - m.ts) < 120000;
    if (!samePrev && !sameNext) return "solo";
    if (!samePrev && sameNext) return "first";
    if (samePrev && sameNext) return "mid";
    return "last";
  }

  function paintBlank(title, copy) {
    msgs.innerHTML = "";
    var blank = document.createElement("div");
    blank.className = "blank";
    var img = document.createElement("img");
    img.className = "hero";
    img.src = "/zzzmail-icon.png";
    img.alt = "";
    var h = document.createElement("h2");
    h.textContent = title;
    var p = document.createElement("p");
    p.textContent = copy;
    blank.appendChild(img);
    blank.appendChild(h);
    blank.appendChild(p);
    msgs.appendChild(blank);
  }

  function paintThread() {
    msgs.innerHTML = "";
    if (!active && !composing) {
      thead.hidden = true;
      threadEl.classList.add("idle");
      tobar.classList.remove("on");
      paintBlank("zzzmail", "Send to @name.hood or @0x. Names are ours — TLD .hood. Sealed body is a CID. 0.25¢ · 0 ETH.");
      return;
    }
    threadEl.classList.remove("idle");
    thead.hidden = false;
    var handle = active || "";
    tava.textContent = handle ? letter(handle) : "+";
    thandle.textContent = handle ? at(handle) : "New message";
    thandle.title = handle && handle.indexOf("0x") === 0 ? handle : "";
    var tsub = document.getElementById("tsub");
    if (tsub) {
      var label = handle && /^0x[0-9a-f]{40}$/.test(handle)
        ? "Encrypted to a 0x · CID off-chain"
        : /\.hood$/.test(handle)
          ? "Encrypted to a .hood name · CID off-chain"
          : "Encrypted · CID off-chain · 0.25¢ postage";
      var lock = tsub.querySelector(".lock");
      if (lock) lock = lock.cloneNode(true);
      tsub.textContent = "";
      if (lock) tsub.appendChild(lock);
      tsub.appendChild(document.createTextNode(" " + label));
    }
    tobar.classList.toggle("on", composing && !active);
    var data = store();
    var t = handle ? findThread(data, handle) : null;
    var list = (t && t.messages) || [];
    if (!list.length) {
      paintBlank(
        handle ? at(handle) : "New message",
        handle
          ? "Write a letter. Sealed to their mailcard, pinned, 0.25¢ voucher."
          : "Address @name.hood or @0x, then write."
      );
      return;
    }
    var lastDay = "";
    list.forEach(function (m, i) {
      var day = dayLabel(m.ts);
      if (day !== lastDay) {
        lastDay = day;
        var cap = document.createElement("div");
        cap.className = "day";
        cap.textContent = day;
        msgs.appendChild(cap);
      }
      var row = document.createElement("div");
      row.className = "row " + (m.me ? "me" : "them") + " " + groupKind(list, i);
      var bub = document.createElement("div");
      bub.className = "bubble";
      bub.textContent = m.text;
      row.appendChild(bub);
      if (i === list.length - 1 || groupKind(list, i) === "last" || groupKind(list, i) === "solo") {
        var st = document.createElement("div");
        st.className = "stamp";
        st.textContent = stampLine(m);
        row.appendChild(st);
      }
      msgs.appendChild(row);
    });
    msgs.scrollTop = msgs.scrollHeight;
  }

  function openThread(handle, isNew) {
    composing = !!isNew;
    active = handle || null;
    app.classList.toggle("show-thread", true);
    setHash(active || (composing ? "new" : ""));
    paintList();
    paintThread();
    if (composing && !active) {
      to.value = "";
      to.focus();
    } else {
      body.focus();
    }
  }

  function idle() {
    composing = false;
    active = null;
    app.classList.remove("show-thread");
    setHash("");
    paintList();
    paintThread();
  }

  function resizeBody() {
    body.style.height = "22px";
    body.style.height = Math.min(120, body.scrollHeight) + "px";
    go.disabled = !String(body.value || "").trim();
  }

  function stampLine(m) {
    if (!m.me) return ago(m.ts);
    var bits = [];
    if (m.postage) bits.push(m.postage);
    else if (m.state === "sent") bits.push(quote.label);
    if (m.gasless !== false) bits.push("0 ETH");
    if (m.pin === "ipfs") bits.push("IPFS");
    else if (m.pin === "relay") bits.push("relay");
    else if (m.cid) bits.push("pinned");
    else bits.push("queued");
    bits.push(ago(m.ts));
    return bits.join("  ·  ");
  }

  function otherOf(item) {
    var from = String(item.from || "").replace(/^@+/, "").toLowerCase();
    var toKey = String(item.to || "").replace(/^@+/, "").toLowerCase();
    var mine = String(me.handle || "").toLowerCase();
    var addr = String(me.address || "").toLowerCase();
    if (item.me) return toKey;
    if ((mine && from === mine) || (addr && from === addr)) return toKey;
    return from;
  }

  function mergeLetter(item) {
    if (!item || !item.body) return;
    var handle = otherOf(item);
    if (!handle) return;
    var parsed = parseTo(handle) || { key: handle, kind: /^0x/.test(handle) ? "address" : "handle" };
    var data = store();
    var t = findThread(data, parsed.key);
    if (!t) {
      t = { handle: parsed.key, kind: parsed.kind, messages: [] };
      data.threads.push(t);
    }
    var cid = item.cid || "";
    if (cid && t.messages.some(function (m) { return m.cid === cid; })) {
      save(data);
      return;
    }
    t.messages.push({
      id: cid || uid(),
      me: !!item.me,
      text: item.body,
      ts: Number(item.ts) ? Number(item.ts) * (String(item.ts).length < 12 ? 1000 : 1) : Date.now(),
      state: cid ? "sent" : "local",
      cid: cid,
      pin: item.pin || "",
      postage: item.postage || (item.me ? quote.label : ""),
      gasless: item.gasless !== false
    });
    save(data);
  }

  function paintMe() {
    var k = document.querySelector(".kicker");
    var bar = document.getElementById("namebar");
    var who = me.hood ? ("@" + me.hood) : (me.handle ? ("@" + me.handle) : "@name.hood");
    if (k) k.textContent = "PNS · " + (quote.label || "0.25¢ $PUSD") + " · " + who;
    if (bar) bar.classList.toggle("on", !me.hood);
  }

  function onHood(res) {
    if (res.name) me.hood = res.name;
    if (res.owner) me.address = res.owner;
    paintMe();
    if (hoodQuiet) return;
    if (res.tx) {
      toast((res.name || "name") + " · " + res.tx.slice(0, 10) + "…");
      return;
    }
    toast((res.name || "name") + (res.onchain ? " · on-chain PNS" : " · local PNS"));
  }

  function applyMail(res) {
    if (!res) return;
    if (res.me) me = res.me;
    if (res.hood && res.hood.primary) me.hood = res.hood.primary;
    if (res.pns && res.pns.primary) me.hood = res.pns.primary;
    if (res.quote) quote = res.quote;
    if (res.pusd) quote = res.pusd;
    paintMe();
    if (Array.isArray(res.inbox)) {
      res.inbox.forEach(mergeLetter);
      paintList();
      paintThread();
    }
    if (res.just_registered || (res.kind === "hood" && res.name) || (res.receipt && res.receipt.name)) {
      var rec = res.receipt || res;
      if (res.primary) rec.name = rec.name || res.primary;
      onHood(rec);
      if (!hoodQuiet && rec.need_gas) toast((rec.name || "name") + " claimed here. Chain needs testnet ETH.");
      return;
    }
    if (res.ok && res.primary && !res.cid) {
      me.hood = res.primary;
      paintMe();
    }
    if (res.ok && res.cid) onSent(res);
    else if (res.ok === false) {
      sending = false;
      go.disabled = !String(body.value || "").trim();
      if (hoodQuiet) return;
      var msg = res.error || "Send failed";
      if (res.error === "unknown") msg = "This vapurr is old. Restart it.";
      else if (res.need_name) msg = "Unknown PNS name.";
      else if (res.taken) msg = "That .hood is taken.";
      else if (res.reserved) msg = "That .hood is reserved.";
      else if (res.need_gas) msg = "Need testnet ETH for PNS. " + (res.faucet || "faucet.testnet.chain.robinhood.com");
      else if (res.need_card) msg = "They need zzzmail once for a mailcard.";
      toast(msg);
    }
  }

  function onSent(res) {
    sending = false;
    body.value = "";
    resizeBody();
    mergeLetter({
      cid: res.cid,
      from: res.from,
      to: res.to,
      body: res.body,
      ts: res.ts,
      me: true,
      pin: res.pin,
      postage: res.postage && res.postage.label,
      gasless: !!(res.postage && res.postage.gasless)
    });
    var parsed = parseTo(String(res.to || "").replace(/^@+/, ""));
    if (parsed) {
      composing = false;
      active = parsed.key;
      tobar.classList.remove("on");
      setHash(parsed.key);
    }
    paintList();
    paintThread();
    var label = (res.postage && res.postage.label) || quote.label;
    var pin = res.pin === "ipfs" ? "IPFS" : (res.pin === "relay" ? "relay" : "pinned");
    toast(label + " · 0 ETH · " + pin);
  }

  function deliver(handle, text) {
    var payload = { to: "@" + handle, body: text, asset: "PUSD" };
    fetch("/zzzmail/api/send", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(payload),
      cache: "no-store"
    }).then(function (r) { return r.json(); }).then(function (res) {
      if (res && res.ok && res.cid) { applyMail(res); return; }
      if (res && res.ok === false && res.error && res.error !== "unknown") { applyMail(res); return; }
      throw new Error("fallback");
    }).catch(function () {
      if (window.vapurr && window.vapurr.send) {
        window.vapurr.send({ cmd: "zzzmail-send", to: "@" + handle, body: text, asset: "PUSD" });
        setTimeout(function () {
          if (!sending) return;
          sending = false;
          go.disabled = !String(body.value || "").trim();
          toast("This build still parks send. Restart vapurr.");
        }, 1400);
        return;
      }
      sending = false;
      go.disabled = !String(body.value || "").trim();
      toast("This build still parks send. Restart vapurr.");
    });
  }

  function send() {
    var text = String(body.value || "").trim();
    if (!text || sending) return;
    var parsed = active ? { key: active } : parseTo(to.value);
    if (!parsed || !parsed.key) {
      toast("Need @name.hood or @0x…");
      if (!active) to.focus();
      return;
    }
    sending = true;
    go.disabled = true;
    deliver(parsed.key, text);
  }

  document.getElementById("compose").onclick = function () {
    openThread(null, true);
  };
  document.getElementById("back").onclick = function () { idle(); };
  document.getElementById("composer").addEventListener("submit", function (e) {
    e.preventDefault();
    send();
  });
  body.addEventListener("input", resizeBody);
  body.addEventListener("keydown", function (e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  });
  to.addEventListener("keydown", function (e) {
    if (e.key === "Enter") {
      e.preventDefault();
      var parsed = parseTo(to.value);
      if (!parsed) { toast("Need @name.hood or @0x…"); return; }
      active = parsed.key;
      composing = false;
      tobar.classList.remove("on");
      setHash(parsed.key);
      paintList();
      paintThread();
      body.focus();
    }
  });
  q.addEventListener("input", function () {
    filter = q.value.trim();
    paintList();
  });
  var hoodQuiet = false;
  function claimHood(raw) {
    raw = String(raw || "").trim();
    if (!raw) { toast("Pick a .hood name."); return; }
    if (!window.vapurr || !window.vapurr.signAndWait) {
      toast("Restart vapurr, then claim again.");
      return;
    }
    if (vapurr.pendingTx) return;
    var label = raw.replace(/^@+/, "").replace(/\.hood$/i, "");
    vapurr.signAndWait({
      title: "Claim " + label + ".hood",
      kicker: "This device signs",
      lede: "PNS on Robinhood testnet 46630. You get a hash, or it failed.",
      rows: [
        { k: "Network", v: "Robinhood Chain Testnet 46630" },
        { k: "Name", v: label + ".hood" },
        { k: "From", v: me.address || "this device" }
      ],
      confirmLabel: "Sign and claim",
      doneTitle: "Claimed",
      failTitle: "Not claimed",
      explorer: "https://explorer.testnet.chain.robinhood.com"
    }, function () {
      var path = "/zzzmail/api/pns/register/" + encodeURIComponent(label);
      return fetch(path, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ name: raw }),
        cache: "no-store"
      }).then(function (r) { return r.json(); }).then(function (res) {
        if (res && res.error === "unknown") {
          return fetch("/zzzmail/api/hood/register/" + encodeURIComponent(label), { method: "POST", cache: "no-store" })
            .then(function (r) { return r.json(); });
        }
        return res;
      });
    }).then(function (res) {
      if (!res || res.rejected) return;
      hoodQuiet = true;
      applyMail(res);
      hoodQuiet = false;
    });
  }
  document.getElementById("namebar").addEventListener("submit", function (e) {
    e.preventDefault();
    claimHood(document.getElementById("hoodname").value);
  });

  function route() {
    var raw = (location.hash || "").replace(/^#\/?/, "");
    if (!raw || raw === "new") {
      if (raw === "new") openThread(null, true);
      else {
        composing = false;
        active = null;
        app.classList.remove("show-thread");
        paintList();
        paintThread();
      }
      return;
    }
    var parsed = parseTo(decodeURIComponent(raw));
    if (parsed) openThread(parsed.key, false);
    else idle();
  }
  window.addEventListener("hashchange", route);
  window.__setMail = applyMail;
  function refreshInbox() {
    fetch("/zzzmail/api/inbox", { cache: "no-store" }).then(function (r) { return r.json(); }).then(applyMail).catch(function () {});
    if (window.vapurr && window.vapurr.send) window.vapurr.send({ cmd: "zzzmail-inbox" });
  }
  fetch("/zzzmail/api/quote", { cache: "no-store" }).then(function (r) { return r.json(); }).then(applyMail).catch(function () {});
  refreshInbox();
  setInterval(refreshInbox, 8000);
  resizeBody();
  route();
})();
