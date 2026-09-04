(function (g) {
  function send(msg) {
    var payload = typeof msg === "string" ? msg : JSON.stringify(msg);
    try {
      if (g.ipc && g.ipc.postMessage) {
        g.ipc.postMessage(payload);
        return true;
      }
      if (g.chrome && g.chrome.webview && g.chrome.webview.postMessage) {
        g.chrome.webview.postMessage(payload);
        return true;
      }
    } catch (e) {}
    return false;
  }

  function go(raw) {
    var u = String(raw || "").trim();
    if (!u) {
      send({ cmd: "go", url: "https://www.google.com" });
      return;
    }
    if (
      u.indexOf("vapurr://") === 0 ||
      u.indexOf("http://") === 0 ||
      u.indexOf("https://") === 0
    ) {
      send({ cmd: "go", url: u });
      return;
    }
    var hood = u.replace(/^@/, "").split(/[/?#]/)[0];
    if (/\.hood$/i.test(hood)) {
      send({ cmd: "go", url: "vapurr://scan?q=" + encodeURIComponent(hood) });
      return;
    }
    if (
      /^0x[0-9a-fA-F]{40}$/.test(u) ||
      /^0x[0-9a-fA-F]{64}$/.test(u) ||
      /^(block|blk|scan)\s*#?:?\s*\d+$/i.test(u)
    ) {
      send({ cmd: "go", url: "vapurr://scan?q=" + encodeURIComponent(u) });
      return;
    }
    var low = u.toLowerCase();
    if (
      low === "fomo" ||
      low === "family" ||
      low === "fomo.family" ||
      low === "live trenches" ||
      low === "trenches" ||
      low === "live-trenches"
    ) {
      send({ cmd: "go", url: "https://fomo.family" });
      return;
    }
    var panes = {
      defi: "defi",
      finance: "defi",
      swap: "swap",
      bridge: "bridge",
      stake: "stake",
      pusd: "pusd",
      $pusd: "pusd",
      vapurr: "pusd",
      mint: "pusd",
      lithe: "lithe",
      vapurrbid: "vapurrbid",
      outbid: "vapurrbid",
      bid: "vapurrbid",
      board: "vapurrbid",
      pns: "pns",
      hood: "pns",
      names: "pns",
      earn: "earn",
      pay: "pay",
      ketpay: "pay",
      wallet: "wallet",
      portfolio: "wallet",
      "robinhood wallet": "wallet",
      scan: "scan",
      explorer: "scan",
      liquidity: "scan?tab=liq",
      liq: "scan?tab=liq",
      dapps: "dapps",
      floor: "floor",
      ketflix: "ketflix",
      ketcharts: "ketcharts",
      charts: "ketcharts",
      chart: "ketcharts",
      settings: "settings",
      history: "history",
      bookmarks: "bookmarks",
      zzzmail: "zmail",
      zmail: "zmail",
      mail: "zmail",
      login: "login",
      "sign in": "login",
      signin: "login"
    };
    if (panes[low]) {
      send({ cmd: "go", url: "vapurr://" + panes[low] });
      return;
    }
    var host = /^[\w.-]+\.[a-z]{2,}([/:?#].*)?$/i.test(u);
    if (host || u === "google" || u === "google.com" || u.indexOf("google.") === 0) {
      send({ cmd: "go", url: "https://" + u.replace(/^\/\//, "") });
      return;
    }
    send({
      cmd: "go",
      url: "https://www.google.com/search?q=" + encodeURIComponent(u),
    });
  }

  document.addEventListener("keydown", function (e) {
    if (g.__vapurrReady) return;
    var c = e.ctrlKey || e.metaKey;
    if (e.altKey && e.key === "ArrowLeft") {
      e.preventDefault();
      send({ cmd: "back" });
      return;
    }
    if (e.altKey && e.key === "ArrowRight") {
      e.preventDefault();
      send({ cmd: "forward" });
      return;
    }
    if (!c) return;
    var k = e.key.toLowerCase();
    var cmds = {
      t: { cmd: "newtab" },
      w: { cmd: "closetab" },
      l: { cmd: "focusurl" },
      r: { cmd: "reload" },
      f: { cmd: "showfind" },
      d: { cmd: "star" },
      h: { cmd: "pane", id: "history" },
      "0": { cmd: "zoomreset" }
    };
    if (k === "+" || k === "=" || e.key === "Add") cmds[k] = { cmd: "zoomin" };
    if (k === "-" || e.key === "Subtract") cmds[k] = { cmd: "zoomout" };
    var msg = cmds[k];
    if (!msg) return;
    e.preventDefault();
    send(msg);
  }, true);

  document.addEventListener("wheel", function (e) {
    if (!(e.ctrlKey || e.metaKey)) return;
    if (g.__vapurrCtrlScroll === false) return;
    e.preventDefault();
    send({ cmd: e.deltaY < 0 ? "zoomin" : "zoomout" });
  }, { capture: true, passive: false });

  function currentPane() {
    try {
      return localStorage.getItem("vapurr.pane") || "";
    } catch (e) {
      return "";
    }
  }

  function markPane() {
    var path = (g.location && g.location.pathname) || "";
    var pane = "web";
    if (path.indexOf("settings.html") >= 0) pane = "settings";
    else if (path.indexOf("shield.html") >= 0) pane = "shield";
    else if (path.indexOf("id.html") >= 0) pane = "id";
    else if (path.indexOf("ketbook") >= 0) pane = "ketbook";
    else if (path.indexOf("home.html") >= 0) pane = "home";
    else {
      var m = path.match(/\/([a-z0-9-]+)\.html$/i);
      if (m) pane = m[1];
    }
    try {
      localStorage.setItem("vapurr.pane", pane);
    } catch (e) {}
  }

  function togglePane(id) {
    var pane = currentPane();
    if (id === "settings" && pane === "settings") {
      send({ cmd: "home" });
      return;
    }
    if ((id === "shield" || id === "adblock") && pane === "shield") {
      send({ cmd: "home" });
      return;
    }
    if (id === "id" && pane === "id") {
      send({ cmd: "home" });
      return;
    }
    if (id === "settings") send({ cmd: "settings" });
    else send({ cmd: "pane", id: id });
  }

  function themeName() {
    try {
      return localStorage.getItem("vapurr.theme") === "light" ? "light" : "dark";
    } catch (e) {
      return "dark";
    }
  }

  var themeBc = null;
  try {
    themeBc = new BroadcastChannel("vapurr-theme");
  } catch (e) {}

  function applyTheme(t, broadcast) {
    t = t === "light" ? "light" : "dark";
    document.documentElement.setAttribute("data-theme", t);
    try {
      localStorage.setItem("vapurr.theme", t);
    } catch (e) {}
    if (broadcast !== false && themeBc) {
      try {
        themeBc.postMessage(t);
      } catch (e) {}
    }
    if (g.__onTheme) g.__onTheme(t);
  }

  g.__applyTheme = function (t) {
    applyTheme(t, false);
  };

  applyTheme(themeName(), false);
  if (themeBc) {
    themeBc.onmessage = function (ev) {
      applyTheme(ev.data, false);
    };
  }
  g.addEventListener("storage", function (e) {
    if (e.key === "vapurr.theme") applyTheme(e.newValue || "dark", false);
  });
  markPane();

  g.vapurr = {
    send: send,
    go: go,
    togglePane: togglePane,
    theme: themeName,
    setTheme: function (t) {
      applyTheme(t, true);
      send({ cmd: "pref", key: "theme", value: t === "light" ? "light" : "dark" });
    },
    toggleTheme: function () {
      g.vapurr.setTheme(themeName() === "light" ? "dark" : "light");
    }
  };
})(window);
