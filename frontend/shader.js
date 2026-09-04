(function (g) {
  var VERT = `#version 300 es
in vec2 a_pos;
void main() {
  gl_Position = vec4(a_pos, 0.0, 1.0);
}`;

  var FRAG = `#version 300 es
precision highp float;
uniform vec2 u_res;
uniform float u_time;
uniform vec2 u_mouse;
uniform float u_light;
out vec4 fragColor;

vec2 rot(vec2 v, float a) {
  float s = sin(a), c = cos(a);
  return vec2(c * v.x - s * v.y, s * v.x + c * v.y);
}

float hash12(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

float vnoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  float a = hash12(i);
  float b = hash12(i + vec2(1.0, 0.0));
  float c = hash12(i + vec2(0.0, 1.0));
  float d = hash12(i + vec2(1.0, 1.0));
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float neuro(vec2 uv, float t) {
  vec2 acc = vec2(0.0);
  vec2 res = vec2(0.0);
  float scale = 8.0;
  for (int j = 0; j < 14; j++) {
    uv = rot(uv, 1.0);
    acc = rot(acc, 1.0);
    vec2 layer = uv * scale + float(j) + acc - t;
    acc += sin(layer);
    res += (0.5 + 0.5 * cos(layer)) / scale;
    scale *= 1.2;
  }
  return res.x + res.y;
}

void main() {
  vec2 frag = gl_FragCoord.xy;
  vec2 uv = (frag - 0.5 * u_res) / u_res.y;
  vec2 nUV = frag / u_res;
  float t = u_time;

  vec2 m = u_mouse;
  float md = exp(-dot(uv - m * 0.55, uv - m * 0.55) * 1.8);
  uv += 0.16 * m * md;
  uv = rot(uv, t * 0.017);

  vec2 w = uv * 1.55;
  w += 0.38 * vec2(vnoise(w + vec2(t * 0.06, -t * 0.04)), vnoise(w + vec2(3.7, 1.1) - t * 0.05));
  float wash = vnoise(w * 1.1 + t * 0.03);
  wash = smoothstep(0.18, 0.88, wash);

  vec3 voidC = mix(vec3(0.055), vec3(0.953, 0.961, 0.941), u_light);
  vec3 forest = mix(vec3(0.165, 0.220, 0.0), vec3(0.784, 0.839, 0.690), u_light);
  vec3 lime = mix(vec3(0.753, 0.973, 0.0), vec3(0.302, 0.541, 0.0), u_light);
  vec3 col = mix(voidC, forest, wash * mix(0.62, 0.38, u_light));
  col = mix(col, lime * mix(0.32, 0.18, u_light), pow(wash, 2.0) * mix(0.55, 0.28, u_light));

  vec2 nu = uv * 0.21;
  float n = neuro(nu, t * 0.46);
  n = max(n, 0.0);
  float filament = smoothstep(0.28, 0.72, n);
  float glow = smoothstep(0.48, 1.05, n);
  glow = pow(glow, 1.35);

  col = mix(col, lime * mix(0.42, 0.22, u_light), filament * mix(0.55, 0.16, u_light));
  col += lime * filament * mix(0.38, 0.06, u_light);
  col += lime * glow * mix(1.05, 0.14, u_light);
  col += mix(vec3(1.0, 1.0, 0.42), forest, u_light) * pow(glow, 3.2) * mix(0.32, 0.05, u_light);

  float ray = pow(max(0.0, dot(normalize(uv + vec2(0.0, 0.12)), rot(vec2(0.0, 1.0), t * 0.1))), 48.0);
  col += lime * ray * mix(0.08, 0.025, u_light);

  float liftD = mix(0.72, 1.12, smoothstep(0.72, 0.18, nUV.y));
  float liftL = mix(1.03, 0.97, smoothstep(0.72, 0.18, nUV.y));
  col *= mix(liftD, liftL, u_light);

  float vig = smoothstep(1.15, 0.28, length((nUV - 0.5) * vec2(1.15, 1.0)));
  col *= mix(mix(0.62, 1.0, vig), mix(0.94, 1.0, vig), u_light);

  float grain = hash12(frag + fract(t * 13.7) * 47.0) - 0.5;
  col += grain * mix(0.045, 0.016, u_light);
  col += (hash12(frag * 1.37 + t) - 0.5) * mix(1.0 / 220.0, 1.0 / 380.0, u_light);

  col = clamp(col, 0.0, 1.0);
  fragColor = vec4(col, 1.0);
}`;

  function compile(gl, type, src) {
    var sh = gl.createShader(type);
    gl.shaderSource(sh, src);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
      console.warn("vapurr shader", gl.getShaderInfoLog(sh));
      gl.deleteShader(sh);
      return null;
    }
    return sh;
  }

  function startShader(canvas) {
    if (!canvas || canvas.__vapurrShade) return;
    canvas.__vapurrShade = true;
    if (g.__vapurrWhenPaint) {
      g.__vapurrWhenPaint(function () { bootShader(canvas); });
      return;
    }
    bootShader(canvas);
  }

  function bootShader(canvas) {
    var gl = canvas.getContext("webgl2", {
      alpha: false,
      antialias: false,
      depth: false,
      stencil: false,
      premultipliedAlpha: true,
      powerPreference: "high-performance",
    });
    if (!gl) return;

    var vs = compile(gl, gl.VERTEX_SHADER, VERT);
    var fs = compile(gl, gl.FRAGMENT_SHADER, FRAG);
    if (!vs || !fs) return;
    var prog = gl.createProgram();
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      console.warn("vapurr shader link", gl.getProgramInfoLog(prog));
      return;
    }
    gl.useProgram(prog);
    var vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    var buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 3, -1, -1, 3]),
      gl.STATIC_DRAW
    );
    var loc = gl.getAttribLocation(prog, "a_pos");
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);

    var uRes = gl.getUniformLocation(prog, "u_res");
    var uTime = gl.getUniformLocation(prog, "u_time");
    var uMouse = gl.getUniformLocation(prog, "u_mouse");
    var uLight = gl.getUniformLocation(prog, "u_light");

    var reduced =
      g.matchMedia && g.matchMedia("(prefers-reduced-motion: reduce)").matches;
    var mx = 0,
      my = 0,
      tx = 0,
      ty = 0;
    var t0 = performance.now();
    var running = true;

    function lightAmt() {
      return document.documentElement.getAttribute("data-theme") === "light" ? 1.0 : 0.0;
    }

    function fit() {
      var dpr = Math.min(g.devicePixelRatio || 1, 1.75);
      var w = canvas.clientWidth || g.innerWidth || 1;
      var h = canvas.clientHeight || g.innerHeight || 1;
      var W = Math.max(1, Math.floor(w * dpr));
      var H = Math.max(1, Math.floor(h * dpr));
      if (canvas.width !== W || canvas.height !== H) {
        canvas.width = W;
        canvas.height = H;
        gl.bindVertexArray(vao);
        gl.useProgram(prog);
      }
      gl.viewport(0, 0, W, H);
      gl.uniform2f(uRes, W, H);
    }

    function onMove(e) {
      var r = canvas.getBoundingClientRect();
      if (!r.width || !r.height) return;
      tx = ((e.clientX - r.left) / r.width) * 2 - 1;
      ty = -(((e.clientY - r.top) / r.height) * 2 - 1);
    }

    g.addEventListener("pointermove", onMove, { passive: true });
    g.addEventListener("resize", fit);
    if (typeof ResizeObserver !== "undefined") {
      new ResizeObserver(fit).observe(canvas);
    }

    function draw(now) {
      mx += (tx - mx) * 0.045;
      my += (ty - my) * 0.045;
      var t = reduced ? 1.4 : (now - t0) / 1000;
      gl.uniform1f(uTime, t);
      gl.uniform2f(uMouse, mx, my);
      gl.uniform1f(uLight, lightAmt());
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    }

    function frame(now) {
      if (!running) return;
      draw(now);
      if (!reduced && !g.document.hidden) g.requestAnimationFrame(frame);
    }

    fit();
    draw(t0);
    if (!reduced) g.requestAnimationFrame(frame);
    g.document.addEventListener("visibilitychange", function () {
      if (!g.document.hidden && !reduced) g.requestAnimationFrame(frame);
    });
    if (typeof MutationObserver !== "undefined") {
      new MutationObserver(function () {
        draw(performance.now());
      }).observe(document.documentElement, {
        attributes: true,
        attributeFilter: ["data-theme"],
      });
    }
  }

  g.startShader = startShader;
})(window);
