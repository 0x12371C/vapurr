(function (g) {
  var SLOTS = [
    { id: "s1", live: false },
    { id: "s2", live: false },
    { id: "s3", live: false },
    { id: "s4", live: false },
    { id: "s5", live: false },
    { id: "s6", live: false }
  ];

  var HOUSE = [
    {
      id: "veil",
      href: "https://veil.markets",
      cls: "floor-card house veil",
      wall: "/veil.jpg",
      mark: "/veil-mark.svg",
      word: "VEIL",
      sub: "veil.markets"
    },
    {
      id: "tsl",
      href: "https://thesecretlab.app",
      cls: "floor-card house tsl",
      wall: "/tsl.png",
      mark: "/tsl-mark.svg",
      word: "thesecretlab",
      sub: "thesecretlab.app"
    },
    {
      id: "fomo",
      href: "https://fomo.family",
      cls: "floor-card house fomo",
      wall: "/fomo.jpg",
      mark: "/fomo-mark.svg",
      word: "fomo",
      sub: "fomo.family"
    },
    {
      id: "pons",
      href: "https://www.ponsfamily.com",
      cls: "floor-card house pons",
      wall: "/pons.jpg",
      mark: "/pons-mark.svg",
      word: "pons",
      sub: "ponsfamily.com"
    }
  ];

  function go(href) {
    if (href && g.vapurr && g.vapurr.go) g.vapurr.go(href);
  }

  function goSell() {
    go("vapurr://vapurrbid");
    if (!(g.vapurr && g.vapurr.go) && g.vapurr && g.vapurr.send) {
      g.vapurr.send({ cmd: "pane", id: "vapurrbid" });
    }
  }

  function card(listing) {
    var b = document.createElement("button");
    b.type = "button";
    if (listing) {
      b.className = "floor-card";
      var rank = listing.rank ? "#" + listing.rank : "LIVE";
      var title = listing.title || listing.url || "Listing";
      var paid = listing.paid ? String(Math.floor(parseFloat(listing.paid) || 0)) : "0";
      b.innerHTML =
        '<span class="mono">' + rank + "</span><b></b><span class='blurb paid'></span>";
      b.querySelector("b").textContent = title;
      b.querySelector(".blurb").textContent = paid + " $PUSD";
      b.onclick = function () {
        go(listing.href || listing.url);
      };
      return b;
    }
    b.className = "floor-card open";
    b.innerHTML =
      '<span class="mono">OPEN</span><b>Your project</b><span class="blurb">vapurrbid · $PUSD</span>';
    b.onclick = goSell;
    return b;
  }

  function ketflixCard() {
    var b = document.createElement("button");
    b.type = "button";
    b.className = "floor-card ketflix";
    b.innerHTML =
      '<img class="kfx-wall" src="/ketflix.png" alt="" decoding="async" fetchpriority="high"/>' +
      '<img class="kfx-logo" src="/ketflix-logo.svg" alt="Ketflix" decoding="async"/>' +
      '<span class="kfx-sub">infinite slop</span>' +
      '<span class="kfx-by">powered by <em>$VAPURR</em></span>';
    b.onclick = function () {
      go("vapurr://ketflix");
    };
    return b;
  }

  function houseCard(h) {
    var b = document.createElement("button");
    b.type = "button";
    b.className = h.cls;
    b.innerHTML =
      '<img class="house-wall" alt=""/>' +
      '<span class="house-id"><img class="house-mark" alt=""/><b class="house-word"></b></span>' +
      '<span class="house-sub"></span>';
    var wall = b.querySelector(".house-wall");
    wall.loading = "lazy";
    wall.decoding = "async";
    wall.src = h.wall;
    var mark = b.querySelector(".house-mark");
    mark.decoding = "async";
    mark.src = h.mark;
    b.querySelector(".house-word").textContent = h.word;
    b.querySelector(".house-sub").textContent = h.sub;
    b.onclick = function () {
      go(h.href);
    };
    return b;
  }

  function paintHouse(grid) {
    if (!grid) return;
    grid.innerHTML = "";
    HOUSE.forEach(function (h) {
      grid.appendChild(houseCard(h));
    });
  }

  g.VAPURR_FLOOR = SLOTS;
  g.VAPURR_HOUSE = HOUSE;
  g.paintHouse = paintHouse;
  g.paintFloor = function (grid, listings) {
    if (!grid) return;
    grid.innerHTML = "";
    var bids = listings || [];
    var reserved = 1 + HOUSE.length;
    SLOTS.forEach(function (s, i) {
      if (i === 0) {
        grid.appendChild(ketflixCard());
        return;
      }
      if (i < reserved) {
        grid.appendChild(houseCard(HOUSE[i - 1]));
        return;
      }
      grid.appendChild(card(bids[i - reserved] || null));
    });
  };
  g.openFloorSell = goSell;
})(window);
