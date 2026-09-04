/* Maestro Play mini radio. Catalog from thesecretlab.app. Audio via /api/maestro/audio (mp4). */
(function (g) {
  var ORIGIN = "https://thesecretlab.app";
  var CATALOG = [{"title":"Subsurface","artist":"NØVUS","suno":"97351120-af70-4dd1-abc0-a0f12b339049","image":"/artists/NOVUS.png","color":"#10B981"},{"title":"South of the River","artist":"KIRA","suno":"2d7a3b52-b8f1-4877-813e-b5c24a4dfa02","image":"/artists/KIRA.png","color":"#A855F7"},{"title":"Fragmentation Event","artist":"HEX_NULL","suno":"5da9ee1d-3690-46c9-a6ba-a168326c616b","image":"/artists/HEX_NULL.png","color":"#EF4444"},{"title":"3am homework","artist":"j u n o","suno":"5b3a3e4e-a9dc-48e8-bfb4-9626ca20d2cb","image":"/artists/j_u_n_o.png","color":"#F59E0B"},{"title":"Acid Cathedral","artist":"VORTEKS","suno":"ebd945f2-8037-4da5-a646-292d0ef3450d","image":"/artists/VORTEKS.png","color":"#22C55E"},{"title":"Midnight Protocol","artist":"CHROME DEITY","suno":"2a8c657a-ab39-4ae4-9c66-d511777cbac9","image":"/artists/CHROME_DEITY.png","color":"#EC4899"},{"title":"Golden Hour","artist":"SOLACE","suno":"132380bb-9883-4753-9c4e-18bb7de2f6ab","image":"/artists/SOLACE.png","color":"#F97316"},{"title":"Danse Avec Moi","artist":"MIROIR","suno":"b4b81057-915b-40c7-a333-6fce9d5f6495","image":"/artists/MIROIR.png","color":"#EAB308"},{"title":"Tidal Memory","artist":"FATHOM","suno":"721b2795-b2be-4ad0-9eb3-58ee67b2374c","image":"/artists/FATHOM.png","color":"#06B6D4"},{"title":"Sugar Crash","artist":"GLITCHPRINCESS","suno":"10bfa423-570c-4618-92da-10dbf07ebdad","image":"/artists/GLITCHPRINCESS.png","color":"#F472B6"},{"title":"Machine Soul","artist":"AXIS","suno":"2c640900-0474-4dcc-956f-e11e6ebc5d07","image":"/artists/AXIS.png","color":"#3B82F6"},{"title":"Pirate Signal","artist":"RUFFCUT","suno":"87303069-14d4-4742-8031-38e4aab24ed0","image":"/artists/RUFFCUT.png","color":"#84CC16"},{"title":"Smoke & Mirrors","artist":"SHADE","suno":"f8fa71ed-83ba-48bf-aa0f-7896ba2ddbdf","image":"/artists/SHADE.png","color":"#78716C"},{"title":"First Light","artist":"ORVANA","suno":"21783ea3-5fb6-4bee-bcaf-19b2cf7d7151","image":"/artists/ORVANA.png","color":"#D946EF"},{"title":"Technicolor","artist":"PRISM","suno":"2862ed4d-503f-412a-a8e0-e7a3c399257f","image":"/artists/PRISM.png","color":"#14B8A6"},{"title":"Fading Polaroid","artist":"POOLSIDE","suno":"f60bffad-ca62-415d-84a1-bf202406cb00","image":"/artists/POOLSIDE.png","color":"#FB923C"},{"title":"Eternal Anthem","artist":"THUNDERDOME","suno":"a93d24e3-ea69-411d-9270-3ac79b1db8e8","image":"/artists/THUNDERDOME.png","color":"#DC2626"},{"title":"Hollow Frequency","artist":"BURIAL GROUND","suno":"36a494aa-42d6-4824-8bb6-72378dd56960","image":"/artists/BURIAL_GROUND.png","color":"#7C3AED"},{"title":"Roaring Circuits","artist":"THE GATSBY MACHINE","suno":"dd965bc3-a973-45b5-938c-2671bdeccce9","image":"/artists/GATSBY_MACHINE.png","color":"#D97706"},{"title":"Spore Network","artist":"MYCELIUM","suno":"6777dcbd-67d5-4ff6-beaa-f6c98f8560b1","image":"/artists/MYCELIUM.png","color":"#16A34A"},{"title":"永遠のモール Eternal Mall","artist":"マクロ MACRO","suno":"4f56ae7c-bff4-4ddd-88bc-5425799931b1","image":"/artists/MACRO.png","color":"#E879F9"},{"title":"Redline","artist":"SXLVR","suno":"971ba520-8285-4d5d-a3f8-c47f8dfac0fe","image":"/artists/SXLVR.png","color":"#B91C1C"},{"title":"Blackout Drift","artist":"KORVØ","suno":"c2cccee2-cbff-4352-91c2-84fe77c19117","image":"/artists/KORVO.png","color":"#991B1B"},{"title":"Body Mechanic","artist":"SOLARDO","suno":"03890068-b81e-42df-b9e4-836dbacacf39","image":"/artists/SOLARDO.png","color":"#0EA5E9"},{"title":"Neural Override","artist":"SYNAPSE","suno":"64bbee53-eeda-4932-8a80-c48c3bb58453","image":"/artists/SYNAPSE.png","color":"#2563EB"},{"title":"No Brakes","artist":"GRIMM × SXLVR","suno":"b375b8c2-1d1a-4cbe-a9ae-f05b6227cba7","image":"/artists/GRIMM_x_SXLVR.png","color":"#9F1239"},{"title":"Purple Boulevard","artist":"DJ SWANG","suno":"3982ef90-17b7-4bcf-87a9-517383a03758","image":"/artists/DJ_SWANG.png","color":"#7E22CE"},{"title":"Magnolia Smoke","artist":"ROUX","suno":"ee9a1755-74c6-4ba8-a5e2-be5e99c85210","image":"/artists/ROUX.png","color":"#CA8A04"},{"title":"Black Sacrament","artist":"††† CVLT","suno":"6dd1f5f1-fd9e-4beb-a68b-ae6b0281d559","image":"/artists/CVLT.png","color":"#525252"},{"title":"The Space Between Signals","artist":"ΔETHER","suno":"3f96ec85-ca8d-4b7d-b61d-bb243078af25","image":"/artists/AETHER.png","color":"#6366F1"},{"title":"Between Floors","artist":"LIMINAL_SPACE","suno":"0cfacd56-3e77-437b-b195-87fd1efa3c4b","image":"/artists/LIMINAL_SPACE.png","color":"#94A3B8"},{"title":"Sakura Protocol","artist":"YŪGEN","suno":"a65920b4-6d21-4b7c-96f8-e92d5c42905f","image":"/artists/YUGEN.png","color":"#F9A8D4"},{"title":"Fieldwork","artist":"MOTH_LIGHT","suno":"80e8b62e-3c7b-4526-bc4b-d10b6a35de30","image":"/artists/MOTH_LIGHT.png","color":"#A3E635"},{"title":"Fountain Court","artist":"DEAD_MALL","suno":"e504a2b8-54dd-4c63-8083-aa8b7d622751","image":"/artists/DEAD_MALL.png","color":"#C084FC"},{"title":"Skin Memory","artist":"TWIN FLAME","suno":"b2b90892-b964-41d5-910e-f5ea96edf52e","image":"/artists/TWIN_FLAME.png","color":"#FB7185"},{"title":"Chrome Hearts","artist":"NITE_PARADE","suno":"c41c824d-27c2-4cd6-b9ad-82fac15b492d","image":"/artists/NITE_PARADE.png","color":"#818CF8"},{"title":"Tessellation","artist":"GLASS_FAUNA","suno":"85bd0297-989a-426b-b846-c506fc670d8a","image":"/artists/GLASS_FAUNA.png","color":"#34D399"},{"title":"Offerings","artist":"TEMPLE_DUST","suno":"f451a74f-8ee9-4dbd-9f4f-90fb6e088ee8","image":"/artists/TEMPLE_DUST.png","color":"#FBBF24"},{"title":"Blur Season","artist":"SOFTSIGNAL","suno":"0bfeb9c5-9678-47f6-9ad4-7624e15f3c8b","image":"/artists/SOFTSIGNAL.png","color":"#E879F9"},{"title":"Null Set","artist":"VANTA","suno":"bf2c12e8-9573-403e-bf81-5ad6ea990e7e","image":"/artists/VANTA.png","color":"#1E1E1E"},{"title":"Decon/struct","artist":"ØZONE","suno":"c8d34cde-9fea-4187-b321-9d93990a2db4","image":"/artists/OZONE.png","color":"#F43F5E"},{"title":"Collection No. 3","artist":"RAIN_MACHINE","suno":"dfae33be-4fbb-4614-9b38-4ab49e3d5307","image":"/artists/RAIN_MACHINE.png","color":"#67E8F9"},{"title":"Event Horizon","artist":"SABLE","suno":"52eb6cd6-fd14-4ee5-b880-ef4a97481d68","image":"/artists/SABLE.png","color":"#44403C"},{"title":"After the Signal","artist":"RUST_PROPHET","suno":"0e5b0086-1c1c-405c-bb80-86458cc0648c","image":"/artists/RUST_PROPHET.png","color":"#B45309"}];

  function shuffle(list) {
    var a = list.slice();
    for (var i = a.length - 1; i > 0; i--) {
      var j = Math.floor(Math.random() * (i + 1));
      var t = a[i];
      a[i] = a[j];
      a[j] = t;
    }
    return a;
  }

  function audioSrc(suno) {
    return ORIGIN + "/api/maestro/audio/" + encodeURIComponent(suno);
  }

  function artSrc(path) {
    if (!path) return "";
    if (path.indexOf("http") === 0) return path;
    return ORIGIN + path;
  }

  g.startRadio = function (root, opts) {
    if (!root || !CATALOG.length) return;
    opts = opts || {};
    var chrome = !!opts.chrome;
    var slot = opts.slot || null;
    var catalog = shuffle(CATALOG);
    var idx = 0;
    var playing = false;
    var failed = {};
    var raf = 0;
    var hue = 280;
    var mode = "dock";
    var corner = "br";
    var collapsed = false;
    var drag = null;
    var el = document.createElement("video");
    el.playsInline = true;
    el.preload = "auto";

    var bar = root.querySelector(".radio-bar");
    var art = root.querySelector(".radio-art");
    var title = root.querySelector(".radio-title");
    var artist = root.querySelector(".radio-artist");
    var playBtn = root.querySelector(".radio-play");
    var brand = root.querySelector(".mp-brand");
    var pinBtn = root.querySelector(".mp-pin");
    var collapseBtn = root.querySelector(".mp-collapse");
    var progress = root.querySelector(".mp-progress");

    try {
      var saved = JSON.parse(localStorage.getItem("vapurr.maestroDock") || "null");
      if (saved && (saved.mode === "dock" || saved.mode === "float")) mode = saved.mode;
      if (saved && /^(br|bl|tr|tl)$/.test(saved.corner)) corner = saved.corner;
      if (saved && saved.collapsed) collapsed = true;
    } catch (e) {}

    function track() {
      return catalog[idx];
    }

    function persist() {
      try {
        localStorage.setItem(
          "vapurr.maestroDock",
          JSON.stringify({ mode: mode, corner: corner, collapsed: collapsed })
        );
      } catch (e) {}
    }

    function applyLayout() {
      root.setAttribute("data-mode", mode);
      root.setAttribute("data-corner", corner);
      root.classList.toggle("is-collapsed", collapsed);
      root.classList.toggle("is-playing", playing);
      root.style.left = "";
      root.style.top = "";
      root.style.right = "";
      root.style.bottom = "";
      if (slot) {
        if (mode === "float") slot.style.height = "0px";
        else slot.style.height = collapsed ? "56px" : "78px";
      }
      if (pinBtn) {
        pinBtn.setAttribute("aria-label", mode === "float" ? "dock" : "undock");
        pinBtn.title = mode === "float" ? "Dock to bottom" : "Undock";
      }
      if (collapseBtn) {
        collapseBtn.setAttribute("aria-label", collapsed ? "expand" : "collapse");
        collapseBtn.title = collapsed ? "Expand" : "Collapse";
        collapseBtn.innerHTML = collapsed
          ? '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M6 10l6 6 6-6"/></svg>'
          : '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M6 14l6-6 6 6"/></svg>';
      }
      persist();
      if (chrome && g.vapurr && g.vapurr.send) {
        g.vapurr.send({
          cmd: "radio-layout",
          mode: mode,
          corner: corner,
          collapsed: collapsed
        });
      }
    }

    function paintPlay() {
      if (!playBtn) return;
      playBtn.setAttribute("aria-label", playing ? "pause" : "play");
      playBtn.innerHTML = playing
        ? '<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M6 5h4v14H6zm8 0h4v14h-4z"/></svg>'
        : '<svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>';
      root.classList.toggle("is-playing", playing);
    }

    function paint() {
      var t = track();
      if (!t) return;
      if (art) {
        art.src = artSrc(t.image);
        art.alt = t.artist;
      }
      if (title) title.textContent = t.title;
      if (artist) artist.textContent = t.artist;
      if (bar) {
        bar.style.width = "0%";
        bar.style.background = t.color || ("hsl(" + hue + ", 80%, 62%)");
      }
      paintPlay();
    }

    function go(delta) {
      if (!catalog.length) return;
      var next = idx;
      var found = false;
      for (var n = 0; n < catalog.length; n++) {
        next = (next + delta + catalog.length) % catalog.length;
        if (!failed[catalog[next].suno]) {
          found = true;
          break;
        }
      }
      if (!found) {
        playing = false;
        paintPlay();
        return;
      }
      idx = next;
      if (bar) bar.style.width = "0%";
      load();
    }

    function load() {
      var t = track();
      if (!t) return;
      paint();
      el.src = audioSrc(t.suno);
      el.load();
      if (playing) {
        el.play().catch(function () {
          playing = false;
          paintPlay();
        });
      }
    }

    function tick() {
      if (el.duration) {
        var p = Math.min(100, (el.currentTime / el.duration) * 100);
        if (bar) bar.style.width = p + "%";
      }
      raf = requestAnimationFrame(tick);
    }

    function setPlaying(on) {
      playing = !!on;
      paintPlay();
      if (playing) {
        if (!el.currentSrc) load();
        else {
          el.play().catch(function () {
            playing = false;
            paintPlay();
          });
        }
        cancelAnimationFrame(raf);
        raf = requestAnimationFrame(tick);
      } else {
        el.pause();
        cancelAnimationFrame(raf);
      }
    }

    function snapCorner(x, y) {
      var left = x < window.innerWidth / 2;
      var top = y < window.innerHeight / 2;
      if (top && left) return "tl";
      if (top && !left) return "tr";
      if (!top && left) return "bl";
      return "br";
    }

    function openMaestro(e) {
      if (e) {
        e.preventDefault();
        e.stopPropagation();
      }
      if (g.vapurr && g.vapurr.go) g.vapurr.go(ORIGIN + "/maestro-play");
    }

    el.addEventListener("ended", function () { go(1); });
    el.addEventListener("error", function () {
      var t = track();
      if (t) failed[t.suno] = 1;
      go(1);
    });

    var prev = root.querySelector(".radio-prev");
    var next = root.querySelector(".radio-next");
    var grip = root.querySelector(".mp-grip");
    if (prev) prev.addEventListener("click", function (e) { e.stopPropagation(); go(-1); });
    if (next) next.addEventListener("click", function (e) { e.stopPropagation(); go(1); });
    if (playBtn) {
      playBtn.addEventListener("click", function (e) {
        e.stopPropagation();
        setPlaying(!playing);
      });
    }
    if (brand) brand.addEventListener("click", openMaestro);
    if (pinBtn) {
      pinBtn.addEventListener("click", function (e) {
        e.preventDefault();
        e.stopPropagation();
        mode = mode === "float" ? "dock" : "float";
        if (mode === "float") collapsed = false;
        applyLayout();
      });
    }
    if (collapseBtn) {
      collapseBtn.addEventListener("click", function (e) {
        e.preventDefault();
        e.stopPropagation();
        collapsed = !collapsed;
        applyLayout();
      });
    }
    if (grip) {
      grip.addEventListener("click", function (e) {
        e.preventDefault();
        e.stopPropagation();
        if (mode === "dock") {
          mode = "float";
          collapsed = false;
          corner = "br";
        } else {
          var order = ["br", "bl", "tl", "tr"];
          var i = order.indexOf(corner);
          corner = order[(i + 1) % order.length];
        }
        applyLayout();
      });
    }
    if (progress) {
      progress.addEventListener("click", function (e) {
        e.stopPropagation();
        if (!el.duration) return;
        var r = progress.getBoundingClientRect();
        var p = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
        el.currentTime = p * el.duration;
        if (bar) bar.style.width = p * 100 + "%";
      });
    }

    function hueTick() {
      hue = (hue + 0.35) % 360;
      root.style.setProperty("--mp-h", hue.toFixed(1));
      requestAnimationFrame(hueTick);
    }

    applyLayout();
    paint();
    hueTick();
  };
})(window);
