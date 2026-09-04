(function (g) {
  var LIME = "#c0f800";
  var RED = "#e50914";
  var STOCKS = {
    AAPL: 1, AMZN: 1, TSLA: 1, NVDA: 1, META: 1, GOOGL: 1, GOOG: 1, MSFT: 1,
    HOOD: 1, NFLX: 1, AMD: 1, INTC: 1, COST: 1, SPY: 1, QQQ: 1, COIN: 1,
    PLTR: 1, SOFI: 1, BA: 1, DIS: 1, V: 1, MA: 1, JPM: 1, GS: 1, NET: 1,
    CRM: 1, ADBE: 1, ORCL: 1, IBM: 1, UBER: 1, ABNB: 1, SNOW: 1, MSTR: 1
  };
  var TFS = [
    { id: "30s", lab: "30s", gecko: "second", agg: 30, paprika: "", ms: 3e4, fromTrades: true },
    { id: "1", lab: "1m", gecko: "minute", agg: 1, paprika: "1m", ms: 6e4 },
    { id: "5", lab: "5m", gecko: "minute", agg: 5, paprika: "5m", ms: 3e5 },
    { id: "15", lab: "15m", gecko: "minute", agg: 15, paprika: "15m", ms: 9e5 },
    { id: "30", lab: "30m", gecko: "minute", agg: 15, paprika: "30m", ms: 18e5, bucketMs: 18e5 },
    { id: "60", lab: "1h", gecko: "hour", agg: 1, paprika: "1h", ms: 36e5 },
    { id: "240", lab: "4h", gecko: "hour", agg: 4, paprika: "1h", ms: 144e5, paprikaMs: 36e5 },
    { id: "D", lab: "1D", gecko: "day", agg: 1, paprika: "24h", ms: 864e5 },
    { id: "W", lab: "1W", gecko: "day", agg: 1, paprika: "24h", ms: 6048e5, bucketMs: 6048e5 },
    { id: "M", lab: "1M", gecko: "day", agg: 1, paprika: "24h", ms: 2592e6, bucketMs: 2592e6 }
  ];
  var INDS = [
    { id: "ma9", type: "moving-average", lab: "MA9", inputs: { maType: "SMA", length: 9 } },
    { id: "ma21", type: "moving-average", lab: "MA21", inputs: { maType: "SMA", length: 21 } },
    { id: "ema", type: "moving-average", lab: "EMA", inputs: { maType: "EMA", length: 20 } },
    { id: "bb", type: "bollinger-bands", lab: "BB", inputs: {} },
    { id: "vwap", type: "vwap", lab: "VWAP", inputs: { anchor: "Day" } },
    { id: "st", type: "supertrend", lab: "ST", inputs: { atrLength: 10, mult: 3 } },
    { id: "rsi", type: "rsi", lab: "RSI", inputs: { length: 14 } },
    { id: "macd", type: "macd", lab: "MACD", inputs: { fastLength: 12, slowLength: 26, signalLength: 9 } },
    { id: "atr", type: "average-true-range", lab: "ATR", inputs: { length: 14 } }
  ];
  var STYLES = [
    { id: "candles", lab: "Cdls" },
    { id: "bars", lab: "OHLC" },
    { id: "line", lab: "Line" },
    { id: "area", lab: "Area" },
    { id: "heikinashi", lab: "HA" }
  ];
  var RANGES = ["1D", "1W", "1M", "ALL"];
  var MAJORS = [
    { id: "binance:ETHUSDT", lab: "ETH", kind: "cash" },
    { id: "binance:BTCUSDT", lab: "BTC", kind: "cash" },
    { id: "binance:SOLUSDT", lab: "SOL", kind: "cash" }
  ];
  var STOCK_BOOT = ["NVDA", "AAPL", "HOOD", "TSLA"];
  var PAPER_KEY = "vapurr.ketcharts.paper";
  var IND_KEY = "vapurr.ketcharts.inds";
  var SORT_KEY = "vapurr.ketcharts.sort";
  var FAV_KEY = "vapurr.ketcharts.fav";
  var ALERT_KEY = "vapurr.ketcharts.alerts";
  var PREF_KEY = "vapurr.ketcharts.pref";
  var tab = "stock";
  var q = "";
  var pairs = [];
  var selected = null;
  var timeframe = "60";
  var chart = null;
  var feed = null;
  var lastClose = 0;
  var lastBars = [];
  var lastTape = "";
  var bootGen = 0;
  var barCache = {};
  var searchTimer = 0;
  var flashTimer = 0;
  var sortKey = "vol";
  var sortDir = -1;
  var activeInds = loadInds();
  var indHandles = {};
  var paper = loadPaper();
  var favs = loadFavs();
  var alerts = loadAlerts();
  var priceStyle = "candles";
  var logScale = false;
  var pctScale = false;
  var rangePreset = "";
  var lastPool = "";
  (function loadPref() {
    try {
      var raw = JSON.parse(localStorage.getItem(PREF_KEY) || "null");
      if (!raw) return;
      if (raw.style) priceStyle = raw.style;
      if (raw.log) logScale = true;
      if (raw.pct) pctScale = true;
      if (raw.tf) timeframe = raw.tf;
      if (raw.tab) tab = raw.tab;
      if (raw.pool) lastPool = raw.pool;
    } catch (e) {}
  })();
  function savePref() {
    try {
      localStorage.setItem(PREF_KEY, JSON.stringify({
        style: priceStyle, log: logScale, pct: pctScale, tf: timeframe, tab: tab,
        pool: selected && selected.pool
      }));
    } catch (e) {}
  }
  function loadFavs() {
    try {
      var raw = JSON.parse(localStorage.getItem(FAV_KEY) || "[]");
      if (Array.isArray(raw)) {
        var m = {};
        raw.forEach(function (id) { if (id) m[id] = 1; });
        return m;
      }
    } catch (e) {}
    return {};
  }
  function saveFavs() {
    try { localStorage.setItem(FAV_KEY, JSON.stringify(Object.keys(favs))); } catch (e) {}
  }
  function loadAlerts() {
    try {
      var raw = JSON.parse(localStorage.getItem(ALERT_KEY) || "[]");
      if (Array.isArray(raw)) return raw;
    } catch (e) {}
    return [];
  }
  function saveAlerts() {
    try { localStorage.setItem(ALERT_KEY, JSON.stringify(alerts)); } catch (e) {}
  }

  function loadPaper() {
    try {
      var raw = JSON.parse(localStorage.getItem(PAPER_KEY) || "null");
      if (raw && typeof raw.cash === "number") return raw;
    } catch (e) {}
    return { cash: 10000, pos: null, slips: [] };
  }
  function savePaper() {
    try { localStorage.setItem(PAPER_KEY, JSON.stringify(paper)); } catch (e) {}
  }
  function loadInds() {
    try {
      var raw = JSON.parse(localStorage.getItem(IND_KEY) || "null");
      if (raw && typeof raw === "object") {
        if (raw.ma9 && raw.ma21) raw.ma9 = false;
        if ((raw.ma9 || raw.ma21) && raw.ema) raw.ema = false;
        return raw;
      }
    } catch (e) {}
    return { ma21: true };
  }
  function saveInds() {
    try { localStorage.setItem(IND_KEY, JSON.stringify(activeInds)); } catch (e) {}
  }
  (function loadSort() {
    try {
      var raw = JSON.parse(localStorage.getItem(SORT_KEY) || "null");
      if (raw && raw.key) { sortKey = raw.key; sortDir = raw.dir === 1 ? 1 : -1; }
    } catch (e) {}
  })();
  function saveSort() {
    try { localStorage.setItem(SORT_KEY, JSON.stringify({ key: sortKey, dir: sortDir })); } catch (e) {}
  }

  function ketTheme() {
    var light = document.documentElement.getAttribute("data-theme") === "light";
    if (light) {
      return {
        background: "#f3f5f0",
        textColor: "#161816",
        gridColor: "rgba(77,138,0,0.16)",
        borderColor: "#c8d6b0",
        upColor: "#4d8a00",
        downColor: RED,
        fontFamily: "Sora, ui-sans-serif, system-ui, sans-serif"
      };
    }
    return {
      background: "#0e0e0e",
      textColor: "#f2f3f4",
      gridColor: "rgba(192,248,0,0.10)",
      borderColor: "#2a3800",
      upColor: LIME,
      downColor: RED,
      fontFamily: "Sora, ui-sans-serif, system-ui, sans-serif"
    };
  }

  function kindOf(sym, name) {
    var s = String(sym || "").toUpperCase();
    if (s === "WETH" || s === "ETH" || s === "USDG" || s === "USDE" || s === "PUSD") return "cash";
    if (/robinhood token/i.test(name || "") || /tokenized/i.test(name || "")) return "stock";
    if (STOCKS[s]) return "stock";
    return "meme";
  }

  function n(x) {
    var v = parseFloat(x);
    return isFinite(v) ? v : 0;
  }
  function usd(x) {
    var v = n(x);
    if (v >= 1e9) return "$" + (v / 1e9).toFixed(2) + "B";
    if (v >= 1e6) return "$" + (v / 1e6).toFixed(2) + "M";
    if (v >= 1e3) return "$" + (v / 1e3).toFixed(1) + "k";
    if (v >= 1) return "$" + v.toFixed(2);
    if (v > 0) return "$" + v.toPrecision(3);
    return "—";
  }
  function pct(x) {
    var v = n(x);
    return (v >= 0 ? "+" : "") + v.toFixed(2) + "%";
  }
  function pxTxt(x) {
    var v = n(x);
    if (!(v > 0)) return "—";
    if (v >= 1000) return v.toFixed(2);
    if (v >= 1) return v.toFixed(4);
    return v.toPrecision(4);
  }

  function specOf(tf) {
    var s = String(tf || timeframe || "60");
    var i;
    for (i = 0; i < TFS.length; i++) {
      if (TFS[i].id === s || TFS[i].lab === s) return TFS[i];
    }
    var aliases = {
      "30s": "30s", "1m": "1", "5m": "5", "15m": "15", "30m": "30",
      "1h": "60", "60m": "60", "4h": "240", "1d": "D", "1D": "D", "D": "D",
      "1w": "W", "1W": "W", "W": "W", "1M": "M", "1mo": "M", "M": "M",
      "1440": "D", "240": "240"
    };
    var id = aliases[s] || aliases[s.toLowerCase()];
    if (id) {
      for (i = 0; i < TFS.length; i++) if (TFS[i].id === id) return TFS[i];
    }
    return TFS[5];
  }

  function scorePair(p) {
    var liq = n(p.liq);
    var vol = n(p.vol);
    var buys = n(p.buys);
    var sells = n(p.sells);
    var tx = buys + sells;
    var ageH = p.created ? Math.max(0, (Date.now() - p.created) / 36e5) : 48;
    var liquidity = Math.max(0, Math.min(100, Math.log10(liq + 1) * 18));
    var holders = Math.max(0, Math.min(100, Math.log10(tx + 1) * 20));
    var security = p.kind === "stock" ? 82 : (liq > 5e5 ? 70 : liq > 5e4 ? 52 : 28);
    var sm = Math.max(0, Math.min(100, (vol / Math.max(liq, 1)) * 40));
    var bot = tx > 0 ? Math.max(0, 100 - Math.abs(buys - sells) / tx * 80) : 40;
    if (ageH < 2) security = Math.min(security, 35);
    var conv = liquidity * 0.15 + security * 0.2 + holders * 0.25 + 70 * 0.15 + sm * 0.15 + bot * 0.1;
    if (liq < 10000) conv = Math.min(conv, 45);
    return {
      conv: Math.round(conv),
      dims: [
        { k: "Liquidity", v: Math.round(liquidity) },
        { k: "Security", v: Math.round(security) },
        { k: "Flow", v: Math.round(holders) },
        { k: "Dev trust", v: p.kind === "stock" ? 88 : (ageH > 24 ? 64 : 38) },
        { k: "Turnover", v: Math.round(sm) },
        { k: "Skew", v: Math.round(bot) }
      ]
    };
  }
  function grade(s) {
    if (s >= 80) return "HIGH";
    if (s >= 60) return "MOD";
    if (s >= 40) return "LOW";
    return "AVOID";
  }

  function parseGecko(data) {
    var out = [];
    (data || []).forEach(function (row) {
      var a = row.attributes || {};
      var name = String(a.name || "");
      var parts = name.split("/")[0].trim().split(" ");
      var sym = parts[0] || "???";
      var rel = row.relationships || {};
      var base = ((rel.base_token || {}).data || {}).id || "";
      var token = base.replace(/^robinhood_/, "");
      var chg = n((a.price_change_percentage || {}).h24);
      var tx = a.transactions && a.transactions.h24 ? a.transactions.h24 : {};
      var pool = String(a.address || "").toLowerCase();
      if (!pool) return;
      out.push({
        pool: pool,
        token: token.toLowerCase(),
        sym: sym,
        name: name,
        px: n(a.base_token_price_usd || a.token_price_usd),
        chg: chg,
        vol: n((a.volume_usd || {}).h24),
        liq: n(a.reserve_in_usd),
        mcap: n(a.market_cap_usd || a.fdv_usd),
        buys: n(tx.buys),
        sells: n(tx.sells),
        created: a.pool_created_at ? Date.parse(a.pool_created_at) : 0,
        kind: kindOf(sym, name),
        source: "rhc"
      });
    });
    return out;
  }

  function fetchJson(url) {
    return fetch(url).then(function (r) {
      return r.json().then(function (j) {
        return { ok: r.ok, status: r.status, json: j };
      }).catch(function () {
        return { ok: false, status: r.status, json: null };
      });
    }).catch(function () {
      return { ok: false, status: 0, json: null };
    });
  }

  function putPair(p) {
    if (!p || !p.pool) return;
    var old = null;
    var i;
    for (i = 0; i < pairs.length; i++) if (pairs[i].pool === p.pool) { old = pairs[i]; break; }
    if (!old) pairs.push(p);
    else if (p.vol > old.vol) pairs[i] = p;
  }

  function cmpPairs(a, b) {
    if (sortKey === "sym") {
      return String(a.sym || "").localeCompare(String(b.sym || "")) * sortDir;
    }
    return (n(a[sortKey]) - n(b[sortKey])) * sortDir;
  }

  function loadBoard() {
    return Promise.all([
      fetchJson("https://api.geckoterminal.com/api/v2/networks/robinhood/trending_pools?page=1"),
      fetchJson("https://api.geckoterminal.com/api/v2/networks/robinhood/new_pools?page=1")
    ]).then(function (res) {
      parseGecko((res[0].json && res[0].json.data) || []).forEach(putPair);
      parseGecko((res[1].json && res[1].json.data) || []).forEach(putPair);
      document.querySelectorAll("[data-tab]").forEach(function (x) {
        x.classList.toggle("on", x.getAttribute("data-tab") === tab);
      });
      if (!selected && lastPool) {
        var i;
        for (i = 0; i < pairs.length; i++) if (pairs[i].pool === lastPool) { selected = pairs[i]; break; }
      }
      if (!selected) selected = visible()[0] || null;
      paintList();
      paintTrench();
      loadStocks();
    });
  }

  function geckoSearch(query) {
    var qq = encodeURIComponent(query);
    return fetchJson("https://api.geckoterminal.com/api/v2/search/pools?query=" + qq + "&network=robinhood")
      .then(function (r) {
        return parseGecko((r.json && r.json.data) || []);
      });
  }

  function loadStocks() {
    var jobs = STOCK_BOOT.map(function (s) { return geckoSearch(s).catch(function () { return []; }); });
    return Promise.all(jobs).then(function (sets) {
      var had = !!selected;
      sets.forEach(function (arr) { arr.forEach(putPair); });
      if (!selected && lastPool) {
        var j;
        for (j = 0; j < pairs.length; j++) if (pairs[j].pool === lastPool) { selected = pairs[j]; break; }
      }
      if (!selected) selected = visible()[0] || pairs[0] || null;
      paintList();
      paintTrench();
      if (!had && selected) bootChart();
    });
  }

  function visible() {
    var qq = q.trim().toLowerCase();
    return pairs.filter(function (p) {
      if (tab === "stock" && p.kind !== "stock") return false;
      if (tab === "meme" && p.kind !== "meme") return false;
      if (tab === "fav" && !favs[p.pool]) return false;
      if (qq && (p.sym + " " + p.name + " " + p.token + " " + p.pool).toLowerCase().indexOf(qq) < 0) return false;
      return true;
    }).sort(cmpPairs);
  }

  function paintCols() {
    var box = document.getElementById("cols");
    if (!box) return;
    box.innerHTML = "";
    box.appendChild(document.createElement("span"));
    [
      { k: "sym", lab: "Pair" },
      { k: "px", lab: "Price" },
      { k: "chg", lab: "24h", num: 1 },
      { k: "vol", lab: "Vol", num: 1 },
      { k: "liq", lab: "Liq", num: 1 }
    ].forEach(function (c) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = (c.k === sortKey ? "on" : "") + (c.num ? " num" : "");
      b.textContent = c.lab + (c.k === sortKey ? (sortDir < 0 ? " ↓" : " ↑") : "");
      b.onclick = function () {
        if (sortKey === c.k) sortDir = -sortDir;
        else { sortKey = c.k; sortDir = c.k === "sym" ? 1 : -1; }
        saveSort();
        paintList();
      };
      box.appendChild(b);
    });
  }

  function paintList() {
    paintCols();
    var box = document.getElementById("rows");
    if (!box) return;
    box.innerHTML = "";
    var rows = tab === "majors" ? MAJORS.map(function (m) {
      return { pool: m.id, token: m.id, sym: m.lab, name: m.lab, px: 0, chg: 0, vol: 0, liq: 0, kind: "cash", source: "binance" };
    }) : visible();
    if (!rows.length) {
      box.innerHTML = "<div class='empty'>" +
        (tab === "fav" ? "Star pairs to build a watchlist." : tab === "stock" ? "No RHC stock pools in this cut yet." : "No pairs in this cut.") +
        "</div>";
      return;
    }
    rows.forEach(function (p) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = "pair" + (selected && selected.pool === p.pool ? " on" : "");
      var up = n(p.chg) >= 0;
      b.innerHTML =
        "<span class='star'></span>" +
        "<span class='sym'></span>" +
        "<span class='px'></span>" +
        "<span class='chg'></span>" +
        "<span class='vol'></span>" +
        "<span class='liq'></span>";
      var st = b.querySelector(".star");
      st.textContent = favs[p.pool] ? "★" : "☆";
      st.className = "star" + (favs[p.pool] ? " on" : "");
      b.querySelector(".sym").textContent = p.sym;
      b.querySelector(".px").textContent = p.px ? usd(p.px) : "—";
      var ch = b.querySelector(".chg");
      ch.textContent = p.source === "binance" ? "—" : pct(p.chg);
      ch.className = "chg " + (up ? "up" : "dn");
      b.querySelector(".vol").textContent = p.source === "binance" ? "—" : usd(p.vol);
      b.querySelector(".liq").textContent = p.source === "binance" ? "—" : usd(p.liq);
      b.onclick = function (ev) {
        if (ev.target && ev.target.classList && ev.target.classList.contains("star")) {
          if (favs[p.pool]) delete favs[p.pool];
          else favs[p.pool] = 1;
          saveFavs();
          paintList();
          return;
        }
        selected = p;
        lastTape = "";
        lastBars = [];
        savePref();
        paintList();
        paintTrench();
        paintHud();
        bootChart();
      };
      box.appendChild(b);
    });
  }

  function ageTxt(ms) {
    if (!(ms > 0)) return "—";
    var h = (Date.now() - ms) / 36e5;
    if (h < 1) return Math.max(1, Math.round(h * 60)) + "m";
    if (h < 48) return h.toFixed(1) + "h";
    return (h / 24).toFixed(1) + "d";
  }

  function atr14(bars) {
    if (!bars || bars.length < 15) return 0;
    var s = 0;
    var i;
    for (i = bars.length - 14; i < bars.length; i++) {
      var p = bars[i - 1];
      var b = bars[i];
      var tr = Math.max(b.high - b.low, Math.abs(b.high - p.close), Math.abs(b.low - p.close));
      s += tr;
    }
    return s / 14;
  }

  function range24(bars) {
    if (!bars || !bars.length) return { lo: 0, hi: 0 };
    var cut = Date.now() - 864e5;
    var lo = Infinity;
    var hi = 0;
    var i;
    for (i = 0; i < bars.length; i++) {
      if (bars[i].time < cut && bars.length > 8) continue;
      if (bars[i].low < lo) lo = bars[i].low;
      if (bars[i].high > hi) hi = bars[i].high;
    }
    if (!(hi > 0) || !isFinite(lo)) return { lo: 0, hi: 0 };
    return { lo: lo, hi: hi };
  }

  function paintTrench() {
    var p = selected;
    var el = document.getElementById("trench");
    if (!el) return;
    if (!p || p.source === "binance") {
      el.innerHTML = "<p class='hint'>Pick a Robinhood Chain pair.</p>";
      return;
    }
    var sc = scorePair(p);
    var dims = sc.dims.map(function (d) {
      return "<div class='dim'><span>" + d.k + "</span><b>" + d.v + "</b></div>";
    }).join("");
    var tx = n(p.buys) + n(p.sells);
    el.innerHTML =
      "<div class='stats'>" +
        "<div><span>Mcap</span><b>" + usd(p.mcap) + "</b></div>" +
        "<div><span>Liq</span><b>" + usd(p.liq) + "</b></div>" +
        "<div><span>Vol 24h</span><b>" + usd(p.vol) + "</b></div>" +
        "<div><span>Age</span><b>" + ageTxt(p.created) + "</b></div>" +
        "<div><span>Buys</span><b>" + (p.buys || "—") + "</b></div>" +
        "<div><span>Sells</span><b>" + (p.sells || "—") + "</b></div>" +
        "<div><span>Tx 24h</span><b>" + (tx || "—") + "</b></div>" +
        "<div><span>Pool</span><b>" + (p.pool ? p.pool.slice(0, 6) + "…" : "—") + "</b></div>" +
      "</div>" +
      "<div class='mono'>Trench</div>" +
      "<div class='conv'><b>" + sc.conv + "</b><span>" + grade(sc.conv) + "</span></div>" +
      "<div class='dims'>" + dims + "</div>" +
      "<p class='hint'>On-chain book only — not GMGN.</p>";
  }

  function markPrice() {
    var px = 0;
    if (lastBars.length) px = n(lastBars[lastBars.length - 1].close);
    if (!(px > 0) && selected) px = n(selected.px);
    if (!(px > 0)) px = n(lastClose);
    if (px > 0) lastClose = px;
    return px;
  }

  function paintPaper() {
    var px = markPrice();
    var cash = document.getElementById("cash");
    if (cash) cash.textContent = usd(paper.cash);
    var mark = document.getElementById("mark");
    if (mark) mark.textContent = px ? usd(px) : "—";
    var pos = document.getElementById("pos");
    var pnlEl = document.getElementById("pnl");
    if (!pos) return;
    if (!paper.pos) {
      pos.textContent = "Flat";
      if (pnlEl) { pnlEl.textContent = "—"; pnlEl.className = ""; }
      return;
    }
    var last = px || paper.pos.entry;
    var pnl = (last - paper.pos.entry) * paper.pos.qty * (paper.pos.side === "short" ? -1 : 1);
    var p = paper.pos.cost ? (pnl / paper.pos.cost) * 100 : 0;
    pos.textContent = paper.pos.side.toUpperCase() + " " + paper.pos.sym + " @ " + pxTxt(paper.pos.entry);
    if (pnlEl) {
      pnlEl.textContent = (pnl >= 0 ? "+" : "") + usd(Math.abs(pnl)).replace("$", (pnl >= 0 ? "$" : "-$")) +
        "  " + (p >= 0 ? "+" : "") + p.toFixed(2) + "%";
      pnlEl.className = p >= 0 ? "up" : "dn";
    }
  }

  function flashPaper(msg) {
    var el = document.getElementById("flash");
    if (!el) return;
    el.textContent = msg || "";
    if (flashTimer) clearTimeout(flashTimer);
    flashTimer = setTimeout(function () { el.textContent = ""; }, 3200);
  }

  function setLive(on, label, warn) {
    var el = document.getElementById("live");
    if (!el) return;
    el.classList.toggle("on", !!on);
    el.classList.toggle("warn", !!warn);
    var lab = el.querySelector(".lab");
    if (lab) lab.textContent = label || "rhc";
  }

  function paintHud() {
    var hud = document.getElementById("hud");
    var quote = document.getElementById("quote");
    var note = document.getElementById("tape-note");
    var p = selected;
    if (!hud) return;
    if (!p) {
      hud.hidden = true;
      if (quote) quote.hidden = true;
      if (note) note.textContent = "";
      return;
    }
    hud.hidden = false;
    if (quote) quote.hidden = false;
    var bars = lastBars;
    var last = bars.length ? bars[bars.length - 1] : null;
    var o = last ? last.open : p.px;
    var h = last ? last.high : p.px;
    var l = last ? last.low : p.px;
    var c = last ? last.close : p.px;
    var px = markPrice();
    var up = n(p.chg) >= 0;
    var barUp = n(c) >= n(o);
    if (quote) {
      quote.querySelector(".sym").textContent = p.sym;
      quote.querySelector(".last").textContent = px ? usd(px) : "—";
      var qch = quote.querySelector(".chg");
      qch.textContent = p.source === "binance" ? "" : pct(p.chg);
      qch.className = "chg " + (up ? "up" : "dn");
      quote.querySelector(".qvol").textContent = usd(p.vol);
      quote.querySelector(".qliq").textContent = usd(p.liq);
      quote.querySelector(".qmcap").textContent = usd(p.mcap);
      var rg = range24(bars);
      quote.querySelector(".qrange").textContent = rg.hi ? (pxTxt(rg.lo) + "–" + pxTxt(rg.hi)) : "—";
      var atr = atr14(bars);
      quote.querySelector(".qatr").textContent = atr ? pxTxt(atr) : "—";
    }
    hud.querySelector(".name").textContent = p.source === "binance" ? "Binance" : (p.name || "Robinhood Chain");
    hud.querySelector(".o").textContent = pxTxt(o);
    hud.querySelector(".h").textContent = pxTxt(h);
    hud.querySelector(".l").textContent = pxTxt(l);
    var ce = hud.querySelector(".c");
    ce.textContent = pxTxt(c);
    ce.className = "c " + (barUp ? "up" : "dn");
    var msg = "";
    if (p.source === "binance") msg = "";
    else if (lastTape === "print") msg = "No tape yet — last print only.";
    else if (lastTape === "prints") msg = "30s from last prints.";
    else if (lastTape === "paprika") msg = "Thin tape · DexPaprika fallback.";
    else if (lastTape === "empty") msg = "No tape and no print for this pool.";
    if (note) note.textContent = msg;
    paintPaper();
    checkAlerts();
  }

  function normalizeBars(list) {
    var map = {};
    (list || []).forEach(function (b) {
      if (!b || !(b.time > 0) || !(b.close > 0)) return;
      var t = b.time;
      if (!map[t]) {
        map[t] = {
          time: t,
          open: n(b.open),
          high: n(b.high),
          low: n(b.low),
          close: n(b.close),
          volume: n(b.volume)
        };
      }
    });
    return Object.keys(map).map(Number).sort(function (a, b) { return a - b; }).map(function (t) { return map[t]; });
  }

  function aggregateBars(bars, bucketMs) {
    var map = {};
    (bars || []).forEach(function (b) {
      var t = Math.floor(b.time / bucketMs) * bucketMs;
      var x = map[t];
      if (!x) {
        map[t] = { time: t, open: b.open, high: b.high, low: b.low, close: b.close, volume: n(b.volume) };
      } else {
        x.high = Math.max(x.high, b.high);
        x.low = Math.min(x.low, b.low);
        x.close = b.close;
        x.volume += n(b.volume);
      }
    });
    return Object.keys(map).map(Number).sort(function (a, b) { return a - b; }).map(function (t) { return map[t]; });
  }

  function scaleUsd(bars, usdPx) {
    var px = n(usdPx);
    if (!(px > 0) || !bars.length) return bars;
    var last = n(bars[bars.length - 1].close);
    if (!(last > 0)) return bars;
    var k = px / last;
    if (!isFinite(k) || k <= 0 || Math.abs(k - 1) < 0.02) return bars;
    return bars.map(function (b) {
      return {
        time: b.time,
        open: b.open * k,
        high: b.high * k,
        low: b.low * k,
        close: b.close * k,
        volume: b.volume
      };
    });
  }

  function stubBars(px, spec) {
    var price = n(px);
    if (!(price > 0)) return [];
    var ms = spec.ms || 36e5;
    var count = spec.fromTrades ? 48 : 96;
    var end = Math.floor(Date.now() / ms) * ms;
    var start = end - count * ms;
    var out = [];
    var i;
    for (i = 0; i <= count; i++) {
      out.push({
        time: start + i * ms,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: 0
      });
    }
    return out;
  }

  function geckoBars(pool, spec, limit) {
    if (!spec.gecko || spec.gecko === "second") return Promise.resolve([]);
    var lim = spec.ms <= 6e4 ? 1000 : (spec.bucketMs ? 400 : 400);
    if (spec.id === "W" || spec.id === "M") lim = 400;
    if (n(limit) > 0) lim = Math.min(lim, n(limit));
    var url = "https://api.geckoterminal.com/api/v2/networks/robinhood/pools/" +
      encodeURIComponent(pool) + "/ohlcv/" + spec.gecko +
      "?aggregate=" + spec.agg + "&limit=" + lim + "&currency=usd";
    return fetchJson(url).then(function (r) {
      if (!r.ok || !r.json) return [];
      var list = (((r.json || {}).data || {}).attributes || {}).ohlcv_list || [];
      var bars = normalizeBars(list.map(function (row) {
        return {
          time: n(row[0]) * 1000,
          open: n(row[1]),
          high: n(row[2]),
          low: n(row[3]),
          close: n(row[4]),
          volume: n(row[5])
        };
      }));
      if (spec.bucketMs && spec.bucketMs > spec.ms / 8) bars = aggregateBars(bars, spec.bucketMs);
      return bars;
    });
  }

  function paprikaBars(pool, spec, limit) {
    if (!spec.paprika) return Promise.resolve([]);
    var step = spec.paprikaMs || spec.ms;
    var lim = Math.max(24, Math.min(n(limit) || 366, 366));
    if (spec.paprikaMs && spec.paprikaMs < spec.ms) {
      lim = Math.min(366, Math.ceil(lim * (spec.ms / spec.paprikaMs)));
    }
    if (spec.bucketMs && spec.bucketMs > step) {
      lim = Math.min(366, Math.ceil((spec.bucketMs / step) * Math.min(lim, 80)));
    }
    var start = new Date(Date.now() - lim * step).toISOString();
    var url = "https://api.dexpaprika.com/networks/robinhood/pools/" +
      encodeURIComponent(pool) + "/ohlcv?interval=" + encodeURIComponent(spec.paprika) +
      "&limit=" + lim + "&start=" + encodeURIComponent(start);
    return fetchJson(url).then(function (r) {
      if (!r.ok || !r.json || !Array.isArray(r.json)) return [];
      var bars = normalizeBars(r.json.map(function (row) {
        var t = Date.parse(row.time_open || row.time_close || "");
        return {
          time: t,
          open: n(row.open),
          high: n(row.high),
          low: n(row.low),
          close: n(row.close),
          volume: n(row.volume)
        };
      }));
      if (spec.paprikaMs && spec.paprikaMs < spec.ms) bars = aggregateBars(bars, spec.ms);
      if (spec.bucketMs && spec.bucketMs > step) bars = aggregateBars(bars, spec.bucketMs);
      return bars;
    });
  }

  function tradePx(a) {
    var from = n(a.price_from_in_usd);
    var to = n(a.price_to_in_usd);
    var ref = n(selected && selected.px);
    if (ref > 0) {
      if (from > 0 && to > 0) return Math.abs(from - ref) <= Math.abs(to - ref) ? from : to;
      return from || to;
    }
    if (a.kind === "buy" && to > 0) return to;
    if (a.kind === "sell" && from > 0) return from;
    if (from > 0 && to > 0) return Math.min(from, to);
    return from || to;
  }

  function geckoTrades(pool) {
    var url = "https://api.geckoterminal.com/api/v2/networks/robinhood/pools/" +
      encodeURIComponent(pool) + "/trades";
    return fetchJson(url).then(function (r) {
      if (!r.ok || !r.json) return [];
      return (r.json.data || []).slice(0, 200).map(function (row) {
        var a = row.attributes || {};
        return {
          time: Date.parse(a.block_timestamp || ""),
          px: tradePx(a),
          vol: n(a.volume_in_usd)
        };
      }).filter(function (t) { return t.time > 0 && t.px > 0; })
        .sort(function (a, b) { return a.time - b.time; });
    });
  }

  function tradesToBars(trades, bucketMs) {
    var map = {};
    (trades || []).forEach(function (t) {
      var ts = Math.floor(t.time / bucketMs) * bucketMs;
      var x = map[ts];
      if (!x) {
        map[ts] = { time: ts, open: t.px, high: t.px, low: t.px, close: t.px, volume: n(t.vol) };
      } else {
        x.high = Math.max(x.high, t.px);
        x.low = Math.min(x.low, t.px);
        x.close = t.px;
        x.volume += n(t.vol);
      }
    });
    return Object.keys(map).map(Number).sort(function (a, b) { return a - b; }).map(function (t) { return map[t]; });
  }

  function loadRhcBars(pool, tf, limit) {
    var spec = specOf(tf);
    var key = String(pool || "").toLowerCase() + "|" + spec.id;
    var hit = barCache[key];
    var ttl = spec.fromTrades ? 8000 : (hit && hit.source === "print" ? 15000 : 45000);
    if (hit && Date.now() - hit.at < ttl) return Promise.resolve(hit);
    var px = (selected && selected.pool === String(pool || "").toLowerCase() && selected.px) || lastClose || 0;

    function finish(res) {
      barCache[key] = { bars: res.bars, source: res.source, at: Date.now() };
      return res;
    }

    if (spec.fromTrades) {
      return geckoTrades(pool).then(function (trades) {
        var bars = tradesToBars(trades, spec.ms);
        if (bars.length > 180) bars = bars.slice(-180);
        if (bars.length >= 2) return finish({ bars: bars, source: "prints" });
        var stub = stubBars(px, spec);
        return finish({ bars: stub, source: stub.length ? "print" : "empty" });
      });
    }

    return geckoBars(pool, spec, limit).then(function (gb) {
      if (gb.length) return finish({ bars: gb, source: "gecko" });
      return paprikaBars(pool, spec, limit).then(function (pb) {
        if (pb.length) return finish({ bars: scaleUsd(pb, px), source: "paprika" });
        var stub = stubBars(px, spec);
        return finish({ bars: stub, source: stub.length ? "print" : "empty" });
      });
    });
  }

  function clipBars(bars, opts) {
    var out = bars || [];
    if (opts && opts.to != null) {
      var to = n(opts.to);
      if (to > 0) out = out.filter(function (b) { return b.time < to; });
    }
    if (opts && opts.from != null) {
      var from = n(opts.from);
      if (from > 0) out = out.filter(function (b) { return b.time >= from; });
    }
    if (opts && n(opts.limit) > 0 && out.length > n(opts.limit)) out = out.slice(-n(opts.limit));
    return out;
  }

  function velaTf(spec) {
    if (!spec) spec = specOf(timeframe);
    if (spec.fromTrades || spec.id === "30s") return "1";
    return spec.id;
  }

  function rhcProvider() {
    return {
      getBars: function (ticker, tf, opts) {
        var pool = String(ticker || "").replace(/^rhc:/i, "").toLowerCase();
        if (!pool) return Promise.resolve([]);
        var spec = specOf(tf);
        if (spec.fromTrades) return Promise.resolve([]);
        return loadRhcBars(pool, tf, opts && opts.limit).then(function (res) {
          var bars = clipBars(res.bars, opts);
          if (!selected || selected.source === "binance" || selected.pool === pool) {
            lastBars = res.bars;
            lastTape = res.source;
            if (lastBars.length) lastClose = lastBars[lastBars.length - 1].close;
            paintHud();
            setLive(res.source !== "empty", res.source === "print" || res.source === "prints" ? "print" : "rhc", res.source === "print" || res.source === "paprika" || res.source === "prints");
          }
          return bars;
        }).catch(function () {
          var stub = stubBars((selected && selected.px) || lastClose, specOf(tf));
          lastBars = stub;
          lastTape = stub.length ? "print" : "empty";
          paintHud();
          setLive(false, "print", true);
          return clipBars(stub, opts);
        });
      },
      listSymbols: function () {
        return Promise.resolve(pairs.map(function (p) {
          return { ticker: p.pool, description: p.sym + " · Robinhood Chain" };
        }));
      }
    };
  }

  function colorVolume() {
    if (!chart || !chart.addNativeIndicator) return;
    var th = ketTheme();
    try {
      var h = chart.addNativeIndicator("volume", {
        inputs: { upColor: th.upColor, downColor: th.downColor, heightPct: 18 }
      });
      if (h && h.setInputs) h.setInputs({ upColor: th.upColor, downColor: th.downColor, heightPct: 18 });
    } catch (e) {}
  }

  function syncIndicators() {
    if (!chart || !chart.addNativeIndicator) return;
    colorVolume();
    INDS.forEach(function (ind) {
      var on = !!activeInds[ind.id];
      if (!on) {
        if (indHandles[ind.id]) {
          try { if (indHandles[ind.id].remove) indHandles[ind.id].remove(); } catch (e) {}
          indHandles[ind.id] = null;
        }
        return;
      }
      if (indHandles[ind.id]) return;
      try {
        indHandles[ind.id] = chart.addNativeIndicator(ind.type, { inputs: ind.inputs || {} });
      } catch (e) {
        indHandles[ind.id] = null;
      }
    });
  }

  function clearIndHandles() {
    Object.keys(indHandles).forEach(function (k) {
      try { if (indHandles[k] && indHandles[k].remove) indHandles[k].remove(); } catch (e) {}
    });
    indHandles = {};
  }

  function paintInds() {
    var box = document.getElementById("inds");
    if (!box) return;
    box.innerHTML = "";
    INDS.forEach(function (ind) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = "pill" + (activeInds[ind.id] ? " on" : "");
      b.textContent = ind.lab;
      b.onclick = function () {
        var next = !activeInds[ind.id];
        if (next && (ind.id === "ma9" || ind.id === "ma21" || ind.id === "ema")) {
          ["ma9", "ma21", "ema"].forEach(function (k) {
            if (k === ind.id) return;
            activeInds[k] = false;
            if (indHandles[k]) {
              try { if (indHandles[k].remove) indHandles[k].remove(); } catch (e) {}
              indHandles[k] = null;
            }
          });
        }
        activeInds[ind.id] = next;
        saveInds();
        if (!activeInds[ind.id] && indHandles[ind.id]) {
          try { if (indHandles[ind.id].remove) indHandles[ind.id].remove(); } catch (e) {}
          indHandles[ind.id] = null;
        }
        paintInds();
        syncIndicators();
      };
      box.appendChild(b);
    });
  }

  function ensureFeed(api) {
    if (feed) return feed;
    feed = new api.MultiProviderFeed();
    feed.registerProvider("rhc", rhcProvider());
    if (api.BinanceProvider) feed.registerProvider("binance", new api.BinanceProvider());
    return feed;
  }

  function marketSymbol(p) {
    if (!p) return "";
    return p.source === "binance" ? p.pool : ("rhc:" + p.pool);
  }

  function finishBoot(gen) {
    if (gen !== bootGen) return;
    syncIndicators();
    applyChartPrefs();
    try { if (chart.drawings && chart.drawings.showToolbar) chart.drawings.showToolbar(true); } catch (e) {}
    if (rangePreset) {
      try { if (chart.setVisibleRangePreset) chart.setVisibleRangePreset(rangePreset); } catch (e) {}
    }
    paintHud();
    paintAlerts();
    try { if (chart.resize) chart.resize(); } catch (e) {}
  }

  function mountChart(api, plot, p, spec, gen, data) {
    var isBinance = p.source === "binance";
    var symbol = marketSymbol(p);
    var th = ketTheme();
    var tf = velaTf(spec);
    var market = {
      symbol: symbol,
      timeframe: tf,
      bars: spec.fromTrades ? 120 : 400,
      live: !!isBinance && !spec.fromTrades
    };
    if (data) market.data = data;

    if (chart && chart.setMarket) {
      try {
        if (chart.setTheme) chart.setTheme(th);
        chart.setMarket(market);
        if (chart.ready) {
          chart.ready().then(function () { finishBoot(gen); }).catch(function () {});
        }
        return;
      } catch (e) {
        try { if (chart.destroy) chart.destroy(); } catch (e2) {}
        chart = null;
        clearIndHandles();
      }
    }

    plot.innerHTML = "";
    clearIndHandles();
    try {
      chart = new api.Vela(plot, {
        symbol: market.symbol,
        timeframe: market.timeframe,
        bars: market.bars,
        live: market.live,
        data: market.data,
        theme: th,
        upColor: th.upColor,
        downColor: th.downColor,
        currentPriceLine: true,
        drawings: true,
        volume: false
      }, { dataFeed: ensureFeed(api) });
      var ready = chart.ready ? chart.ready() : Promise.resolve();
      ready.then(function () {
        if (isBinance) setLive(true, "binance", false);
        finishBoot(gen);
      }).catch(function () {
        if (gen !== bootGen) return;
        setLive(false, "rhc", true);
      });
    } catch (e) {
      setLive(false, "rhc", true);
    }
  }

  function bootChart() {
    var api = g.Vela;
    var plot = document.getElementById("plot");
    if (!api || !api.Vela || !plot) return;
    paintPills();
    paintInds();
    paintStyles();
    paintScale();
    paintRange();
    var p = selected;
    if (!p) return;
    if (p.source === "binance" && specOf(timeframe).fromTrades) {
      timeframe = "1";
      paintPills();
      flashPaper("Binance has no 30s tape — 1m.");
    }
    var spec = specOf(timeframe);
    var gen = ++bootGen;
    var isBinance = p.source === "binance";
    lastTape = isBinance ? "binance" : lastTape;
    if (isBinance) setLive(true, "binance", false);

    if (spec.fromTrades && !isBinance) {
      loadRhcBars(p.pool, spec.id, 200).then(function (res) {
        if (gen !== bootGen) return;
        lastBars = res.bars;
        lastTape = res.source;
        if (lastBars.length) lastClose = lastBars[lastBars.length - 1].close;
        paintHud();
        setLive(res.source !== "empty", "print", true);
        mountChart(api, plot, p, spec, gen, lastBars.length ? lastBars : stubBars(markPrice(), spec));
      }).catch(function () {
        if (gen !== bootGen) return;
        mountChart(api, plot, p, spec, gen, stubBars(markPrice(), spec));
      });
      return;
    }

    mountChart(api, plot, p, spec, gen);
  }

  function paintPills() {
    var box = document.getElementById("tfs");
    if (!box) return;
    box.innerHTML = "";
    TFS.forEach(function (t) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = "pill" + (t.id === timeframe ? " on" : "");
      b.textContent = t.lab;
      b.onclick = function () { timeframe = t.id; savePref(); bootChart(); };
      box.appendChild(b);
    });
  }

  function paintStyles() {
    var box = document.getElementById("styles");
    if (!box) return;
    box.innerHTML = "";
    STYLES.forEach(function (s) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = "pill" + (priceStyle === s.id ? " on" : "");
      b.textContent = s.lab;
      b.onclick = function () {
        priceStyle = s.id;
        savePref();
        paintStyles();
        applyChartPrefs();
      };
      box.appendChild(b);
    });
  }

  function paintScale() {
    var box = document.getElementById("scale");
    if (!box) return;
    box.innerHTML = "";
    [
      { k: "log", lab: "Log", on: logScale },
      { k: "pct", lab: "%", on: pctScale }
    ].forEach(function (s) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = "pill" + (s.on ? " on" : "");
      b.textContent = s.lab;
      b.onclick = function () {
        if (s.k === "log") logScale = !logScale;
        else pctScale = !pctScale;
        savePref();
        paintScale();
        applyChartPrefs();
      };
      box.appendChild(b);
    });
  }

  function paintRange() {
    var box = document.getElementById("range");
    if (!box) return;
    box.innerHTML = "";
    RANGES.forEach(function (r) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = "pill" + (rangePreset === r ? " on" : "");
      b.textContent = r;
      b.onclick = function () {
        rangePreset = r;
        paintRange();
        try { if (chart && chart.setVisibleRangePreset) chart.setVisibleRangePreset(r); } catch (e) {}
      };
      box.appendChild(b);
    });
  }

  function paintRisk() {
    var box = document.getElementById("risk");
    if (!box) return;
    box.innerHTML = "";
    [0.1, 0.25, 0.5, 1].forEach(function (frac) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = "pill";
      b.textContent = (frac * 100) + "%";
      b.onclick = function () {
        var el = document.getElementById("notional");
        if (el) el.value = String(Math.max(1, Math.round(paper.cash * frac)));
      };
      box.appendChild(b);
    });
  }

  function applyChartPrefs() {
    if (!chart || !chart.renderer || !chart.renderer.set) return;
    try { chart.renderer.set("priceStyle", priceStyle); } catch (e) {}
    try { chart.renderer.set("logScale", logScale); } catch (e) {}
    try { chart.renderer.set("scaleMode", pctScale ? "percent" : "price"); } catch (e) {}
    try { chart.renderer.set("keyboard", true); } catch (e) {}
    try { chart.renderer.set("countdown", true); } catch (e) {}
    try { chart.renderer.set("priceLabel", true); } catch (e) {}
    try { chart.renderer.set("attribution", false); } catch (e) {}
    try {
      chart.renderer.set("tradeMarkers", {
        visible: true, labels: true, qty: true,
        colors: { long: LIME, short: RED, exit: "#8aa090" }
      });
    } catch (e) {}
  }

  function shotChart() {
    if (!chart || !chart.renderer || !chart.renderer.screenshot) {
      flashPaper("Shot unavailable.");
      return;
    }
    try {
      var url = chart.renderer.screenshot();
      if (!url) { flashPaper("Shot unavailable."); return; }
      var a = document.createElement("a");
      a.href = url;
      a.download = ((selected && selected.sym) || "ketcharts").toLowerCase() + "-chart.png";
      a.click();
    } catch (e) {
      flashPaper("Shot failed.");
    }
  }

  function paintAlerts() {
    var box = document.getElementById("alert-list");
    if (!box) return;
    box.innerHTML = "";
    var mine = alerts.filter(function (a) { return selected && a.pool === selected.pool; });
    if (!mine.length) return;
    mine.forEach(function (a) {
      var row = document.createElement("div");
      row.className = "alert-row";
      var lab = document.createElement("span");
      lab.textContent = (a.dir === "above" ? "≥ " : "≤ ") + pxTxt(a.px);
      var x = document.createElement("button");
      x.type = "button";
      x.className = "pill";
      x.textContent = "×";
      x.onclick = function () {
        alerts = alerts.filter(function (z) { return z.id !== a.id; });
        saveAlerts();
        paintAlerts();
      };
      row.appendChild(lab);
      row.appendChild(x);
      box.appendChild(row);
    });
  }

  function addAlert(dir) {
    if (!selected) { flashPaper("Pick a pair first."); return; }
    var el = document.getElementById("alert-px");
    var px = n(el && el.value);
    if (!(px > 0)) px = markPrice();
    if (!(px > 0)) { flashPaper("Need a price."); return; }
    alerts.push({
      id: Date.now() + "-" + Math.random().toString(16).slice(2),
      pool: selected.pool,
      sym: selected.sym,
      px: px,
      dir: dir
    });
    saveAlerts();
    paintAlerts();
    flashPaper("Alert " + selected.sym + " " + (dir === "above" ? "≥" : "≤") + " " + pxTxt(px));
    if (typeof Notification !== "undefined" && Notification.permission === "default") {
      try { Notification.requestPermission(); } catch (e) {}
    }
  }

  function checkAlerts() {
    if (!selected || !alerts.length) return;
    var px = markPrice();
    if (!(px > 0)) return;
    var keep = [];
    var fired = [];
    alerts.forEach(function (a) {
      if (a.pool !== selected.pool) { keep.push(a); return; }
      var hit = a.dir === "above" ? px >= a.px : px <= a.px;
      if (hit) fired.push(a);
      else keep.push(a);
    });
    if (!fired.length) return;
    alerts = keep;
    saveAlerts();
    paintAlerts();
    fired.forEach(function (a) {
      var line = a.sym + " " + (a.dir === "above" ? "≥" : "≤") + " " + pxTxt(a.px) + " · last " + pxTxt(px);
      flashPaper(line);
      try {
        if (typeof Notification !== "undefined" && Notification.permission === "granted") {
          new Notification("Ketcharts", { body: line });
        }
      } catch (e) {}
    });
  }

  function notional() {
    var el = document.getElementById("notional");
    var v = n(el && el.value);
    if (!(v > 0)) v = 500;
    return v;
  }

  function fill(side) {
    var px = markPrice();
    if (!selected) { flashPaper("Pick a pair first."); return; }
    if (!(px > 0)) { flashPaper("No print yet — wait for a mark."); return; }
    if (paper.pos) {
      if (paper.pos.side === side) {
        flashPaper("Already " + side + " " + paper.pos.sym + ".");
        return;
      }
      closeFill();
    }
    var usdAmt = notional();
    if (usdAmt > paper.cash) usdAmt = paper.cash;
    if (usdAmt < 1) { flashPaper("Not enough cash."); return; }
    paper.cash -= usdAmt;
    paper.pos = {
      side: side,
      qty: usdAmt / px,
      entry: px,
      cost: usdAmt,
      sym: selected.sym,
      pool: selected.pool,
      opened: Date.now()
    };
    savePaper();
    paintPaper();
    flashPaper((side === "long" ? "Long" : "Short") + " " + selected.sym + " @ " + pxTxt(px));
  }

  function closeFill() {
    var px = markPrice() || (paper.pos && paper.pos.entry);
    var pos = paper.pos;
    if (!pos) { flashPaper("Flat."); return; }
    if (!(px > 0)) px = pos.entry;
    var pnl = (px - pos.entry) * pos.qty * (pos.side === "short" ? -1 : 1);
    var ret = pos.cost ? pnl / pos.cost : 0;
    paper.cash += pos.cost + pnl;
    paper.pos = null;
    var slip = {
      sym: pos.sym,
      side: pos.side,
      entry: pos.entry,
      exit: px,
      pnl: pnl,
      ret: ret,
      held: Date.now() - pos.opened,
      at: Date.now()
    };
    paper.slips.unshift(slip);
    paper.slips = paper.slips.slice(0, 12);
    savePaper();
    paintPaper();
    flashPaper("Closed " + pos.sym + "  " + (ret >= 0 ? "+" : "") + (ret * 100).toFixed(2) + "%");
    showSlip(slip);
  }

  function showSlip(s) {
    var m = document.getElementById("slip");
    if (!m) return;
    m.hidden = false;
    document.getElementById("slip-sym").textContent = s.sym;
    document.getElementById("slip-side").textContent = s.side.toUpperCase();
    var up = s.ret >= 0;
    var pctEl = document.getElementById("slip-pct");
    pctEl.textContent = (up ? "+" : "") + (s.ret * 100).toFixed(2) + "%";
    pctEl.className = "big " + (up ? "up" : "dn");
    document.getElementById("slip-pnl").textContent = (up ? "+" : "") + usd(Math.abs(s.pnl)).replace("$", (up ? "$" : "-$"));
    document.getElementById("slip-entry").textContent = usd(s.entry);
    document.getElementById("slip-exit").textContent = usd(s.exit);
    document.getElementById("slip-copy").onclick = function () {
      var line = s.sym + " " + s.side.toUpperCase() + " " + (up ? "+" : "") + (s.ret * 100).toFixed(2) + "% · paper on Ketcharts";
      if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(line);
    };
    document.getElementById("slip-png").onclick = function () { drawSlip(s); };
  }

  function drawSlip(s) {
    var c = document.createElement("canvas");
    c.width = 1200;
    c.height = 675;
    var x = c.getContext("2d");
    var lg = x.createLinearGradient(0, 0, 1200, 675);
    lg.addColorStop(0, "#0e0e0e");
    lg.addColorStop(0.45, "#12140c");
    lg.addColorStop(1, "#0a0a0a");
    x.fillStyle = lg;
    x.fillRect(0, 0, 1200, 675);
    var rg = x.createRadialGradient(80, -40, 20, 80, -40, 520);
    rg.addColorStop(0, "rgba(192,248,0,0.18)");
    rg.addColorStop(1, "rgba(192,248,0,0)");
    x.fillStyle = rg;
    x.fillRect(0, 0, 1200, 675);
    x.fillStyle = RED;
    x.font = "900 64px Arial Black, Impact, sans-serif";
    x.fillText("KETCHARTS", 72, 110);
    x.fillStyle = "#8aa090";
    x.font = "600 22px Sora, sans-serif";
    x.fillText("PAPER  ·  ROBINHOOD CHAIN", 72, 150);
    x.fillStyle = "#f2f3f4";
    x.font = "600 42px Sora, sans-serif";
    x.fillText(s.sym + "  " + s.side.toUpperCase(), 72, 250);
    x.fillStyle = s.ret >= 0 ? LIME : RED;
    x.font = "600 120px Sora, sans-serif";
    x.fillText((s.ret >= 0 ? "+" : "") + (s.ret * 100).toFixed(2) + "%", 72, 400);
    x.fillStyle = "#8aa090";
    x.font = "400 22px Sora, sans-serif";
    x.fillText("in " + usd(s.entry) + "   out " + usd(s.exit), 72, 460);
    x.fillStyle = "#8aa090";
    x.font = "400 18px Sora, sans-serif";
    x.fillText("Not a fill. Local paper only.", 72, 620);
    x.fillStyle = LIME;
    x.font = "600 18px Sora, sans-serif";
    x.fillText("vapurr", 1040, 620);
    c.toBlob(function (blob) {
      if (!blob) return;
      var a = document.createElement("a");
      a.href = URL.createObjectURL(blob);
      a.download = s.sym.toLowerCase() + "-ketcharts.png";
      a.click();
      if (navigator.clipboard && navigator.clipboard.write) {
        try { navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]); } catch (e) {}
      }
    }, "image/png");
  }

  function bind() {
    document.querySelectorAll("[data-tab]").forEach(function (el) {
      el.classList.toggle("on", el.getAttribute("data-tab") === tab);
      el.onclick = function () {
        tab = el.getAttribute("data-tab");
        document.querySelectorAll("[data-tab]").forEach(function (x) { x.classList.toggle("on", x === el); });
        if (tab === "majors") {
          selected = { pool: MAJORS[0].id, sym: "ETH", source: "binance", kind: "cash", px: 0 };
        } else if (!selected || (tab === "stock" && selected.kind !== "stock") || (tab === "meme" && selected.kind !== "meme") || (tab === "fav" && selected && !favs[selected.pool])) {
          selected = visible()[0] || selected;
        }
        savePref();
        paintList();
        paintTrench();
        paintHud();
        paintAlerts();
        bootChart();
      };
    });
    document.getElementById("q").addEventListener("input", function () {
      q = document.getElementById("q").value;
      paintList();
      if (searchTimer) clearTimeout(searchTimer);
      var qq = q.trim();
      if (qq.length < 2 || tab === "majors") return;
      searchTimer = setTimeout(function () {
        geckoSearch(qq).then(function (found) {
          found.forEach(putPair);
          paintList();
        });
      }, 280);
    });
    document.getElementById("buy").onclick = function () { fill("long"); };
    document.getElementById("short").onclick = function () { fill("short"); };
    document.getElementById("flat").onclick = closeFill;
    document.getElementById("slip-x").onclick = function () { document.getElementById("slip").hidden = true; };
    document.getElementById("vela").onclick = function (e) {
      e.preventDefault();
      g.vapurr.go("https://luxalgo.com/vela");
    };
    document.getElementById("shot").onclick = shotChart;
    document.getElementById("alert-up").onclick = function () { addAlert("above"); };
    document.getElementById("alert-dn").onclick = function () { addAlert("below"); };
    document.getElementById("copyca").onclick = function () {
      if (!selected || !selected.pool || selected.source === "binance") {
        flashPaper("No pool on this cut.");
        return;
      }
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(selected.pool);
        flashPaper("Copied " + selected.pool.slice(0, 10) + "…");
      }
    };
    document.addEventListener("keydown", function (e) {
      var tag = (e.target && e.target.tagName) || "";
      if (tag === "INPUT" || tag === "TEXTAREA") {
        if (e.key === "Escape") e.target.blur();
        return;
      }
      var k = e.key;
      if (k === "/" ) { e.preventDefault(); document.getElementById("q").focus(); return; }
      if (k === "Escape") { document.getElementById("slip").hidden = true; return; }
      var tfMap = { "1": "1", "2": "5", "3": "15", "4": "60", "5": "240", "6": "D", "7": "W", "8": "M" };
      if (tfMap[k]) { timeframe = tfMap[k]; savePref(); bootChart(); return; }
      if (k === "c" || k === "C") { priceStyle = "candles"; savePref(); paintStyles(); applyChartPrefs(); return; }
      if (k === "b" || k === "B") { priceStyle = "bars"; savePref(); paintStyles(); applyChartPrefs(); return; }
      if (k === "n" || k === "N") { priceStyle = "line"; savePref(); paintStyles(); applyChartPrefs(); return; }
      if (k === "g" || k === "G") { logScale = !logScale; savePref(); paintScale(); applyChartPrefs(); return; }
      if (k === "l" || k === "L") { fill("long"); return; }
      if (k === "s" || k === "S") { fill("short"); return; }
      if (k === "x" || k === "X") { closeFill(); return; }
    });
    window.addEventListener("resize", function () {
      try { if (chart && chart.resize) chart.resize(); } catch (e) {}
    });
    new MutationObserver(function () {
      if (chart && chart.setTheme) try { chart.setTheme(ketTheme()); } catch (e) {}
      colorVolume();
    }).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    setInterval(function () {
      if (paper.pos) paintPaper();
      checkAlerts();
    }, 1000);
    paintPaper();
    paintPills();
    paintInds();
    paintStyles();
    paintScale();
    paintRange();
    paintRisk();
    paintAlerts();
    paintHud();
    loadBoard().then(bootChart);
  }

  g.bootKetcharts = bind;
})(window);
