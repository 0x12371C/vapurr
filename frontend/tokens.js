(function (g) {
  var MAP = {
    vapurr: "/mascot.png",
    v: "/mascot.png",
    gv: "/mascot.png",
    wgv: "/mascot.png",
    gvapurr: "/mascot.png",
    wgvapurr: "/mascot.png",
    amzn: "/tokens/amzn.svg",
    tsla: "/tokens/tsla.svg",
    amd: "/tokens/amd.svg",
    nflx: "/tokens/nflx.svg",
    pltr: "/tokens/pltr.svg",
    nvda: "/tokens/nvda.svg",
    eeth: "/tokens/weth.svg",
    envda: "/tokens/nvda.svg",
    eamd: "/tokens/amd.svg",
    eamzn: "/tokens/amzn.svg",
    etsla: "/tokens/tsla.svg",
    enflx: "/tokens/nflx.svg",
    epltr: "/tokens/pltr.svg",
    spusd: "/tokens/pusd.svg",
    musdg: "/tokens/usdg.png",
    eth: "/tokens/eth.svg",
    weth: "/tokens/weth.svg",
    usdg: "/tokens/usdg.png",
    pusd: "/tokens/pusd.svg"
  };

  g.tokenIconUrl = function (id, symbol) {
    var k = String(id || symbol || "").toLowerCase().replace(/^\$/, "");
    if (MAP[k]) return MAP[k];
    if (k === "pusd-loop" || k === "pusdloop") return MAP.pusd;
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
      if (/^e?(amzn|tsla|amd|nflx|pltr|nvda)$/i.test(String(id || symbol))) {
        img.style.background = "#fff";
        img.style.padding = "4px";
        img.style.borderRadius = "50%";
      }
      wrap.appendChild(img);
    } else {
      wrap.textContent = String(symbol || "?").slice(0, 2);
    }
    return wrap;
  };
})(window);
