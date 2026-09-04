// Byte-mode QR, ECC L, versions 1-4. Receive addresses fit in v3.
(function (root) {
  var EXP = new Array(512);
  var LOG = new Array(256);
  (function () {
    var x = 1;
    for (var i = 0; i < 255; i++) {
      EXP[i] = x;
      LOG[x] = i;
      x <<= 1;
      if (x & 0x100) x ^= 0x11d;
    }
    for (var j = 255; j < 512; j++) EXP[j] = EXP[j - 255];
  })();

  function gfMul(a, b) {
    if (!a || !b) return 0;
    return EXP[LOG[a] + LOG[b]];
  }

  var DATA_CW = [0, 19, 34, 55, 80];
  var EC_CW = [0, 7, 10, 15, 20];
  var ALIGN = [null, [], [18], [22], [26]];

  function rsGen(n) {
    var g = [1];
    for (var i = 0; i < n; i++) {
      var ng = new Array(g.length + 1).fill(0);
      for (var j = 0; j < g.length; j++) {
        ng[j] ^= g[j];
        ng[j + 1] ^= gfMul(g[j], EXP[i]);
      }
      g = ng;
    }
    return g;
  }

  function rsEncode(data, n) {
    var g = rsGen(n);
    var rest = new Array(n).fill(0);
    for (var i = 0; i < data.length; i++) {
      var factor = data[i] ^ rest[0];
      rest.shift();
      rest.push(0);
      if (!factor) continue;
      for (var j = 0; j < n; j++) rest[j] ^= gfMul(g[j + 1], factor);
    }
    return rest;
  }

  function bitsToBytes(bits) {
    var out = [];
    for (var i = 0; i < bits.length; i += 8) {
      var b = 0;
      for (var j = 0; j < 8; j++) b = (b << 1) | (bits[i + j] || 0);
      out.push(b);
    }
    return out;
  }

  function encodeData(text) {
    var bytes = [];
    for (var i = 0; i < text.length; i++) bytes.push(text.charCodeAt(i) & 255);
    var need = bytes.length + 2;
    var ver = 1;
    while (ver <= 4 && DATA_CW[ver] < need + 1) ver++;
    if (ver > 4) throw new Error("qr too long");
    var cap = DATA_CW[ver] * 8;
    var bits = [0, 1, 0, 0];
    for (var c = 7; c >= 0; c--) bits.push((bytes.length >> c) & 1);
    bytes.forEach(function (b) {
      for (var k = 7; k >= 0; k--) bits.push((b >> k) & 1);
    });
    var term = Math.min(4, cap - bits.length);
    for (var t = 0; t < term; t++) bits.push(0);
    while (bits.length % 8) bits.push(0);
    var pad = [1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1];
    var p = 0;
    while (bits.length < cap) {
      bits.push(pad[p % 16]);
      p++;
    }
    var data = bitsToBytes(bits).slice(0, DATA_CW[ver]);
    return { ver: ver, data: data.concat(rsEncode(data, EC_CW[ver])) };
  }

  function formatBits(mask) {
    var data = (1 << 3) | mask;
    var rem = data << 10;
    for (var i = 14; i >= 10; i--) {
      if ((rem >>> i) & 1) rem ^= 0x537 << (i - 10);
    }
    return ((data << 10) | (rem & 0x3ff)) ^ 0x5412;
  }

  function placeFinders(m, n) {
    function finder(r0, c0) {
      for (var r = -1; r <= 7; r++) {
        for (var c = -1; c <= 7; c++) {
          var rr = r0 + r, cc = c0 + c;
          if (rr < 0 || cc < 0 || rr >= n || cc >= n) continue;
          var dark =
            r >= 0 && r <= 6 && c >= 0 && c <= 6 &&
            (r === 0 || r === 6 || c === 0 || c === 6 || (r >= 2 && r <= 4 && c >= 2 && c <= 4));
          m[rr][cc] = dark ? 1 : 0;
        }
      }
    }
    finder(0, 0);
    finder(0, n - 7);
    finder(n - 7, 0);
  }

  function reservedMap(n, ver) {
    var r = [];
    for (var i = 0; i < n; i++) r.push(new Array(n).fill(0));
    function markFinder(r0, c0) {
      for (var y = -1; y <= 7; y++) {
        for (var x = -1; x <= 7; x++) {
          var rr = r0 + y, cc = c0 + x;
          if (rr >= 0 && cc >= 0 && rr < n && cc < n) r[rr][cc] = 1;
        }
      }
    }
    markFinder(0, 0);
    markFinder(0, n - 7);
    markFinder(n - 7, 0);
    for (var k = 0; k < n; k++) {
      r[6][k] = 1;
      r[k][6] = 1;
    }
    for (var a = 0; a < 9; a++) {
      r[8][a] = 1;
      r[a][8] = 1;
      r[8][n - 1 - a] = 1;
      r[n - 1 - a][8] = 1;
    }
    r[8][n - 8] = 1;
    ALIGN[ver].forEach(function (pos) {
      for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
          r[pos + y][pos + x] = 1;
        }
      }
    });
    return r;
  }

  function placeAlign(m, ver) {
    ALIGN[ver].forEach(function (pos) {
      for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
          m[pos + y][pos + x] = y === -2 || y === 2 || x === -2 || x === 2 || (y === 0 && x === 0) ? 1 : 0;
        }
      }
    });
  }

  function maskAt(mask, r, c) {
    switch (mask) {
      case 0: return (r + c) % 2 === 0;
      case 1: return r % 2 === 0;
      case 2: return c % 3 === 0;
      case 3: return (r + c) % 3 === 0;
      case 4: return (Math.floor(r / 2) + Math.floor(c / 3)) % 2 === 0;
      case 5: return ((r * c) % 2) + ((r * c) % 3) === 0;
      case 6: return (((r * c) % 2) + ((r * c) % 3)) % 2 === 0;
      default: return (((r + c) % 2) + ((r * c) % 3)) % 2 === 0;
    }
  }

  function penalty(m) {
    var n = m.length, s = 0;
    function runScore(run) {
      if (run >= 5) s += run - 2;
    }
    for (var r = 0; r < n; r++) {
      var run = 1;
      for (var c = 1; c < n; c++) {
        if (m[r][c] === m[r][c - 1]) run++;
        else { runScore(run); run = 1; }
      }
      runScore(run);
    }
    for (var c2 = 0; c2 < n; c2++) {
      var run2 = 1;
      for (var r2 = 1; r2 < n; r2++) {
        if (m[r2][c2] === m[r2 - 1][c2]) run2++;
        else { runScore(run2); run2 = 1; }
      }
      runScore(run2);
    }
    for (var y = 0; y < n - 1; y++) {
      for (var x = 0; x < n - 1; x++) {
        var v = m[y][x];
        if (v === m[y][x + 1] && v === m[y + 1][x] && v === m[y + 1][x + 1]) s += 3;
      }
    }
    var dark = 0;
    for (var i = 0; i < n; i++) for (var j = 0; j < n; j++) if (m[i][j]) dark++;
    var pct = Math.floor((100 * dark) / (n * n));
    s += Math.floor(Math.abs(pct - 50) / 5) * 10;
    return s;
  }

  function build(text) {
    var enc = encodeData(text);
    var ver = enc.ver;
    var n = ver * 4 + 17;
    var reserved = reservedMap(n, ver);
    var best = null;
    var bestScore = 1e9;
    for (var mask = 0; mask < 8; mask++) {
      var m = [];
      for (var i = 0; i < n; i++) m.push(new Array(n).fill(0));
      placeFinders(m, n);
      placeAlign(m, ver);
      for (var t = 0; t < n; t++) {
        m[6][t] = t % 2 === 0 ? 1 : 0;
        m[t][6] = t % 2 === 0 ? 1 : 0;
      }
      m[8][n - 8] = 1;
      var bits = [];
      enc.data.forEach(function (b) {
        for (var k = 7; k >= 0; k--) bits.push((b >> k) & 1);
      });
      var bi = 0;
      var dir = -1;
      var row = n - 1;
      for (var col = n - 1; col > 0; col -= 2) {
        if (col === 6) col--;
        for (;;) {
          for (var dx = 0; dx < 2; dx++) {
            var c = col - dx;
            if (reserved[row][c]) continue;
            var bit = bits[bi] || 0;
            bi++;
            if (maskAt(mask, row, c)) bit ^= 1;
            m[row][c] = bit;
          }
          row += dir;
          if (row < 0 || row >= n) {
            row -= dir;
            dir = -dir;
            break;
          }
        }
      }
      var fmt = formatBits(mask);
      var horiz = [0, 1, 2, 3, 4, 5, 7, 8];
      var vert = [8, 7, 5, 4, 3, 2, 1, 0];
      for (var f = 0; f < 15; f++) {
        var bit = (fmt >> f) & 1;
        if (f < 8) m[8][horiz[f]] = bit;
        else m[vert[f - 7]][8] = bit;
        if (f < 7) m[n - 1 - f][8] = bit;
        else m[8][n - 15 + f] = bit;
      }
      var score = penalty(m);
      if (score < bestScore) {
        bestScore = score;
        best = m;
      }
    }
    return best;
  }

  function drawQr(canvas, text) {
    if (!canvas || !text) return false;
    var m;
    try { m = build(String(text)); } catch (e) { return false; }
    var n = m.length;
    var ctx = canvas.getContext("2d");
    var w = canvas.width;
    var quiet = 4;
    var modules = n + quiet * 2;
    var cell = w / modules;
    var light = document.documentElement.getAttribute("data-theme") === "light";
    ctx.fillStyle = light ? "#f3f5f0" : "#0e0e0e";
    ctx.fillRect(0, 0, w, w);
    ctx.fillStyle = light ? "#161816" : "#c0f800";
    for (var r = 0; r < n; r++) {
      for (var c = 0; c < n; c++) {
        if (!m[r][c]) continue;
        ctx.fillRect((c + quiet) * cell, (r + quiet) * cell, cell + 0.4, cell + 0.4);
      }
    }
    return true;
  }

  root.drawQr = drawQr;
})(window);
