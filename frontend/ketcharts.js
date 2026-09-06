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
    { id: "30s", lab: "30s", gecko: "", agg: 1, paprika: "", ms: 3e4, fromTrades: true },
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
  var DX = "https://api.dexscreener.com";
  var GECKO = "https://api.geckoterminal.com/api/v2";
  var PAP = "https://api.dexpaprika.com";
  var CASH_ADDR = {
    "0x5fc5360d0400a0fd4f2af552add042d716f1d168": "USDG",
    "0x0bd7d308f8e1639fab988df18a8011f41eacad73": "WETH",
    "0x0000000000000000000000000000000000000000": "ETH"
  };
  var srcGate = {
    gecko: { cool: 0, fail: 0 },
    paprika: { cool: 0, fail: 0 },
    dex: { cool: 0, fail: 0 }
  };
  var boardStep = 0;
  var FEEDS = [
    "geckoTrend",
    "paprikaVol",
    "dex",
    "geckoNew",
    "paprikaNew",
    "geckoTop",
    "paprikaLiq",
    "paprikaStock"
  ];
  var PAPER_KEY = "vapurr.ketcharts.paper";
  var IND_KEY = "vapurr.ketcharts.inds";
  var SORT_KEY = "vapurr.ketcharts.sort";
  var FAV_KEY = "vapurr.ketcharts.fav";
  var ALERT_KEY = "vapurr.ketcharts.alerts";
  var PREF_KEY = "vapurr.ketcharts.pref";
  var BARS_KEY = "vapurr.ketcharts.bars.v1";
  var TAPE_KEY = "vapurr.ketcharts.tape.v1";
  var tab = "new";
  var q = "";
  var pairs = [];
  var selected = null;
  var timeframe = "60";
  var chart = null;
  var feed = null;
  var lastClose = 0;
  var lastBars = [];
  var lastTape = "";
  var hoverBar = null;
  var chartBound = false;
  var bootGen = 0;
  var barCache = {};
  var barFetch = {};
  var tapeSaveTimer = 0;
  var searchTimer = 0;
  var flashTimer = 0;
  var sortKey = "created";
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
  var ketSnap = null;
  var listBusy = false;
  var listWaitHash = "";
  var listAmtDirty = false;
  var railTab = "txns";
  (function loadPref() {
    try {
      var raw = JSON.parse(localStorage.getItem(PREF_KEY) || "null");
      if (!raw) return;
      if (raw.style) priceStyle = raw.style;
      if (raw.log) logScale = true;
      if (raw.pct) pctScale = true;
      if (raw.tf) timeframe = raw.tf;
      if (raw.tab === "meme") tab = "hot";
      else if (raw.tab === "listed") tab = "new";
      else if (raw.tab) tab = raw.tab;
      if (raw.pool) lastPool = raw.pool;
      if (tab !== "majors" && String(lastPool).indexOf("binance:") === 0) lastPool = "";
      if (raw.rail === "pair" || raw.rail === "paper" || raw.rail === "txns") railTab = raw.rail;
    } catch (e) {}
  })();
  function savePref() {
    try {
      localStorage.setItem(PREF_KEY, JSON.stringify({
        style: priceStyle, log: logScale, pct: pctScale, tf: timeframe, tab: tab,
        pool: selected && selected.pool, rail: railTab
      }));
    } catch (e) {}
  }
  function tabRows() {
    return tab === "majors" ? MAJORS.map(majorRow) : visible();
  }
  function poolInRows(pool, rows) {
    if (!pool || !rows) return null;
    var i;
    for (i = 0; i < rows.length; i++) if (rows[i].pool === pool) return rows[i];
    return null;
  }
  function isHousePair(p) {
    if (!p) return false;
    if (String(p.dex || "").toLowerCase().indexOf("house") >= 0) return true;
    return String(p.pool || "").toLowerCase() === "0x667bfcaf9d3ee809336788bf52511d35ae9c1bf7";
  }
  function preferHouse(rows) {
    var i;
    for (i = 0; i < (rows || []).length; i++) {
      if (isHousePair(rows[i]) && (n(rows[i].px) > 0 || rows[i].mid_ok)) return rows[i];
    }
    return null;
  }
  function ensureSelected() {
    var rows = tabRows();
    var hit = selected ? poolInRows(selected.pool, rows) : null;
    if (hit) { selected = hit; return selected; }
    if (lastPool) {
      hit = poolInRows(lastPool, rows);
      if (hit) { selected = hit; return selected; }
    }
    selected = preferHouse(rows) || rows[0] || null;
    return selected;
  }
  function paintRail() {
    document.querySelectorAll("[data-rail]").forEach(function (el) {
      var id = el.getAttribute("data-rail");
      el.classList.toggle("on", id === railTab);
      el.classList.toggle("hot", id === "paper" && !!paper.pos);
    });
    ["txns", "pair", "paper"].forEach(function (id) {
      var pane = document.getElementById("rail-" + id);
      if (pane) pane.classList.toggle("on", id === railTab);
    });
  }
  function setRail(id) {
    if (id !== "txns" && id !== "pair" && id !== "paper") return;
    railTab = id;
    savePref();
    paintRail();
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
      if (raw && typeof raw.cash === "number") {
        if (!Array.isArray(raw.book)) raw.book = Array.isArray(raw.slips) ? raw.slips.slice() : [];
        if (!Array.isArray(raw.resets)) raw.resets = [];
        if (!Array.isArray(raw.slips)) raw.slips = [];
        return raw;
      }
    } catch (e) {}
    return { cash: 10000, pos: null, slips: [], book: [], resets: [] };
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

  function packBars(bars) {
    return (bars || []).slice(-240).map(function (b) {
      return [b.time, b.open, b.high, b.low, b.close, b.volume];
    });
  }
  function unpackBars(rows) {
    return (rows || []).map(function (r) {
      return { time: n(r[0]), open: n(r[1]), high: n(r[2]), low: n(r[3]), close: n(r[4]), volume: n(r[5]) };
    }).filter(function (b) { return b.time > 0 && b.close > 0; });
  }
  function realBars(hit) {
    return !!(hit && hit.bars && hit.bars.length > 1 && hit.source && hit.source !== "empty" && hit.source !== "print");
  }
  function persistBars() {
    try {
      var keys = Object.keys(barCache);
      keys.sort(function (a, b) { return n(barCache[b] && barCache[b].at) - n(barCache[a] && barCache[a].at); });
      var out = {};
      var i;
      for (i = 0; i < keys.length && Object.keys(out).length < 16; i++) {
        var h = barCache[keys[i]];
        if (!realBars(h)) continue;
        out[keys[i]] = { bars: packBars(h.bars), source: h.source, at: h.at };
      }
      localStorage.setItem(BARS_KEY, JSON.stringify(out));
    } catch (e) {
      try { localStorage.removeItem(BARS_KEY); } catch (e2) {}
    }
  }
  function restoreBars() {
    try {
      var raw = JSON.parse(localStorage.getItem(BARS_KEY) || "null");
      if (!raw || typeof raw !== "object") return;
      Object.keys(raw).forEach(function (k) {
        var h = raw[k];
        if (!h || !h.bars || Date.now() - n(h.at) > 864e5) return;
        var bars = unpackBars(h.bars);
        if (bars.length < 2) return;
        barCache[k] = { bars: bars, source: h.source || "cache", at: n(h.at) };
      });
    } catch (e) {}
  }
  function slimPair(p) {
    if (!p) return null;
    return {
      pool: p.pool, token: p.token, quote: p.quote, quote_sym: p.quote_sym,
      sym: p.sym, name: p.name, dex: p.dex, fee: p.fee,
      px: p.px, chg: p.chg, vol: p.vol, vol1: p.vol1, vol6: p.vol6,
      liq: p.liq, mcap: p.mcap, buys: p.buys, sells: p.sells, txns: p.txns,
      created: p.created, kind: p.kind, source: p.source || "rhc",
      icon: p.icon || "", dx: p.dx || "", blurb: p.blurb ? String(p.blurb).slice(0, 180) : ""
    };
  }
  function persistTape() {
    try {
      var slim = [];
      var i;
      for (i = 0; i < pairs.length && slim.length < 80; i++) {
        if (pairs[i].source === "binance") continue;
        slim.push(slimPair(pairs[i]));
      }
      localStorage.setItem(TAPE_KEY, JSON.stringify({
        at: Date.now(),
        pool: selected && selected.pool,
        pairs: slim
      }));
    } catch (e) {
      try { localStorage.removeItem(TAPE_KEY); } catch (e2) {}
    }
  }
  function persistTapeSoon() {
    if (tapeSaveTimer) return;
    tapeSaveTimer = setTimeout(function () {
      tapeSaveTimer = 0;
      persistTape();
    }, 500);
  }
  function restoreTape() {
    try {
      var raw = JSON.parse(localStorage.getItem(TAPE_KEY) || "null");
      if (!raw || !Array.isArray(raw.pairs) || !raw.pairs.length) return false;
      raw.pairs.forEach(function (p) {
        if (p && p.pool) {
          p.seen = n(raw.at) || 1;
          putPair(p);
        }
      });
      return true;
    } catch (e) {
      return false;
    }
  }
  function cachedBars(pool, tf) {
    var spec = specOf(tf || timeframe);
    return barCache[String(pool || "").toLowerCase() + "|" + spec.id] || null;
  }
  function hydrateFromCache() {
    restoreBars();
    restoreTape();
    ensureSelected();
    if (selected && selected.source !== "binance") {
      var hit = cachedBars(selected.pool, timeframe);
      if (realBars(hit)) {
        lastBars = hit.bars;
        lastTape = hit.source;
        lastClose = hit.bars[hit.bars.length - 1].close;
      }
    }
    paintList();
    paintTrench();
    paintHud();
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
    if (v === 0) return "$0.00";
    return "—";
  }
  function signedUsd(x) {
    var v = n(x);
    if (v > 0) return "+" + usd(v);
    if (v < 0) return "−" + usd(-v);
    return "$0.00";
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

  function icoUrl(sym, addr) {
    if (window.tokenIconUrl) {
      var u = tokenIconUrl("", sym);
      if (u) return u;
      var a = String(addr || "").toLowerCase();
      if (a === "0x5fc5360d0400a0fd4f2af552add042d716f1d168") return tokenIconUrl("usdg");
      if (a === "0x0bd7d308f8e1639fab988df18a8011f41eacad73") return tokenIconUrl("weth");
    }
    return "";
  }
  function pairIcon(p) {
    if (p && (p.listedLogo || p.icon)) return p.listedLogo || p.icon;
    return icoUrl(p && p.sym, p && p.token);
  }
  function escTxt(s) {
    return String(s || "").replace(/[&<>"]/g, function (c) {
      return ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c];
    });
  }

  function mapLiqPair(p) {
    if (!p) return null;
    var base = p.base && typeof p.base === "object" ? p.base : {};
    var quoteObj = p.quote && typeof p.quote === "object" ? p.quote : {};
    var pool = String(p.pool || p.address || "").toLowerCase();
    if (!pool) return null;
    var sym = p.sym || base.symbol || "???";
    var qsym = p.quote_sym || quoteObj.symbol || "";
    var qaddr = String((typeof p.quote === "string" ? p.quote : quoteObj.address) || "").toLowerCase();
    return {
      pool: pool,
      token: String(p.token || base.address || "").toLowerCase(),
      quote: qaddr,
      quote_sym: qsym,
      sym: sym,
      name: p.name || (sym + (qsym ? " / " + qsym : "")),
      dex: p.dex || "",
      fee: p.fee || "",
      px: n((p.mid_ok && p.pool_mid != null) ? p.pool_mid : (p.pool_mid > 0 ? p.pool_mid : (p.px != null ? p.px : base.price_usd))),
      mid_ok: !!p.mid_ok,
      chg: n(p.chg),
      vol: n(p.vol != null ? p.vol : p.vol24_usd),
      vol1: n(p.vol1),
      vol6: n(p.vol6),
      liq: n(p.liq != null ? p.liq : p.reserve_usd),
      mcap: n(p.mcap),
      buys: n(p.buys),
      sells: n(p.sells),
      txns: n(p.txns),
      created: n(p.created),
      kind: p.kind === "stable" || p.kind === "eth" ? "cash" : kindOf(sym, p.name),
      source: "rhc"
    };
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

  function srcReady(name) {
    return Date.now() >= (srcGate[name] && srcGate[name].cool || 0);
  }
  function srcMark(name, r, gapOk) {
    var g = srcGate[name];
    if (!g) return !!(r && r.ok);
    var st = r && r.status;
    if (st === 429 || st === 402) {
      g.fail = (g.fail || 0) + 1;
      g.cool = Date.now() + Math.min(180000, 12000 * Math.pow(2, g.fail));
      return false;
    }
    g.fail = 0;
    g.cool = Date.now() + (gapOk || 8000);
    return !!(r && r.ok);
  }
  function fetchSrc(name, url, gapOk) {
    if (!srcReady(name)) {
      return Promise.resolve({ ok: false, status: 0, json: null, skipped: true });
    }
    return fetchJson(url).then(function (r) {
      srcMark(name, r, gapOk);
      return r;
    });
  }

  function dxPairsOf(json) {
    if (Array.isArray(json)) return json;
    if (json && Array.isArray(json.pairs)) return json.pairs;
    return [];
  }

  function parseDxPair(row, profile) {
    if (!row || String(row.chainId || "").toLowerCase() !== "robinhood") return null;
    var base = row.baseToken || {};
    var quote = row.quoteToken || {};
    var pool = String(row.pairAddress || "").toLowerCase();
    if (!pool) return null;
    var tx = (row.txns && row.txns.h24) || {};
    var vol = row.volume || {};
    var chg = row.priceChange || {};
    var liq = row.liquidity || {};
    var info = row.info || {};
    var sym = base.symbol || "???";
    var name = base.name || sym;
    var links = (profile && profile.links) || [];
    if (!links.length && info.websites) {
      links = (info.websites || []).map(function (w) {
        return { label: w.label || "Website", url: w.url };
      }).concat((info.socials || []).map(function (s) {
        return { type: s.type, url: s.url };
      }));
    }
    return {
      pool: pool,
      token: String(base.address || "").toLowerCase(),
      quote: String(quote.address || "").toLowerCase(),
      quote_sym: quote.symbol || "",
      sym: sym,
      name: name,
      dex: row.dexId || "uniswap",
      fee: (row.labels && row.labels.length) ? row.labels.join(" ") : "",
      px: n(row.priceUsd),
      chg: n(chg.h24),
      vol: n(vol.h24),
      vol1: n(vol.h1),
      vol6: n(vol.h6),
      liq: n(liq.usd),
      mcap: n(row.marketCap || row.fdv),
      buys: n(tx.buys),
      sells: n(tx.sells),
      txns: n(tx.buys) + n(tx.sells),
      created: n(row.pairCreatedAt),
      kind: kindOf(sym, name),
      source: "rhc",
      icon: (profile && profile.icon) || info.imageUrl || "",
      header: (profile && profile.header) || info.header || "",
      blurb: (profile && profile.description) || "",
      dx: row.url || (profile && profile.url) || "",
      links: links
    };
  }

  function putDexPair(p) {
    if (!p || !p.pool) return;
    var i;
    if (p.token) {
      for (i = 0; i < pairs.length; i++) {
        var old = pairs[i];
        if (!old || old.source === "binance") continue;
        if (old.token && old.token === p.token) {
          var pool = old.pool;
          var holders = old.holders;
          var seen = old.seen;
          pairs[i] = Object.assign({}, old, p);
          if (pool) pairs[i].pool = pool;
          if (seen) pairs[i].seen = seen;
          if (holders && !p.holders) pairs[i].holders = holders;
          if (old.icon && !p.icon) pairs[i].icon = old.icon;
          return;
        }
      }
    }
    putPair(p);
  }

  function loadDex() {
    if (!srcReady("dex")) return Promise.resolve();
    return fetchJson(DX + "/token-profiles/latest/v1").then(function (r) {
      if (!srcMark("dex", r, 25000) || !r.ok) return;
      var list = Array.isArray(r.json) ? r.json : [];
      var rh = [];
      var byTok = {};
      list.forEach(function (p) {
        if (!p || String(p.chainId || "").toLowerCase() !== "robinhood") return;
        var a = String(p.tokenAddress || "").toLowerCase();
        if (!a || byTok[a]) return;
        byTok[a] = p;
        rh.push(p);
      });
      if (!rh.length) return;
      var addrs = rh.map(function (p) { return p.tokenAddress; }).slice(0, 30);
      return fetchJson(DX + "/tokens/v1/robinhood/" + addrs.join(",")).then(function (t) {
        srcMark("dex", t, 25000);
        if (!t.ok) return;
        var rows = dxPairsOf(t.json);
        var best = {};
        rows.forEach(function (row) {
          if (String(row.chainId || "").toLowerCase() !== "robinhood") return;
          var tok = String((row.baseToken && row.baseToken.address) || "").toLowerCase();
          if (!tok) return;
          var liq = n(row.liquidity && row.liquidity.usd);
          if (!best[tok] || liq > n(best[tok].liquidity && best[tok].liquidity.usd)) best[tok] = row;
        });
        Object.keys(best).forEach(function (tok) {
          var mapped = parseDxPair(best[tok], byTok[tok]);
          if (mapped) putDexPair(mapped);
        });
      });
    });
  }

  function dxSearch(query) {
    var qq = encodeURIComponent(query);
    return fetchSrc("dex", DX + "/latest/dex/search?q=" + qq, 15000).then(function (r) {
      if (!r || r.skipped || !r.ok) return [];
      var out = [];
      dxPairsOf(r.json).forEach(function (row) {
        var p = parseDxPair(row, null);
        if (p) out.push(p);
      });
      return out;
    });
  }

  function paprikaTok(t) {
    if (!t) return { addr: "", sym: "", name: "", fdv: 0, h24: {} };
    var addr = String(t.id || t.address || "").toLowerCase();
    return {
      addr: addr,
      sym: t.symbol || CASH_ADDR[addr] || "",
      name: t.name || t.symbol || "",
      fdv: n(t.fdv),
      h24: t["24h"] || {},
      h1: t["1h"] || {},
      h6: t["6h"] || {}
    };
  }

  function parsePaprika(row) {
    if (!row) return null;
    if (row.chain && String(row.chain).toLowerCase() !== "robinhood") return null;
    var pool = String(row.id || row.address || "").toLowerCase();
    if (!pool) return null;
    var toks = row.tokens || [];
    var a = paprikaTok(toks[0]);
    var b = paprikaTok(toks[1]);
    if (CASH_ADDR[a.addr] && !CASH_ADDR[b.addr]) {
      var sw = a;
      a = b;
      b = sw;
    }
    var sym = a.sym || "???";
    return {
      pool: pool,
      token: a.addr,
      quote: b.addr,
      quote_sym: b.sym,
      sym: sym,
      name: a.name || sym,
      dex: row.dex_name || row.dex_id || "",
      fee: row.fee != null ? String(row.fee) : "",
      px: n(row.price_usd),
      chg: n(row.price_change_percentage_24h),
      vol: n(row.volume_usd_24h),
      vol1: n(a.h1 && a.h1.volume_usd),
      vol6: n(a.h6 && a.h6.volume_usd),
      liq: n(row.liquidity_usd),
      mcap: n(a.fdv),
      buys: n(a.h24.buys),
      sells: n(a.h24.sells),
      txns: n(row.transactions_24h || a.h24.txns),
      created: row.created_at ? Date.parse(row.created_at) : 0,
      kind: kindOf(sym, a.name),
      source: "rhc"
    };
  }

  function loadPaprika(order) {
    var url = PAP + "/networks/robinhood/pools/search?order_by=" + encodeURIComponent(order) +
      "&sort=desc&limit=30&detailed=true";
    return fetchSrc("paprika", url, 10000).then(function (r) {
      if (!r || r.skipped || !r.ok) return;
      var rows = (r.json && (r.json.results || r.json.pools)) || [];
      rows.forEach(function (row) {
        var p = parsePaprika(row);
        if (p) putDexPair(p);
      });
    });
  }

  function loadGecko(kind) {
    var path = kind === "pools"
      ? GECKO + "/networks/robinhood/pools?page=1"
      : GECKO + "/networks/robinhood/" + kind + "?page=1";
    return fetchSrc("gecko", path, 12000).then(function (r) {
      if (!r || r.skipped || !r.ok) return;
      parseGecko((r.json && r.json.data) || []).forEach(putDexPair);
    });
  }

  function paprikaSearch(query) {
    var url = PAP + "/pools/search?chains=robinhood&query=" + encodeURIComponent(query) +
      "&limit=20&detailed=true";
    return fetchSrc("paprika", url, 8000).then(function (r) {
      if (!r || r.skipped || !r.ok) return [];
      var rows = (r.json && (r.json.results || r.json.pools)) || [];
      var out = [];
      rows.forEach(function (row) {
        var p = parsePaprika(row);
        if (p) out.push(p);
      });
      return out;
    });
  }

  function runFeed(feed) {
    if (feed.indexOf("gecko") === 0 && !srcReady("gecko")) return null;
    if (feed.indexOf("paprika") === 0 && !srcReady("paprika")) return null;
    if (feed === "dex" && !srcReady("dex")) return null;
    if (feed === "geckoTrend") return loadGecko("trending_pools");
    if (feed === "geckoNew") return loadGecko("new_pools");
    if (feed === "geckoTop") return loadGecko("pools");
    if (feed === "paprikaVol") return loadPaprika("volume_usd_24h");
    if (feed === "paprikaNew") return loadPaprika("created_at");
    if (feed === "paprikaLiq") return loadPaprika("liquidity_usd");
    if (feed === "paprikaStock") {
      var s = STOCK_BOOT[boardStep % STOCK_BOOT.length];
      return paprikaSearch(s).then(function (found) {
        (found || []).forEach(putDexPair);
      });
    }
    if (feed === "dex") return loadDex();
    return null;
  }

  function loadNextScreener() {
    var i;
    for (i = 0; i < FEEDS.length; i++) {
      var feed = FEEDS[boardStep % FEEDS.length];
      boardStep += 1;
      var job = runFeed(feed);
      if (job) return job;
    }
    return Promise.resolve();
  }

  function searchScreeners(qq) {
    var order = [];
    if (srcReady("paprika")) order.push("paprika");
    if (srcReady("gecko")) order.push("gecko");
    if (srcReady("dex")) order.push("dex");
    function one(name) {
      if (name === "paprika") return paprikaSearch(qq);
      if (name === "gecko") return geckoSearch(qq);
      if (name === "dex") return dxSearch(qq);
      return Promise.resolve([]);
    }
    if (!order.length) return Promise.resolve([]);
    return one(order[0]).then(function (found) {
      if (found && found.length) return found;
      if (order[1]) return one(order[1]);
      return found || [];
    });
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
    if (!old) {
      p.seen = Date.now();
      pairs.push(p);
    } else {
      var seen = old.seen;
      var holders = old.holders;
      pairs[i] = Object.assign({}, old, p);
      pairs[i].seen = seen;
      if (holders && !p.holders) pairs[i].holders = holders;
    }
  }

  function cmpPairs(a, b) {
    if (sortKey === "sym") {
      return String(a.sym || "").localeCompare(String(b.sym || "")) * sortDir;
    }
    return (n(a[sortKey]) - n(b[sortKey])) * sortDir;
  }

  function loadTape() {
    return fetchJson("/liq/api/tape").then(function (r) {
      var list = (r.json && r.json.pairs) || [];
      list.forEach(function (row) {
        var p = mapLiqPair(row);
        if (p) putPair(p);
      });
      if (r.json && r.json.ok) setLive(true, "rhc", false);
      persistTapeSoon();
      return r;
    });
  }

  function loadBoard() {
    return loadTape().then(function () {
      document.querySelectorAll("[data-tab]").forEach(function (x) {
        x.classList.toggle("on", x.getAttribute("data-tab") === tab);
      });
      ensureSelected();
      paintList();
      paintTrench();
      paintTxns();
      return loadPaprika("volume_usd_24h").then(function () {
        ensureSelected();
        paintList();
        return loadGecko("trending_pools");
      }).then(function () {
        paintList();
        return loadDex();
      }).then(function () {
        var had = selected && selected.pool;
        ensureSelected();
        paintList();
        paintTrench();
        paintHud();
        persistTapeSoon();
        if (selected && selected.pool !== had) bootChart();
      }).catch(function () {});
    });
  }

  function geckoSearch(query) {
    var qq = encodeURIComponent(query);
    return fetchSrc("gecko", GECKO + "/search/pools?query=" + qq + "&network=robinhood", 10000)
      .then(function (r) {
        if (!r || r.skipped || !r.ok) return [];
        return parseGecko((r.json && r.json.data) || []);
      });
  }

  function visible() {
    var qq = q.trim().toLowerCase();
    return pairs.filter(function (p) {
      if (p.source === "binance") return false;
      if (tab === "stock" && p.kind !== "stock") return false;
      if (tab === "hot" && p.kind === "cash") return false;
      if (tab === "gain" && p.kind === "cash") return false;
      if (tab === "fav" && !favs[p.pool]) return false;
      if (tab === "listed" && !p.listed) return false;
      if (qq && (p.sym + " " + (p.quote_sym || "") + " " + p.name + " " + p.token + " " + p.pool).toLowerCase().indexOf(qq) < 0) return false;
      return true;
    }).sort(function (a, b) {
      if (tab === "listed" && (sortKey === "created" || sortKey === "listedPaid")) return n(b.listedPaid) - n(a.listedPaid);
      if (tab === "gain" && sortKey === "created") return (n(b.chg) - n(a.chg));
      if (tab === "hot" && sortKey === "created") return (n(b.vol) - n(a.vol));
      return cmpPairs(a, b);
    });
  }

  function feedTitle() {
    if (tab === "new") return "New pairs";
    if (tab === "hot") return "Hot";
    if (tab === "gain") return "Gainers";
    if (tab === "stock") return "Stocks";
    if (tab === "fav") return "Watch";
    if (tab === "listed") return "Listed";
    if (tab === "majors") return "CEX";
    return "Pairs";
  }

  function paintCols() {
    var box = document.getElementById("cols");
    if (!box) return;
    box.innerHTML = "";
    box.appendChild(document.createElement("span"));
    box.appendChild(document.createElement("span"));
    var cols = [
      { k: "sym", lab: "Pair" },
      { k: "px", lab: "Last", num: 1 },
      {
        k: tab === "new" ? "created" : tab === "listed" ? "listedPaid" : "chg",
        lab: tab === "new" ? "Age" : tab === "listed" ? "Paid" : "24h",
        num: 1
      }
    ];
    cols.forEach(function (c) {
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
    try { stampListed(); } catch (e) {}
    var head = document.getElementById("feed-head");
    if (head) head.textContent = feedTitle();
    paintCols();
    var favn = document.getElementById("favn");
    if (favn) {
      var nFav = Object.keys(favs).length;
      favn.textContent = nFav ? nFav : "";
    }
    var box = document.getElementById("rows");
    if (!box) return;
    box.innerHTML = "";
    var rows = tab === "majors" ? MAJORS.map(majorRow) : visible();
    var picked = false;
    if (rows.length && (!selected || !poolInRows(selected.pool, rows))) {
      var pick = preferHouse(rows);
      var i;
      if (!pick) {
        pick = rows[0];
        for (i = 0; i < rows.length; i++) {
          if (n(rows[i].px) > 0 || rows[i].source === "binance") { pick = rows[i]; break; }
        }
      }
      selected = pick;
      picked = true;
    }
    if (!rows.length) {
      box.innerHTML = "<div class='empty'>" +
        (tab === "listed"
          ? "Pay $PUSD to list a token. Organic pairs stay on New / Hot."
          : tab === "fav" ? "Star pairs to build a watchlist."
          : tab === "stock" ? "No RHC stock pools in this cut yet."
          : pairs.length ? "No pairs in this cut."
          : "Fetching RHC pairs…") +
        "</div>";
      return;
    }
    rows.forEach(function (p) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = "pair" + (selected && selected.pool === p.pool ? " on" : "");
      var up = n(p.chg) >= 0;
      var fresh = p.created && (Date.now() - p.created) < 300000;
      if (fresh) b.className += " fresh";
      if (p.created) b.setAttribute("data-created", String(p.created));
      b.innerHTML =
        "<span class='star'></span>" +
        "<span class='ico-wrap'><img class='ico' alt=''/><span class='av'></span></span>" +
        "<span class='mid'><span class='sym'></span><span class='sub'></span></span>" +
        "<span class='px'></span>" +
        "<span class='chg'></span>";
      var st = b.querySelector(".star");
      st.textContent = favs[p.pool] ? "★" : "☆";
      st.className = "star" + (favs[p.pool] ? " on" : "");
      var ico = b.querySelector(".ico");
      var av = b.querySelector(".av");
      var src = pairIcon(p);
      function showAv() {
        ico.style.display = "none";
        av.style.display = "grid";
        av.textContent = String(p.sym || "?").slice(0, 2).toUpperCase();
      }
      if (src) {
        ico.src = src;
        ico.style.display = "block";
        av.style.display = "none";
        ico.onerror = showAv;
      } else {
        showAv();
      }
      var symEl = b.querySelector(".sym");
      var lab = document.createElement("span");
      lab.className = "lab";
      lab.textContent = p.sym || "???";
      symEl.textContent = "";
      symEl.appendChild(lab);
      if (p.listed) {
        var tag = document.createElement("span");
        tag.className = "tag";
        tag.textContent = "LIST";
        symEl.appendChild(tag);
      }
      var bits = [];
      if (tab === "new") bits.push(ageTxt(p.created));
      if (tab === "listed") bits.push(p.listedPaid ? (Math.floor(n(p.listedPaid)) + " PUSD") : "unlisted");
      if (p.source !== "binance" && p.vol) bits.push(usd(p.vol));
      else if (p.name && p.name !== p.sym) bits.push(p.name);
      b.querySelector(".sub").textContent = bits.join(" · ");
      b.querySelector(".px").textContent = p.px ? usd(p.px) : "—";
      var ch = b.querySelector(".chg");
      if (tab === "new") {
        ch.textContent = ageTxt(p.created);
        ch.className = "chg";
      } else if (tab === "listed") {
        ch.textContent = p.listedPaid ? (Math.floor(n(p.listedPaid)) + " P") : "—";
        ch.className = "chg";
      } else {
        ch.textContent = pct(p.chg);
        ch.className = "chg " + (up ? "up" : "dn");
      }
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
        hoverBar = null;
        savePref();
        paintList();
        paintTrench();
        paintTxns();
        paintHud();
        bootChart();
      };
      box.appendChild(b);
    });
    var on = box.querySelector(".pair.on");
    if (on && on.scrollIntoView) on.scrollIntoView({ block: "nearest" });
    if (picked) savePref();
    return picked;
  }

  function ageTxt(ms) {
    if (!(ms > 0)) return "—";
    var s = Math.max(0, (Date.now() - ms) / 1000);
    if (s < 60) return Math.round(s) + "s";
    if (s < 3600) return Math.round(s / 60) + "m";
    if (s < 86400) return (s / 3600).toFixed(1) + "h";
    return (s / 86400).toFixed(1) + "d";
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
    try { paintTrenchInner(); } catch (e) {}
  }
  function paintTrenchInner() {
    var p = selected;
    var el = document.getElementById("trench");
    if (!el) return;
    if (!p || p.source === "binance") {
      el.innerHTML = "<p class='hint'>Pick a Robinhood Chain pair. CEX majors have no pool.</p>";
      return;
    }
    var tx = n(p.txns) || (n(p.buys) + n(p.sells));
    el.innerHTML =
      "<div class='stats'>" +
        "<div><span>Price</span><b>" + (p.px ? usd(p.px) : "—") + "</b></div>" +
        "<div><span>24h</span><b class='" + (n(p.chg) >= 0 ? "up" : "dn") + "'>" + pct(p.chg) + "</b></div>" +
        "<div><span>Liq</span><b>" + usd(p.liq) + "</b></div>" +
        "<div><span>Vol 24h</span><b>" + usd(p.vol) + "</b></div>" +
        "<div><span>Vol 1h</span><b>" + usd(p.vol1) + "</b></div>" +
        "<div><span>Vol 6h</span><b>" + usd(p.vol6) + "</b></div>" +
        "<div><span>Buys</span><b class='up'>" + (p.buys || "—") + "</b></div>" +
        "<div><span>Sells</span><b class='dn'>" + (p.sells || "—") + "</b></div>" +
        "<div><span>Tx 24h</span><b>" + (tx || "—") + "</b></div>" +
        "<div><span>Dex</span><b>" + (p.dex || "—") + "</b></div>" +
        "<div><span>Fee</span><b>" + (p.fee || "—") + "</b></div>" +
        "<div><span>Token</span><b>" + (p.token ? p.token.slice(0, 6) + "…" : "—") + "</b></div>" +
        (p.listed ? "<div><span>Listed</span><b>#" + (p.listedRank || "—") + " · " + Math.floor(n(p.listedPaid)) + " PUSD</b></div>" : "<div><span>Listed</span><b>No</b></div>") +
      "</div>" +
      (p.blurb ? "<p class='listed-bio'>" + escTxt(p.blurb).slice(0, 280) + "</p>" : "") +
      "<div class='pair-acts'>" +
        "<button class='btn ghost' type='button' id='go-scan'>Scan</button>" +
        "<button class='btn ghost' type='button' id='go-swap'>Swap</button>" +
        "<button class='btn ghost' type='button' id='copy-token'>Copy CA</button>" +
        "<button class='btn ghost' type='button' id='copy-pool'>Copy pool</button>" +
        "<button class='btn ghost' type='button' id='go-list'>" + (p.listed ? "Raise list" : "List") + "</button>" +
        (p.dx ? "<button class='btn ghost' type='button' id='go-dx'>DexScreener</button>" : "") +
        (p.links || []).slice(0, 3).map(function (l, i) {
          var lab = l.label || l.type || "link";
          return "<button class='btn ghost' type='button' data-href='" + escTxt(l.url) + "'>" + escTxt(lab) + "</button>";
        }).join("") +
      "</div>" +
      "<p class='hint'>Robinhood Chain. Tape is ours. Screeners rotate Gecko · Paprika · DexScreener so none of them 429.</p>";
    document.getElementById("go-scan").onclick = function () {
      if (p.token) g.vapurr.go("vapurr://scan?q=" + encodeURIComponent(p.token));
    };
    document.getElementById("go-swap").onclick = function () {
      g.vapurr.send({ cmd: "pane", id: "swap" });
    };
    document.getElementById("copy-token").onclick = function () {
      if (p.token && navigator.clipboard) navigator.clipboard.writeText(p.token);
      flashPaper("Copied token");
    };
    document.getElementById("copy-pool").onclick = function () {
      if (p.pool && navigator.clipboard) navigator.clipboard.writeText(p.pool);
      flashPaper("Copied pool");
    };
    var listBtn = document.getElementById("go-list");
    if (listBtn) listBtn.onclick = function () { openListSheet(p); };
    var dxb = document.getElementById("go-dx");
    if (dxb && p.dx) {
      dxb.onclick = function () { g.vapurr.go(p.dx); };
    }
    el.querySelectorAll("[data-href]").forEach(function (b) {
      b.onclick = function () { g.vapurr.go(b.getAttribute("data-href")); };
    });
    pullHolders(p);
  }

  var lastTrades = [];
  function paintTxns() {
    var box = document.getElementById("txns");
    if (!box) return;
    var p = selected;
    if (!p || p.source === "binance") {
      box.innerHTML = "<div class='empty'>Pick an RHC pair.</div>";
      lastTrades = [];
      return;
    }
    box.innerHTML = "<div class='empty'>Loading swaps…</div>";
    var pool = p.pool;
    Promise.all([
      fetchJson("/liq/api/trades/" + encodeURIComponent(pool)),
      geckoTradesFull(pool).catch(function () { return []; })
    ]).then(function (res) {
      if (!selected || selected.pool !== pool) return;
      var ours = ((res[0].json && res[0].json.trades) || []).map(function (t) {
        return {
          time: n(t.time), px: n(t.px), vol: n(t.vol), buy: !!t.buy,
          tx: t.tx || "", src: "rhc"
        };
      });
      var gecko = res[1] || [];
      var rows = ours.length ? ours : gecko;
      rows.sort(function (a, b) { return n(b.time) - n(a.time); });
      lastTrades = rows.slice(0, 40);
      if (!lastTrades.length) {
        box.innerHTML = "<div class='empty'>" + ((res[0].json && res[0].json.loading) ? "Waiting on logs…" : "No swaps in this window.") + "</div>";
        return;
      }
      box.innerHTML = "";
      lastTrades.forEach(function (t) {
        var b = document.createElement("button");
        b.type = "button";
        b.className = "txn";
        b.innerHTML = "<span class='side'></span><span class='when'></span><span class='amt'></span>";
        b.querySelector(".side").textContent = t.buy ? "BUY" : "SELL";
        b.querySelector(".side").className = "side " + (t.buy ? "up" : "dn");
        b.querySelector(".when").textContent = t.time ? ageTxt(t.time) : "—";
        b.querySelector(".amt").textContent = usd(t.vol);
        b.querySelector(".amt").className = "amt " + (t.buy ? "up" : "dn");
        b.onclick = function () {
          if (t.tx) g.vapurr.go("vapurr://scan?q=" + encodeURIComponent(t.tx));
        };
        box.appendChild(b);
      });
    });
  }

  function pullHolders(p) {
    if (!p || !p.token || p.source === "binance" || p.holders != null) return;
    if (!srcReady("gecko")) return;
    fetchSrc("gecko", GECKO + "/networks/robinhood/tokens/" + encodeURIComponent(p.token), 15000)
      .then(function (r) {
        var a = (((r.json || {}).data || {}).attributes) || {};
        var h = a.holders_count || a.holders || a.normalized_holders;
        if (h == null) return;
        p.holders = typeof h === "object" ? n(h.count || h.value) : n(h);
        if (selected && selected.pool === p.pool) {
          selected.holders = p.holders;
          paintTrench();
        }
      });
  }

  function tickBoard() {
    return loadTape().then(function () { return loadNextScreener(); }).then(function () {
      if (selected && selected.source !== "binance") {
        var i;
        for (i = 0; i < pairs.length; i++) {
          if (pairs[i].pool === selected.pool || (selected.token && pairs[i].token === selected.token)) {
            selected = pairs[i];
            break;
          }
        }
      }
      paintList();
      paintTrench();
      paintHud();
      paintTxns();
      persistTapeSoon();
    });
  }

  function paintAges() {
    if (tab !== "new") return;
    document.querySelectorAll("#rows .pair[data-created]").forEach(function (el) {
      var t = ageTxt(n(el.getAttribute("data-created")));
      var ch = el.querySelector(".chg");
      if (ch) ch.textContent = t;
    });
  }

  function markPrice() {
    var px = 0;
    if (lastBars.length) px = n(lastBars[lastBars.length - 1].close);
    if (!(px > 0) && selected) px = n(selected.px);
    if (!(px > 0)) px = n(lastClose);
    if (px > 0) lastClose = px;
    return px;
  }

  function bookStats() {
    var trades = paper.book || [];
    var realized = 0;
    var gp = 0;
    var gl = 0;
    var w = 0;
    var l = 0;
    var maxW = 0;
    var maxL = 0;
    var i;
    for (i = 0; i < trades.length; i++) {
      var pnl = n(trades[i].pnl);
      realized += pnl;
      if (pnl >= 0) {
        w += 1;
        gp += pnl;
        if (pnl > maxW) maxW = pnl;
      } else {
        l += 1;
        gl += -pnl;
        if (pnl < maxL) maxL = pnl;
      }
    }
    var injected = 0;
    var resets = paper.resets || [];
    for (i = 0; i < resets.length; i++) injected += n(resets[i].injected);
    return {
      n: trades.length,
      realized: realized,
      gp: gp,
      gl: gl,
      w: w,
      l: l,
      maxW: maxW,
      maxL: maxL,
      injected: injected,
      resets: resets.length
    };
  }

  function paintPaper() {
    var px = markPrice();
    var st = bookStats();
    var cash = document.getElementById("cash");
    if (cash) cash.textContent = usd(paper.cash);
    var mark = document.getElementById("mark");
    if (mark) mark.textContent = px ? usd(px) : "—";
    var real = document.getElementById("life-real");
    if (real) {
      real.textContent = signedUsd(st.realized);
      real.className = "real " + (st.realized > 0 ? "up" : st.realized < 0 ? "dn" : "");
    }
    var gl = document.getElementById("life-gl");
    if (gl) gl.textContent = st.gl ? ("−" + usd(st.gl)) : "$0.00";
    var gp = document.getElementById("life-gp");
    if (gp) gp.textContent = signedUsd(st.gp);
    var wl = document.getElementById("life-wl");
    if (wl) wl.textContent = st.w + "–" + st.l;
    var wr = document.getElementById("life-wr");
    if (wr) wr.textContent = st.n ? ((st.w / st.n) * 100).toFixed(0) + "%" : "—";
    var maxl = document.getElementById("life-maxl");
    if (maxl) maxl.textContent = st.maxL < 0 ? signedUsd(st.maxL) : "—";
    var refill = document.getElementById("life-refill");
    if (refill) refill.textContent = st.resets ? (st.resets + " · " + usd(st.injected)) : "0";
    var pos = document.getElementById("pos");
    var pnlEl = document.getElementById("pnl");
    var buy = document.getElementById("buy");
    var sh = document.getElementById("short");
    if (pos) {
      if (!paper.pos) {
        pos.textContent = "Flat";
        if (pnlEl) { pnlEl.textContent = "—"; pnlEl.className = ""; }
        if (buy) buy.textContent = "Long";
        if (sh) sh.textContent = "Short";
      } else {
        var last = px || paper.pos.entry;
        var pnl = (last - paper.pos.entry) * paper.pos.qty * (paper.pos.side === "short" ? -1 : 1);
        var p = paper.pos.cost ? (pnl / paper.pos.cost) * 100 : 0;
        var adds = (paper.pos.adds && paper.pos.adds.length) || 1;
        pos.textContent = paper.pos.side.toUpperCase() + " " + paper.pos.sym + "  " + adds + " fill" + (adds > 1 ? "s" : "") + "  avg " + pxTxt(paper.pos.entry);
        if (pnlEl) {
          pnlEl.textContent = signedUsd(pnl) + "  " + pct(p);
          pnlEl.className = p >= 0 ? "up" : "dn";
        }
        if (buy) buy.textContent = paper.pos.side === "long" ? "Add long" : "Long";
        if (sh) sh.textContent = paper.pos.side === "short" ? "Add short" : "Short";
      }
    }
    var box = document.getElementById("book");
    if (box) {
      box.innerHTML = "";
      var rows = (paper.book || []).slice().reverse().slice(0, 24);
      if (!rows.length) {
        box.innerHTML = "<div class='hint' style='margin:0'>No closed tickets yet.</div>";
      } else {
        rows.forEach(function (s) {
          var row = document.createElement("div");
          row.className = "row";
          var a = document.createElement("span");
          a.className = "sym";
          a.textContent = (s.side === "short" ? "S " : "L ") + s.sym;
          var b = document.createElement("span");
          b.className = n(s.pnl) >= 0 ? "up" : "dn";
          b.textContent = pct(n(s.ret) * 100);
          var c = document.createElement("span");
          c.className = n(s.pnl) >= 0 ? "up" : "dn";
          c.textContent = signedUsd(s.pnl);
          row.appendChild(a);
          row.appendChild(b);
          row.appendChild(c);
          box.appendChild(row);
        });
      }
    }
    paintRail();
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

  function setLoad(on) {
    var el = document.getElementById("load");
    if (el) el.hidden = !on;
  }

  function bindChartEvents() {
    if (!chart || !chart.on || chartBound) return;
    chartBound = true;
    try { chart.on("load:start", function () { setLoad(true); }); } catch (e) {}
    try { chart.on("load:end", function () { setLoad(false); paintHud(); }); } catch (e) {}
  }

  function bindHover() {
    var plot = document.getElementById("plot");
    if (!plot || plot.getAttribute("data-hover") === "1") return;
    plot.setAttribute("data-hover", "1");
    plot.addEventListener("mousemove", function (e) {
      if (!lastBars.length) return;
      var r = plot.getBoundingClientRect();
      if (!(r.width > 40)) return;
      var x = e.clientX - r.left;
      var padL = 48;
      var padR = 52;
      var w = Math.max(1, r.width - padL - padR);
      var t = Math.max(0, Math.min(1, (x - padL) / w));
      var i = Math.round(t * (lastBars.length - 1));
      hoverBar = lastBars[i] || null;
      paintHud();
    });
    plot.addEventListener("mouseleave", function () {
      hoverBar = null;
      paintHud();
    });
  }

  function majorRow(m) {
    return {
      pool: m.id, token: m.id, sym: m.lab, name: m.lab,
      px: n(m.px), chg: n(m.chg), vol: n(m.vol), liq: 0,
      kind: "cash", source: "binance"
    };
  }

  function hydrateMajors() {
    MAJORS.forEach(function (m) {
      var sym = String(m.id).split(":")[1];
      if (!sym) return;
      fetchJson("https://api.binance.com/api/v3/ticker/24hr?symbol=" + encodeURIComponent(sym)).then(function (r) {
        var j = r.json;
        if (!j || !j.lastPrice) return;
        m.px = n(j.lastPrice);
        m.chg = n(j.priceChangePercent);
        m.vol = n(j.quoteVolume);
        if (selected && selected.pool === m.id) {
          selected.px = m.px;
          selected.chg = m.chg;
          selected.vol = m.vol;
          lastClose = m.px;
          hoverBar = null;
          lastBars = [];
          paintHud();
        }
        if (tab === "majors") paintList();
      });
    });
  }

  function listedRows() {
    return tab === "majors" ? MAJORS.map(majorRow) : visible();
  }

  function stepPair(dir) {
    var rows = listedRows();
    if (!rows.length) return;
    var i = 0;
    if (selected) {
      for (i = 0; i < rows.length; i++) if (rows[i].pool === selected.pool) break;
      if (i >= rows.length) i = 0;
    }
    i = (i + dir + rows.length) % rows.length;
    selected = rows[i];
    lastTape = "";
    lastBars = [];
    hoverBar = null;
    savePref();
    paintList();
    paintTrench();
    paintHud();
    bootChart();
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
    var last = hoverBar || (bars.length ? bars[bars.length - 1] : null);
    var o = last ? last.open : p.px;
    var h = last ? last.high : p.px;
    var l = last ? last.low : p.px;
    var c = last ? last.close : p.px;
    var px = markPrice();
    var up = n(p.chg) >= 0;
    var barUp = n(c) >= n(o);
    var dlt = n(o) > 0 ? ((n(c) - n(o)) / n(o)) * 100 : 0;
    if (quote) {
      var qico = document.getElementById("qico");
      var qav = document.getElementById("qav");
      var src = pairIcon(p);
      if (qico) {
        if (src) {
          qico.hidden = false;
          qico.src = src;
          qico.onerror = function () {
            qico.hidden = true;
            if (qav) {
              qav.hidden = false;
              qav.textContent = String(p.sym || "?").slice(0, 2).toUpperCase();
            }
          };
          if (qav) qav.hidden = true;
        } else {
          qico.hidden = true;
          if (qav) {
            qav.hidden = false;
            qav.textContent = String(p.sym || "?").slice(0, 2).toUpperCase();
          }
        }
      }
      quote.querySelector(".sym").textContent = p.quote_sym ? (p.sym + " / " + p.quote_sym) : p.sym;
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
    var de = hud.querySelector(".d");
    if (de) {
      de.textContent = last ? pct(dlt) : "—";
      de.className = "d " + (dlt >= 0 ? "up" : "dn");
    }
    var ve = hud.querySelector(".v");
    if (ve) ve.textContent = last && last.volume ? usd(last.volume) : "—";
    var utc = document.getElementById("utc");
    if (utc) {
      utc.textContent = last && last.time
        ? new Date(last.time).toISOString().slice(11, 19) + " UTC"
        : "UTC";
    }
    var msg = "";
    if (p.source === "binance") msg = "";
    else if (lastTape === "print") msg = "No tape yet — last print only.";
    else if (lastTape === "prints") msg = "30s from last prints.";
    else if (lastTape === "paprika") msg = "Thin tape · DexPaprika fallback.";
    else if (lastTape === "empty") msg = "No tape and no print for this pool.";
    if (note) note.textContent = msg;
    paintPaper();
    checkAlerts();
    checkStops();
  }

  function checkStops() {
    var pos = paper.pos;
    if (!pos) return;
    var px = markPrice();
    if (!(px > 0)) return;
    var hit = "";
    if (pos.tp > 0) {
      if (pos.side === "long" && px >= pos.tp) hit = "TP";
      if (pos.side === "short" && px <= pos.tp) hit = "TP";
    }
    if (!hit && pos.sl > 0) {
      if (pos.side === "long" && px <= pos.sl) hit = "SL";
      if (pos.side === "short" && px >= pos.sl) hit = "SL";
    }
    if (!hit) return;
    flashPaper(hit + " hit @ " + pxTxt(px));
    closeFill();
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

  function ensureBars(bars, spec) {
    // Honesty: real on-chain only. Never invent flat series from oracle/mark.
    var list = bars || [];
    if (list.length >= 2) return list;
    return list.length === 1 ? list : [];
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
    if (!srcReady("gecko")) return Promise.resolve([]);
    var lim = spec.ms <= 6e4 ? 1000 : (spec.bucketMs ? 400 : 400);
    if (spec.id === "W" || spec.id === "M") lim = 400;
    if (n(limit) > 0) lim = Math.min(lim, n(limit));
    var url = GECKO + "/networks/robinhood/pools/" +
      encodeURIComponent(pool) + "/ohlcv/" + spec.gecko +
      "?aggregate=" + spec.agg + "&limit=" + lim + "&currency=usd";
    return fetchSrc("gecko", url, 6000).then(function (r) {
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
    if (!srcReady("paprika")) return Promise.resolve([]);
    var url = PAP + "/networks/robinhood/pools/" +
      encodeURIComponent(pool) + "/ohlcv?interval=" + encodeURIComponent(spec.paprika) +
      "&limit=" + lim + "&start=" + encodeURIComponent(start);
    return fetchSrc("paprika", url, 6000).then(function (r) {
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

  function geckoTradesFull(pool) {
    if (!srcReady("gecko")) return Promise.resolve([]);
    var url = GECKO + "/networks/robinhood/pools/" +
      encodeURIComponent(pool) + "/trades";
    return fetchSrc("gecko", url, 8000).then(function (r) {
      if (!r.ok || !r.json) return [];
      return (r.json.data || []).slice(0, 80).map(function (row) {
        var a = row.attributes || {};
        return {
          time: Date.parse(a.block_timestamp || ""),
          px: tradePx(a),
          vol: n(a.volume_in_usd),
          buy: a.kind === "buy",
          tx: a.tx_hash || a.transaction_hash || "",
          src: "gecko"
        };
      }).filter(function (t) { return t.time > 0 && t.px > 0; })
        .sort(function (a, b) { return a.time - b.time; });
    });
  }

  function geckoTrades(pool) {
    return geckoTradesFull(pool);
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

  function nativeId(spec) {
    if (!spec || spec.fromTrades) return "trades";
    if (spec.ms <= 18e5) return "1";
    if (spec.ms <= 144e5) return "60";
    return "D";
  }

  function fromNative(hit, spec) {
    if (!realBars(hit)) return null;
    var src = specOf(nativeId(spec) === "trades" ? "1" : nativeId(spec));
    var bars = hit.bars;
    if (spec.ms > src.ms) bars = aggregateBars(bars, spec.ms);
    if (!bars || bars.length < 2) return null;
    return { bars: bars, source: hit.source, at: hit.at };
  }

  function loadRhcBars(pool, tf, limit) {
    var spec = specOf(tf);
    var key = String(pool || "").toLowerCase() + "|" + spec.id;
    var hit = barCache[key];
    var freshMs = spec.fromTrades ? 8000 : (hit && hit.source === "print" ? 15000 : 90000);
    if (realBars(hit) && Date.now() - hit.at < freshMs) return Promise.resolve(hit);
    var nid = nativeId(spec);
    if (nid !== "trades" && nid !== spec.id) {
      var nhit = barCache[String(pool || "").toLowerCase() + "|" + nid];
      var derived = fromNative(nhit, spec);
      if (derived) {
        barCache[key] = derived;
        if (Date.now() - nhit.at >= freshMs) fetchBars(pool, specOf(nid), nid === "1" ? 1000 : 400, String(pool).toLowerCase() + "|" + nid);
        return Promise.resolve(derived);
      }
    }
    if (realBars(hit)) {
      fetchBars(pool, spec, limit, key);
      return Promise.resolve(hit);
    }
    return fetchBars(pool, spec, limit, key);
  }

  function prefetchTfs(pool) {
    if (!pool) return;
    ["1", "60", "D"].forEach(function (id, i) {
      setTimeout(function () {
        loadRhcBars(pool, id, id === "1" ? 1000 : 400);
      }, 280 * (i + 1));
    });
  }

  function fetchBars(pool, spec, limit, key) {
    if (barFetch[key]) return barFetch[key];
    var px = (selected && selected.pool === String(pool || "").toLowerCase() && selected.px) || lastClose || 0;
    var nid = nativeId(spec);
    var nspec = nid === "trades" ? spec : specOf(nid);
    var nkey = String(pool || "").toLowerCase() + "|" + (nid === "trades" ? spec.id : nid);

    function finish(res) {
      if (!res.bars || res.bars.length < 2) {
        if (!res.bars) res.bars = [];
        // No stubBars — empty stays empty (pool mid / prints only).
        if (!res.bars.length) res.source = "empty";
        else if (res.bars.length < 2) res.source = res.source || "prints";
      } else if (nid !== "trades" && spec.id !== nid && spec.ms > nspec.ms) {
        barCache[nkey] = { bars: res.bars, source: res.source, at: Date.now() };
        res = { bars: aggregateBars(res.bars, spec.ms), source: res.source };
      }
      if (!res.bars.length) res.source = "empty";
      barCache[key] = { bars: res.bars, source: res.source, at: Date.now() };
      delete barFetch[key];
      if (realBars(res)) persistBars();
      if (selected && selected.pool === pool && specOf(timeframe).id === spec.id && realBars(res)) {
        var had = lastBars && lastBars.length > 1 && lastTape !== "print" && lastTape !== "empty";
        lastBars = res.bars;
        lastTape = res.source;
        lastClose = res.bars[res.bars.length - 1].close;
        paintHud();
        if (!had) {
          try {
            if (chart && chart.setMarket && selected.source !== "binance") {
              chart.setMarket({
                symbol: marketSymbol(selected),
                timeframe: velaTf(spec),
                bars: spec.fromTrades ? 120 : 400,
                live: false,
                data: res.bars
              });
            }
          } catch (e) {}
        }
      }
      return res;
    }

    var job;
    if (spec.fromTrades) {
      var localTrades = String(pool).length === 42
        ? fetchJson("/liq/api/trades/" + encodeURIComponent(pool)).then(function (r) {
            return ((r.json && r.json.trades) || []).map(function (t) {
              return { time: n(t.time), px: n(t.px), vol: n(t.vol) };
            }).filter(function (t) { return t.time > 0 && t.px > 0; });
          }).catch(function () { return []; })
        : Promise.resolve([]);
      job = Promise.all([
        localTrades,
        geckoTrades(pool).catch(function () { return []; })
      ]).then(function (sets) {
        var trades = (sets[0] || []).concat(sets[1] || []);
        var bars = tradesToBars(trades, spec.ms);
        if (bars.length > 180) bars = bars.slice(-180);
        if (bars.length >= 2) return finish({ bars: bars, source: "prints" });
        return loadRhcBars(pool, "1", 1000).then(function (one) {
          if (realBars(one)) return finish({ bars: one.bars, source: one.source });
          return finish({ bars: [], source: "empty" });
        });
      });
    } else {
      function barsFrom(name) {
        if (name === "gecko") {
          return geckoBars(pool, nspec, nid === "1" ? 1000 : (n(limit) || 400)).then(function (b) {
            return b.length ? { bars: b, source: "gecko" } : null;
          });
        }
        return paprikaBars(pool, nspec, nid === "1" ? 1000 : (n(limit) || 400)).then(function (b) {
          return b.length ? { bars: scaleUsd(b, px), source: "paprika" } : null;
        });
      }
      var order = srcReady("gecko") ? ["gecko", "paprika"] : ["paprika", "gecko"];
      job = barsFrom(order[0]).then(function (got) {
        if (got) return finish(got);
        return barsFrom(order[1]).then(function (got2) {
          if (got2) return finish(got2);
          // Real on-chain prints only — never stubBars / Lithe flats.
          if (String(pool).length === 42) {
            return fetchJson("/liq/api/trades/" + encodeURIComponent(pool)).then(function (r) {
              var trades = ((r.json && r.json.trades) || []).map(function (t) {
                return { time: n(t.time), px: n(t.px), vol: n(t.vol) };
              }).filter(function (t) { return t.time > 0 && t.px > 0; });
              var bars = tradesToBars(trades, nspec.ms);
              if (bars.length > 400) bars = bars.slice(-400);
              if (bars.length >= 2) return finish({ bars: bars, source: "prints" });
              return finish({ bars: [], source: "empty" });
            }).catch(function () { return finish({ bars: [], source: "empty" }); });
          }
          return finish({ bars: [], source: "empty" });
        });
      });
    }
    barFetch[key] = job;
    job.catch(function () { delete barFetch[key]; });
    return job;
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
          lastBars = [];
          lastTape = "empty";
          paintHud();
          setLive(false, "rhc", true);
          return [];
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
    setLoad(false);
    bindChartEvents();
    bindHover();
    syncIndicators();
    applyChartPrefs();
    try { if (chart.drawings && chart.drawings.showToolbar) chart.drawings.showToolbar(true); } catch (e) {}
    if (rangePreset) {
      try { if (chart.setVisibleRangePreset) chart.setVisibleRangePreset(rangePreset); } catch (e) {}
    }
    paintHud();
    paintAlerts();
    try { if (chart.resize) chart.resize(); } catch (e) {}
    try { if (chart.renderer && chart.renderer.set) chart.renderer.set("autoScale", true); } catch (e) {}
    try { if (chart.renderer && chart.renderer.fitContent) chart.renderer.fitContent(); } catch (e) {}
    try { if (chart.setVisibleRangePreset) chart.setVisibleRangePreset("ALL"); } catch (e) {}
    requestAnimationFrame(function () {
      try { if (chart && chart.resize) chart.resize(); } catch (e) {}
      try { if (chart && chart.renderer && chart.renderer.set) chart.renderer.set("autoScale", true); } catch (e) {}
      try { if (chart && chart.renderer && chart.renderer.fitContent) chart.renderer.fitContent(); } catch (e) {}
    });
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
    // Honesty: real bars only. Empty/short must not become Vela data=[] (breaks setMarket for later pairs).
    if (data && data.length >= 2) market.data = data;

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
        chartBound = false;
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
        priceStyle: priceStyle || "candles",
        logScale: !!logScale,
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
        setLoad(false);
        setLive(false, "rhc", true);
      });
    } catch (e) {
      setLoad(false);
      setLive(false, "rhc", true);
    }
  }

  function bootChart() {
    var api = g.Vela;
    var plot = document.getElementById("plot");
    if (!api || !api.Vela || !plot) return;
    setLoad(true);
    paintPills();
    paintInds();
    paintStyles();
    paintScale();
    paintRange();
    var p = selected;
    if (!p) {
      setLoad(false);
      return;
    }
    if (p.source === "binance" && specOf(timeframe).fromTrades) {
      timeframe = "1";
      paintPills();
      flashPaper("Binance has no 30s tape — 1m.");
    }
    var spec = specOf(timeframe);
    var gen = ++bootGen;
    setTimeout(function () { if (gen === bootGen) setLoad(false); }, 8000);
    var isBinance = p.source === "binance";
    lastTape = isBinance ? "binance" : lastTape;
    if (isBinance) setLive(true, "binance", false);

    var hit = !isBinance ? cachedBars(p.pool, spec.id) : null;
    if (!realBars(hit) && !isBinance) {
      var nid = nativeId(spec);
      if (nid !== "trades" && nid !== spec.id) {
        hit = fromNative(cachedBars(p.pool, nid), spec);
      }
    }
    var cached = realBars(hit) ? hit.bars : null;
    if (cached) {
      lastBars = cached;
      lastTape = hit.source;
      lastClose = cached[cached.length - 1].close;
      paintHud();
      setLive(true, "rhc", false);
    }

    if (spec.fromTrades && !isBinance) {
      if (cached) mountChart(api, plot, p, spec, gen, ensureBars(cached, spec));
      loadRhcBars(p.pool, spec.id, 200).then(function (res) {
        if (gen !== bootGen) return;
        lastBars = res.bars;
        lastTape = res.source;
        if (lastBars.length) lastClose = lastBars[lastBars.length - 1].close;
        paintHud();
        setLive(res.source !== "empty", res.source === "prints" ? "print" : "rhc", res.source === "print" || res.source === "prints");
        if (!cached) mountChart(api, plot, p, spec, gen, ensureBars(lastBars, spec));
      }).catch(function () {
        if (gen !== bootGen || cached) return;
        mountChart(api, plot, p, spec, gen, ensureBars([], spec));
      });
      prefetchTfs(p.pool);
      return;
    }

    if (!isBinance) {
      loadRhcBars(p.pool, spec.id, 200);
      prefetchTfs(p.pool);
    }
    mountChart(api, plot, p, spec, gen, cached ? ensureBars(cached, spec) : undefined);
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
    var style = priceStyle || "candles";
    try { chart.renderer.set("priceStyle", style); } catch (e) {}
    try { chart.renderer.set("candleVisible", true); } catch (e) {}
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
    var usdAmt = notional();
    if (usdAmt > paper.cash) usdAmt = paper.cash;
    if (usdAmt < 1) { flashPaper("Not enough cash."); return; }
    if (paper.pos) {
      if (paper.pos.side !== side) {
        closeFill();
        if (paper.pos) return;
        if (usdAmt > paper.cash) usdAmt = paper.cash;
        if (usdAmt < 1) return;
      } else if (paper.pos.pool !== selected.pool) {
        flashPaper("Already " + paper.pos.side + " " + paper.pos.sym + ". Close it before opening " + selected.sym + ".");
        return;
      } else {
        paper.cash -= usdAmt;
        var addQty = usdAmt / px;
        var newQty = paper.pos.qty + addQty;
        var newCost = paper.pos.cost + usdAmt;
        paper.pos.qty = newQty;
        paper.pos.cost = newCost;
        paper.pos.entry = newQty > 0 ? newCost / newQty : px;
        var tpEl = document.getElementById("tp");
        var slEl = document.getElementById("sl");
        if (tpEl && n(tpEl.value) > 0) paper.pos.tp = n(tpEl.value);
        if (slEl && n(slEl.value) > 0) paper.pos.sl = n(slEl.value);
        if (!Array.isArray(paper.pos.adds)) paper.pos.adds = [];
        paper.pos.adds.push({ at: Date.now(), px: px, cost: usdAmt, qty: addQty });
        savePaper();
        paintPaper();
        paintRail();
        flashPaper("Added " + usd(usdAmt) + " " + selected.sym + " @ " + pxTxt(px) + " · avg " + pxTxt(paper.pos.entry));
        return;
      }
    }
    paper.cash -= usdAmt;
    paper.pos = {
      side: side,
      qty: usdAmt / px,
      entry: px,
      cost: usdAmt,
      sym: selected.sym,
      pool: selected.pool,
      opened: Date.now(),
      adds: [{ at: Date.now(), px: px, cost: usdAmt, qty: usdAmt / px }],
      tp: n(document.getElementById("tp") && document.getElementById("tp").value),
      sl: n(document.getElementById("sl") && document.getElementById("sl").value)
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
      id: Date.now() + "-" + Math.random().toString(16).slice(2),
      sym: pos.sym,
      side: pos.side,
      entry: pos.entry,
      exit: px,
      pnl: pnl,
      ret: ret,
      held: Date.now() - pos.opened,
      at: Date.now()
    };
    if (!Array.isArray(paper.book)) paper.book = [];
    paper.book.push(slip);
    paper.slips = paper.book.slice(-24);
    savePaper();
    paintPaper();
    flashPaper("Closed " + pos.sym + "  " + signedUsd(pnl) + " · book " + signedUsd(bookStats().realized));
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
    var bookEl = document.getElementById("slip-book");
    if (bookEl) {
      bookEl.textContent = signedUsd(bookStats().realized);
      bookEl.className = bookStats().realized >= 0 ? "up" : "dn";
    }
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
    x.fillText("Not a fill. Local paper only.  Book " + signedUsd(bookStats().realized), 72, 620);
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

  function tokenKey(t) {
    return String(t || "").toLowerCase();
  }

  function listingOf(token) {
    var m = ketSnap && ketSnap.by_token;
    if (!m || !token) return null;
    return m[tokenKey(token)] || null;
  }

  function stampListed() {
    var map = (ketSnap && ketSnap.live && ketSnap.by_token) || {};
    pairs.forEach(function (p) {
      var row = map[tokenKey(p.token)];
      if (row) {
        p.listed = true;
        p.listedPaid = n(row.paid);
        p.listedRank = row.rank;
        applyListingProfile(p, row);
      } else {
        p.listed = false;
        p.listedPaid = 0;
        p.listedRank = 0;
      }
    });
    if (tab !== "listed") return;
    var list = (ketSnap && ketSnap.listings) || [];
    list.forEach(function (r) {
      var tok = tokenKey(r.token);
      if (!tok) return;
      var found = false;
      var i;
      for (i = 0; i < pairs.length; i++) {
        if (tokenKey(pairs[i].token) === tok) { found = true; break; }
      }
      if (found) return;
      var stub = {
        pool: tokenKey(r.pool || r.token),
        token: tok,
        sym: r.symbol || "???",
        name: r.name || r.symbol || "",
        listed: true,
        listedPaid: n(r.paid),
        listedRank: r.rank,
        kind: "meme",
        source: "listed",
        px: 0, chg: 0, vol: 0, liq: 0
      };
      applyListingProfile(stub, r);
      putPair(stub);
    });
  }

  function applyListingProfile(p, row) {
    if (!p || !row) return;
    p.listedWeb = row.website || "";
    p.listedX = row.twitter || "";
    p.listedTg = row.telegram || "";
    p.listedDc = row.discord || "";
    p.listedLogo = row.logo || "";
    if (row.bio) p.blurb = row.bio;
    if (row.logo) p.icon = row.logo;
    if (row.name) p.name = row.name;
    if (row.symbol) p.sym = p.sym || row.symbol;
    var links = Array.isArray(p.links) ? p.links.slice() : [];
    function addLink(lab, url) {
      if (!url) return;
      var href = url;
      if (lab === "X" && url.charAt(0) !== "h") href = "https://x.com/" + url.replace(/^@/, "");
      if (lab === "Telegram" && url.charAt(0) !== "h") href = "https://t.me/" + url.replace(/^@/, "");
      var i;
      for (i = 0; i < links.length; i++) if (links[i].url === href) return;
      links.push({ label: lab, url: href });
    }
    addLink("Web", row.website);
    addLink("X", row.twitter);
    addLink("Telegram", row.telegram);
    addLink("Discord", row.discord);
    if (links.length) p.links = links;
  }

  function packListMeta() {
    var o = {};
    var w = (document.getElementById("list-web").value || "").trim();
    var x = (document.getElementById("list-tw").value || "").trim().replace(/^@/, "");
    var t = (document.getElementById("list-tg").value || "").trim();
    var d = (document.getElementById("list-dc").value || "").trim();
    var b = (document.getElementById("list-bio").value || "").trim().slice(0, 160);
    var l = (document.getElementById("list-logo").value || "").trim();
    if (w) o.w = w;
    if (x) o.x = x;
    if (t) o.t = t;
    if (d) o.d = d;
    if (b) o.b = b;
    if (l) o.l = l;
    return Object.keys(o).length ? JSON.stringify(o) : "";
  }

  function fillListProfile(row) {
    if (!row) return;
    if (row.website) document.getElementById("list-web").value = row.website;
    if (row.twitter) document.getElementById("list-tw").value = row.twitter;
    if (row.telegram) document.getElementById("list-tg").value = row.telegram;
    if (row.discord) document.getElementById("list-dc").value = row.discord;
    if (row.logo) document.getElementById("list-logo").value = row.logo;
    if (row.bio) document.getElementById("list-bio").value = row.bio;
  }

  function wholePusd(x) {
    return Math.max(0, Math.floor(n(x)));
  }

  function netLabel() {
    if (ketSnap && (ketSnap.net === "mainnet" || ketSnap.chain_id === 4663)) {
      return "Robinhood Chain " + (ketSnap.chain_id || 4663);
    }
    return "Robinhood Chain Testnet 46630";
  }

  function openListSheet(p) {
    var sheet = document.getElementById("list-sheet");
    if (!sheet) return;
    sheet.hidden = false;
    sheet.classList.add("open");
    document.getElementById("list-err").textContent = "";
    if (p && p.token) {
      document.getElementById("list-ca").value = p.token;
      document.getElementById("list-pool").value = p.pool && p.source !== "binance" ? p.pool : "";
      document.getElementById("list-sym").value = p.sym || "";
      document.getElementById("list-name").value = p.name || p.sym || "";
      fillListProfile(listingOf(p.token) || {
        website: p.listedWeb, twitter: p.listedX, telegram: p.listedTg,
        discord: p.listedDc, logo: p.listedLogo || p.icon, bio: p.blurb
      });
    }
    paintListSheet();
    quoteList();
    if (g.vapurr && g.vapurr.send) g.vapurr.send({ cmd: "ketlist" });
  }

  function closeListSheet() {
    if (g.vapurr && g.vapurr.pendingTx) return;
    var sheet = document.getElementById("list-sheet");
    if (sheet) {
      sheet.classList.remove("open");
      sheet.hidden = true;
    }
  }

  function paintListSheet() {
    var s = ketSnap || {};
    document.getElementById("list-top").textContent = String(wholePusd(s.top));
    document.getElementById("list-pot").textContent = String(wholePusd(s.pot));
    document.getElementById("list-bal").textContent = String(wholePusd(s.pusd));
    var dep = document.getElementById("list-deploy");
    var go = document.getElementById("list-go");
    if (s.live) {
      dep.style.display = "none";
      go.style.display = "";
    } else {
      dep.style.display = "";
      dep.textContent = s.need_market ? "Mint $PUSD" : "Open the list";
      go.style.display = s.need_deploy ? "none" : "";
    }
    if (!listAmtDirty) {
      document.getElementById("list-amt").value = String(wholePusd(s.quote_first || 50) || 50);
    }
    quoteList();
  }

  function quoteList() {
    var amtEl = document.getElementById("list-amt");
    var q = document.getElementById("list-quote");
    var btn = document.getElementById("list-go");
    if (!amtEl || !q || !btn) return;
    var amt = wholePusd(amtEl.value);
    var token = (document.getElementById("list-ca") && document.getElementById("list-ca").value || "").trim();
    var pool = (document.getElementById("list-pool") && document.getElementById("list-pool").value || "").trim();
    q.className = "hint";
    q.onclick = null;
    btn.disabled = true;
    if (!ketSnap || !ketSnap.live) {
      q.textContent = ketSnap && ketSnap.need_market
        ? "Mint $PUSD first."
        : "Open the list, then pay $PUSD.";
      btn.textContent = "List";
      return;
    }
    if (!token || !pool) {
      q.textContent = wholePusd(ketSnap.quote_first || 50) + " $PUSD lists a token.";
      btn.textContent = "List";
      return;
    }
    var row = listingOf(token);
    var top = wholePusd(ketSnap.top);
    var minePaid = row && row.mine ? wholePusd(row.paid) : 0;
    if (row && !row.mine) {
      q.textContent = "That's listed.";
      btn.textContent = "List";
      return;
    }
    if (amt < 1) {
      q.textContent = "Whole $PUSD only.";
      btn.textContent = minePaid ? "Raise" : "List";
      return;
    }
    if (!minePaid && amt < 50) {
      q.textContent = "First listing is 50 $PUSD.";
      btn.textContent = "List";
      return;
    }
    if (minePaid && amt <= minePaid) {
      q.textContent = "You sit at " + minePaid + ". Raise at least 10.";
      btn.textContent = "Raise";
      return;
    }
    if (minePaid && amt < minePaid + 10) {
      q.textContent = "Raise at least 10 $PUSD.";
      btn.textContent = "Raise";
      return;
    }
    if (amt > top && amt < top + 25) {
      q.textContent = "#1 is " + top + ". Taking it costs " + (top + 25) + ".";
      btn.textContent = minePaid ? "Raise" : "List";
      return;
    }
    var charge = minePaid ? amt - minePaid : amt;
    var bal = wholePusd(ketSnap.pusd);
    if (charge > bal) {
      q.textContent = "Mint " + charge + " $PUSD.";
      q.className = "hint go";
      q.onclick = function () { g.vapurr.go("vapurr://pusd"); };
      btn.textContent = minePaid ? "Raise" : "List";
      return;
    }
    q.textContent = (minePaid ? "Raise " : "Pay ") + charge + " $PUSD" + (row && row.rank ? " · #" + row.rank : "") + ".";
    btn.textContent = listBusy ? "…" : (minePaid ? "Raise" : "List");
    btn.disabled = listBusy;
  }

  function submitList() {
    if (listBusy || (g.vapurr && g.vapurr.pendingTx)) return;
    var btn = document.getElementById("list-go");
    if (btn.disabled) return;
    document.getElementById("list-err").textContent = "";
    var token = document.getElementById("list-ca").value;
    var pool = document.getElementById("list-pool").value;
    var symbol = document.getElementById("list-sym").value;
    var name = document.getElementById("list-name").value;
    var amt = String(wholePusd(document.getElementById("list-amt").value));
    var meta = packListMeta();
    var web = (document.getElementById("list-web").value || "").trim();
    g.vapurr.beginTx({
      title: "List on Ketcharts",
      kicker: "This device signs",
      lede: "Pay $PUSD on this device. Profile rides with the tx. Hash or it failed.",
      rows: [
        { k: "Network", v: netLabel() },
        { k: "From", v: (ketSnap && ketSnap.address) || "this device" },
        { k: "Token", v: token },
        { k: "Pool", v: pool },
        { k: "Ticker", v: symbol },
        { k: "Website", v: web || "—" },
        { k: "Amount", v: amt + " PUSD" }
      ],
      confirmLabel: "Sign and list",
      doneTitle: "Listed",
      failTitle: "Not listed",
      explorer: ketSnap && ketSnap.explorer
    }).then(function (ok) {
      if (!ok) return;
      listBusy = true;
      listWaitHash = (ketSnap && ketSnap.tx) || "";
      quoteList();
      g.vapurr.send({
        cmd: "ketlist-pay",
        token: token,
        pool: pool,
        symbol: symbol,
        name: name,
        amt: amt,
        meta: meta
      });
    });
  }

  function deployList() {
    if (g.vapurr && g.vapurr.pendingTx) return;
    document.getElementById("list-err").textContent = "";
    if (ketSnap && ketSnap.need_market) {
      g.vapurr.go("vapurr://pusd");
      return;
    }
    g.vapurr.beginTx({
      title: "Open the list",
      kicker: "This device signs",
      kind: "deploy",
      lede: "Deploy the Ketcharts list on this device. Hash or it failed.",
      rows: [
        { k: "Network", v: netLabel() },
        { k: "From", v: (ketSnap && ketSnap.address) || "this device" }
      ],
      confirmLabel: "Sign and open",
      doneTitle: "Open",
      failTitle: "Not opened",
      explorer: ketSnap && ketSnap.explorer
    }).then(function (ok) {
      if (!ok) return;
      listBusy = true;
      listWaitHash = (ketSnap && ketSnap.tx) || "";
      g.vapurr.send({ cmd: "ketlist-deploy" });
    });
  }

  g.__setKetList = function (s) {
    var was = listBusy;
    ketSnap = s;
    listBusy = false;
    try { stampListed(); } catch (e) {}
    var sheet = document.getElementById("list-sheet");
    if (sheet && sheet.classList.contains("open")) {
      try { paintListSheet(); } catch (e2) {}
    }
    if (tab === "listed") {
      try { paintList(); paintTrench(); } catch (e3) {}
    }
    if (!g.vapurr || !g.vapurr.pendingTx) return;
    if (s.tx && s.tx !== listWaitHash) {
      g.vapurr.finishTx(true, { tx: s.tx, txUrl: s.tx_url });
      return;
    }
    if (was && g.vapurr.pendingTx.kind === "deploy" && s.live) {
      g.vapurr.finishTx(true, { already: true, title: "Already live" });
    }
  };
  g.__econErr = function (which, msg) {
    if (which !== "ketlist" && which !== "deploy") return;
    listBusy = false;
    if (g.vapurr && g.vapurr.pendingTx) g.vapurr.finishTx(false, { error: msg || "failed" });
    var el = document.getElementById("list-err");
    if (el) el.textContent = msg || "";
    var sheet = document.getElementById("list-sheet");
    if (sheet && sheet.classList.contains("open")) {
      try { quoteList(); } catch (e) {}
    }
  };

  function bind() {
    document.querySelectorAll("[data-tab]").forEach(function (el) {
      el.classList.toggle("on", el.getAttribute("data-tab") === tab);
      el.onclick = function () {
        tab = el.getAttribute("data-tab");
        document.querySelectorAll("[data-tab]").forEach(function (x) { x.classList.toggle("on", x === el); });
        if (tab === "new") { sortKey = "created"; sortDir = -1; }
        if (tab === "hot") { sortKey = "vol"; sortDir = -1; }
        if (tab === "listed") { sortKey = "listedPaid"; sortDir = -1; }
        if (tab === "stock") { sortKey = "vol"; sortDir = -1; }
        if (tab === "majors") {
          lastBars = [];
          lastClose = 0;
          hoverBar = null;
          selected = {
            pool: MAJORS[0].id, token: MAJORS[0].id, sym: MAJORS[0].lab, name: MAJORS[0].lab,
            px: n(MAJORS[0].px), chg: n(MAJORS[0].chg), vol: n(MAJORS[0].vol), liq: 0,
            kind: "cash", source: "binance"
          };
          hydrateMajors();
        } else {
          ensureSelected();
        }
        savePref();
        paintList();
        paintTrench();
        paintHud();
        paintAlerts();
        paintRail();
        bootChart();
      };
    });
    function on(id, fn) {
      var el = document.getElementById(id);
      if (el) el.onclick = fn;
    }
    on("list-token", function () { openListSheet(selected); });
    on("list-go", submitList);
    on("list-deploy", deployList);
    on("list-cancel", closeListSheet);
    on("list-x", closeListSheet);
    on("list-dim", closeListSheet);
    document.querySelectorAll("[data-rail]").forEach(function (el) {
      el.onclick = function () { setRail(el.getAttribute("data-rail")); };
    });
    ["list-ca", "list-pool", "list-sym", "list-name", "list-web", "list-tw", "list-tg", "list-dc", "list-logo", "list-bio"].forEach(function (id) {
      var el = document.getElementById(id);
      if (el) el.addEventListener("input", quoteList);
    });
    var amtEl = document.getElementById("list-amt");
    if (amtEl) amtEl.addEventListener("input", function () {
      listAmtDirty = true;
      quoteList();
    });
    document.getElementById("q").addEventListener("input", function () {
      q = document.getElementById("q").value;
      paintList();
      if (searchTimer) clearTimeout(searchTimer);
      var qq = q.trim();
      if (qq.length < 2 || tab === "majors") return;
      searchTimer = setTimeout(function () {
        searchScreeners(qq).then(function (found) {
          (found || []).forEach(putDexPair);
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
    document.getElementById("share-pair").onclick = function () { drawPairCard(); };
    document.getElementById("alert-up").onclick = function () { addAlert("above"); };
    document.getElementById("alert-dn").onclick = function () { addAlert("below"); };
    document.getElementById("reset").onclick = function () {
      if (paper.pos) closeFill();
      var equity = n(paper.cash);
      var injected = Math.max(0, 10000 - equity);
      if (!Array.isArray(paper.resets)) paper.resets = [];
      paper.resets.push({ at: Date.now(), cashBefore: equity, injected: injected });
      paper.cash = 10000;
      paper.pos = null;
      savePaper();
      paintPaper();
      var st = bookStats();
      flashPaper("Cash refilled. Lifetime still " + signedUsd(st.realized) + (st.gl ? (" · gross loss " + usd(st.gl)) : ""));
    };
    var copyca = document.getElementById("copyca");
    if (copyca) {
      copyca.onclick = function () {
        if (!selected || !selected.pool || selected.source === "binance") {
          flashPaper("No pool on this cut.");
          return;
        }
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(selected.pool);
          flashPaper("Copied " + selected.pool.slice(0, 10) + "…");
        }
      };
    }
    document.addEventListener("keydown", function (e) {
      var tag = (e.target && e.target.tagName) || "";
      if (tag === "INPUT" || tag === "TEXTAREA") {
        if (e.key === "Escape") e.target.blur();
        return;
      }
      var k = e.key;
      if (k === "/" ) { e.preventDefault(); document.getElementById("q").focus(); return; }
      if (k === "Escape") {
        document.getElementById("slip").hidden = true;
        closeListSheet();
        return;
      }
      var tfMap = { "1": "1", "2": "5", "3": "15", "4": "60", "5": "240", "6": "D", "7": "W", "8": "M" };
      if (tfMap[k]) { timeframe = tfMap[k]; savePref(); bootChart(); return; }
      if (k === "c" || k === "C") { priceStyle = "candles"; savePref(); paintStyles(); applyChartPrefs(); return; }
      if (k === "b" || k === "B") { priceStyle = "bars"; savePref(); paintStyles(); applyChartPrefs(); return; }
      if (k === "n" || k === "N") { priceStyle = "line"; savePref(); paintStyles(); applyChartPrefs(); return; }
      if (k === "g" || k === "G") { logScale = !logScale; savePref(); paintScale(); applyChartPrefs(); return; }
      if (k === "j" || k === "J") { stepPair(1); return; }
      if (k === "k" || k === "K") { stepPair(-1); return; }
      if (k === "l" || k === "L") { fill("long"); return; }
      if (k === "s" || k === "S") { fill("short"); return; }
      if (k === "x" || k === "X") { closeFill(); return; }
    });
    window.addEventListener("resize", function () {
      try { if (chart && chart.resize) chart.resize(); } catch (e) {}
    });
    var wrap = document.querySelector(".plot-wrap");
    if (wrap && typeof ResizeObserver !== "undefined") {
      new ResizeObserver(function () {
        try { if (chart && chart.resize) chart.resize(); } catch (e) {}
      }).observe(wrap);
    }
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
    paintRail();
    paintHud();
    hydrateMajors();
    try { hydrateFromCache(); } catch (e) {}
    var hadChart = !!selected;
    if (hadChart) bootChart();
    loadBoard().then(function () {
      if (!hadChart && selected) bootChart();
    }).catch(function () {
      paintList();
      if (selected) bootChart();
    });
    if (g.vapurr && g.vapurr.send) g.vapurr.send({ cmd: "ketlist" });
    setInterval(tickBoard, 12000);
    setInterval(function () {
      if (selected && selected.source !== "binance") paintTxns();
    }, 14000);
    setInterval(paintAges, 1000);
  }

  function drawPairCard() {
    var p = selected;
    if (!p || p.source === "binance") { flashPaper("Pick an RHC pair."); return; }
    var c = document.createElement("canvas");
    c.width = 1200;
    c.height = 675;
    var x = c.getContext("2d");
    var lg = x.createLinearGradient(0, 0, 1200, 675);
    lg.addColorStop(0, "#0e0e0e");
    lg.addColorStop(0.5, "#12140c");
    lg.addColorStop(1, "#0a0a0a");
    x.fillStyle = lg;
    x.fillRect(0, 0, 1200, 675);
    var rg = x.createRadialGradient(80, -40, 20, 80, -40, 520);
    rg.addColorStop(0, "rgba(192,248,0,0.18)");
    rg.addColorStop(1, "rgba(192,248,0,0)");
    x.fillStyle = rg;
    x.fillRect(0, 0, 1200, 675);
    x.fillStyle = RED;
    x.font = "900 56px Arial Black, Impact, sans-serif";
    x.fillText("KETCHARTS", 72, 100);
    x.fillStyle = "#8aa090";
    x.font = "600 20px Sora, sans-serif";
    x.fillText("ROBINHOOD CHAIN  ·  " + ageTxt(p.created), 72, 140);
    x.fillStyle = "#f2f3f4";
    x.font = "600 64px Sora, sans-serif";
    x.fillText(p.sym, 72, 240);
    var up = n(p.chg) >= 0;
    x.fillStyle = up ? LIME : RED;
    x.font = "600 96px Sora, sans-serif";
    x.fillText(p.source === "binance" ? usd(markPrice()) : pct(p.chg), 72, 360);
    x.fillStyle = "#8aa090";
    x.font = "400 22px Sora, sans-serif";
    x.fillText("px " + usd(p.px) + "    liq " + usd(p.liq) + "    vol " + usd(p.vol), 72, 430);
    x.fillText("mcap " + usd(p.mcap) + (p.holders != null ? ("    holders " + p.holders) : ""), 72, 470);
    x.fillStyle = "#8aa090";
    x.font = "400 18px Sora, sans-serif";
    x.fillText(p.pool, 72, 620);
    x.fillStyle = LIME;
    x.font = "600 18px Sora, sans-serif";
    x.fillText("vapurr", 1040, 620);
    c.toBlob(function (blob) {
      if (!blob) return;
      var a = document.createElement("a");
      a.href = URL.createObjectURL(blob);
      a.download = p.sym.toLowerCase() + "-rhc.png";
      a.click();
      if (navigator.clipboard && navigator.clipboard.write) {
        try { navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]); } catch (e) {}
      }
    }, "image/png");
  }

  g.bootKetcharts = bind;
})(window);
