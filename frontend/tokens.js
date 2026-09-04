(function (g) {
  var MAP = {
    vapurr: "/mascot.png",
    v: "/mascot.png",
    eth: "/tokens/eth.svg",
    weth: "/tokens/weth.svg",
    usdg: "/tokens/usdg.png",
    pusd: "/tokens/pusd.svg"
  };

  g.tokenIconUrl = function (id, symbol) {
    var k = String(id || symbol || "").toLowerCase().replace(/^\$/, "");
    if (MAP[k]) return MAP[k];
    if (k === "purr usd") return MAP.pusd;
    return "";
  };

  g.tokenIconEl = function (id, symbol, extraClass) {
    var url = g.tokenIconUrl(id, symbol);
    var wrap = document.createElement("span");
    var cls = "dot ico";
    if (id) cls += " " + id;
    if (extraClass) cls += " " + extraClass;
    wrap.className = cls;
    if (url) {
      var img = document.createElement("img");
      img.src = url;
      img.alt = "";
      wrap.appendChild(img);
    } else {
      wrap.textContent = String(symbol || "?").slice(0, 2);
    }
    return wrap;
  };
})(window);
