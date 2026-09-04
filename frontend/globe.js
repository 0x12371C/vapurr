import * as THREE from "three/webgpu";

const LIME = 0xc0f800;
const LIME_DIM = 0x6a8a00;
const LIME_LIGHT = 0x4d8a00;
const FOREST = 0x2a3800;
const VOID = 0x0e0e0e;

function isLight() {
  return document.documentElement.getAttribute("data-theme") === "light";
}

function whenPaint(fn) {
  if (window.__vapurrWhenPaint) {
    window.__vapurrWhenPaint(fn);
    return;
  }
  fn();
}

export function startGlobe(canvas) {
  if (!canvas) return function () {};

  let dead = false;
  let stopInner = function () {
    dead = true;
  };

  whenPaint(function () {
    boot().catch(function (err) {
      console.warn("pusd scene", err);
    });
  });

  async function boot() {
    const renderer = new THREE.WebGPURenderer({
      canvas: canvas,
      antialias: true,
      alpha: true,
      powerPreference: "high-performance",
      stencil: false,
    });
    renderer.toneMapping = THREE.NoToneMapping;
    renderer.toneMappingExposure = 1;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.setClearColor(0x000000, 0);
    await renderer.init();
    if (dead) {
      renderer.dispose();
      return;
    }
    canvas.dataset.gpu = renderer.backend && renderer.backend.isWebGPUBackend ? "webgpu" : "webgl2";

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(32, 1, 0.4, 60);
    camera.position.set(0, 0.55, 10.4);
    camera.lookAt(0, 0.05, 0);

    const globe = new THREE.Group();
    globe.name = "faucet.globe";
    globe.position.set(0, -1.25, -1.1);
    globe.rotation.x = 0.42;
    scene.add(globe);

    const R = 4.9;
    const latN = 56;
    const lonEq = 120;
    const pts = [];
    for (let i = 4; i <= 26; i++) {
      const lat = (i / latN) * Math.PI;
      const y = Math.cos(lat);
      const ring = Math.sin(lat);
      const lonN = Math.max(12, Math.round(lonEq * ring));
      for (let j = 0; j < lonN; j++) {
        const lon = ((j + 0.12) / lonN) * Math.PI * 2;
        const n = new THREE.Vector3(ring * Math.cos(lon), y, ring * Math.sin(lon));
        n.x += (Math.random() - 0.5) * 0.01;
        n.y += (Math.random() - 0.5) * 0.01;
        n.z += (Math.random() - 0.5) * 0.01;
        n.normalize();
        pts.push(n);
      }
    }

    const mat = new THREE.MeshStandardMaterial({
      color: LIME,
      emissive: LIME,
      emissiveIntensity: 0.22,
      roughness: 0.4,
      metalness: 0.12,
    });
    const dotGeo = new THREE.SphereGeometry(0.009, 10, 8);
    const dots = new THREE.InstancedMesh(dotGeo, mat, pts.length);
    dots.name = "faucet.dots";
    const dummy = new THREE.Object3D();
    const front = new THREE.Color(LIME);
    const back = new THREE.Color(LIME_DIM);
    const col = new THREE.Color();
    for (let i = 0; i < pts.length; i++) {
      const p = pts[i];
      dummy.position.copy(p).multiplyScalar(R);
      const facing = Math.max(0, p.z);
      dummy.scale.setScalar(0.5 + facing * 0.45);
      dummy.updateMatrix();
      dots.setMatrixAt(i, dummy.matrix);
      col.copy(back).lerp(front, 0.45 + facing * 0.55);
      dots.setColorAt(i, col);
    }
    if (dots.instanceColor) dots.instanceColor.needsUpdate = true;
    globe.add(dots);

    scene.add(new THREE.AmbientLight(FOREST, 0.5));
    const hemi = new THREE.HemisphereLight(LIME, VOID, 0.65);
    scene.add(hemi);
    const key = new THREE.DirectionalLight(0xffffff, 0.9);
    key.position.set(-2.2, 4.2, 6.4);
    scene.add(key);
    const rim = new THREE.DirectionalLight(LIME, 0.55);
    rim.position.set(3.4, 0.6, -4.2);
    scene.add(rim);

    function applyQuality() {
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      resize();
    }
    function resize() {
      const w = canvas.clientWidth || window.innerWidth;
      const h = canvas.clientHeight || window.innerHeight;
      if (w < 2 || h < 2) return;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    }
    applyQuality();

    let mx = 0;
    let my = 0;
    let tx = 0;
    let ty = 0;
    function onMove(e) {
      tx = (e.clientX / (window.innerWidth || 1)) * 2 - 1;
      ty = (e.clientY / (window.innerHeight || 1)) * 2 - 1;
    }
    window.addEventListener("pointermove", onMove, { passive: true });
    window.addEventListener("resize", resize);

    function paintDots(frontHex, backHex) {
      front.setHex(frontHex);
      back.setHex(backHex);
      for (let i = 0; i < pts.length; i++) {
        const facing = Math.max(0, pts[i].z);
        col.copy(back).lerp(front, 0.45 + facing * 0.55);
        dots.setColorAt(i, col);
      }
      if (dots.instanceColor) dots.instanceColor.needsUpdate = true;
    }

    function applyTheme() {
      const hex = isLight() ? LIME_LIGHT : LIME;
      mat.color.setHex(hex);
      mat.emissive.setHex(hex);
      hemi.color.setHex(hex);
      rim.color.setHex(hex);
      if (isLight()) paintDots(LIME_LIGHT, 0x6a8a00);
      else paintDots(LIME, LIME_DIM);
    }
    applyTheme();
    const themeObs = new MutationObserver(applyTheme);
    themeObs.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });

    const reduced =
      window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
    const t0 = performance.now();

    function tick(now) {
      if (dead) return;
      mx += (tx - mx) * 0.035;
      my += (ty - my) * 0.035;
      const t = reduced ? 0 : (now - t0) * 0.001;
      globe.rotation.y = t * 0.04;
      camera.position.set(mx * 0.32, 0.55 - my * 0.14, 10.4);
      camera.lookAt(mx * 0.1, 0.05 - my * 0.05, 0);
      renderer.render(scene, camera);
    }

    function onVis() {
      renderer.setAnimationLoop(!document.hidden && !dead ? tick : null);
    }
    document.addEventListener("visibilitychange", onVis);
    if (!reduced) renderer.setAnimationLoop(tick);
    else tick(performance.now());

    stopInner = function () {
      dead = true;
      renderer.setAnimationLoop(null);
      document.removeEventListener("visibilitychange", onVis);
      window.removeEventListener("resize", resize);
      window.removeEventListener("pointermove", onMove);
      themeObs.disconnect();
      renderer.dispose();
      mat.dispose();
      dotGeo.dispose();
      dots.dispose();
    };
  }

  return function () {
    stopInner();
  };
}

function globePts(latN, lonEq, i0, i1) {
  const pts = [];
  for (let i = i0; i <= i1; i++) {
    const lat = (i / latN) * Math.PI;
    const y = Math.cos(lat);
    const ring = Math.sin(lat);
    const lonN = Math.max(10, Math.round(lonEq * ring));
    for (let j = 0; j < lonN; j++) {
      const lon = ((j + 0.12) / lonN) * Math.PI * 2;
      const n = new THREE.Vector3(ring * Math.cos(lon), y, ring * Math.sin(lon));
      n.x += (Math.random() - 0.5) * 0.012;
      n.y += (Math.random() - 0.5) * 0.012;
      n.z += (Math.random() - 0.5) * 0.012;
      n.normalize();
      pts.push(n);
    }
  }
  return pts;
}

function placeGlobe(mesh, pts, R, dummy, front, back, col) {
  for (let i = 0; i < pts.length; i++) {
    const p = pts[i];
    dummy.position.copy(p).multiplyScalar(R);
    const facing = Math.max(0, p.z);
    dummy.scale.setScalar(0.5 + facing * 0.45);
    dummy.updateMatrix();
    mesh.setMatrixAt(i, dummy.matrix);
    col.copy(back).lerp(front, 0.45 + facing * 0.55);
    mesh.setColorAt(i, col);
  }
  if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
}

function bez(a, b, c, t, out) {
  const u = 1 - t;
  out.copy(a).multiplyScalar(u * u);
  out.addScaledVector(b, 2 * u * t);
  out.addScaledVector(c, t * t);
  return out;
}

/// Two dotted worlds, one arc. Same lime dots as PUSD — a bridge, not the faucet globe.
export function startBridge(canvas) {
  if (!canvas) return function () {};

  let dead = false;
  let stopInner = function () {
    dead = true;
  };

  whenPaint(function () {
    boot().catch(function (err) {
      console.warn("bridge scene", err);
    });
  });

  async function boot() {
    const renderer = new THREE.WebGPURenderer({
      canvas: canvas,
      antialias: true,
      alpha: true,
      powerPreference: "high-performance",
      stencil: false,
    });
    renderer.toneMapping = THREE.NoToneMapping;
    renderer.toneMappingExposure = 1;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.setClearColor(0x000000, 0);
    await renderer.init();
    if (dead) {
      renderer.dispose();
      return;
    }
    canvas.dataset.gpu = renderer.backend && renderer.backend.isWebGPUBackend ? "webgpu" : "webgl2";

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(30, 1, 0.4, 70);
    camera.position.set(0, 0.7, 13.2);
    camera.lookAt(0, 0.35, 0);

    const span = new THREE.Group();
    span.name = "bridge.span";
    span.position.set(0, -0.55, -0.6);
    scene.add(span);

    const R = 2.15;
    const leftC = new THREE.Vector3(-3.55, -0.55, 0);
    const rightC = new THREE.Vector3(3.55, -0.55, 0);
    const ptsL = globePts(42, 78, 5, 21);
    const ptsR = globePts(42, 78, 5, 21);

    const mat = new THREE.MeshStandardMaterial({
      color: LIME,
      emissive: LIME,
      emissiveIntensity: 0.22,
      roughness: 0.4,
      metalness: 0.12,
    });
    const matArc = mat.clone();
    matArc.emissiveIntensity = 0.55;
    const matBead = mat.clone();
    matBead.emissiveIntensity = 0.9;

    const dotGeo = new THREE.SphereGeometry(0.009, 10, 8);
    const dummy = new THREE.Object3D();
    const front = new THREE.Color(LIME);
    const back = new THREE.Color(LIME_DIM);
    const col = new THREE.Color();

    const left = new THREE.Group();
    left.position.copy(leftC);
    left.rotation.x = 0.38;
    const dotsL = new THREE.InstancedMesh(dotGeo, mat, ptsL.length);
    placeGlobe(dotsL, ptsL, R, dummy, front, back, col);
    left.add(dotsL);
    span.add(left);

    const right = new THREE.Group();
    right.position.copy(rightC);
    right.rotation.x = 0.38;
    const dotsR = new THREE.InstancedMesh(dotGeo, mat, ptsR.length);
    placeGlobe(dotsR, ptsR, R, dummy, front, back, col);
    right.add(dotsR);
    span.add(right);

    const p0 = new THREE.Vector3(-0.92, 0.48, 0.28).normalize().multiplyScalar(R).add(leftC);
    const p1 = new THREE.Vector3(0, 2.55, 1.15);
    const p2 = new THREE.Vector3(0.92, 0.48, 0.28).normalize().multiplyScalar(R).add(rightC);

    const ARC_N = 52;
    const arc = new THREE.InstancedMesh(dotGeo, matArc, ARC_N);
    const tmp = new THREE.Vector3();
    for (let i = 0; i < ARC_N; i++) {
      const t = i / (ARC_N - 1);
      bez(p0, p1, p2, t, tmp);
      dummy.position.copy(tmp);
      dummy.scale.setScalar(0.55 + Math.sin(t * Math.PI) * 0.55);
      dummy.updateMatrix();
      arc.setMatrixAt(i, dummy.matrix);
      col.copy(back).lerp(front, 0.35 + Math.sin(t * Math.PI) * 0.65);
      arc.setColorAt(i, col);
    }
    if (arc.instanceColor) arc.instanceColor.needsUpdate = true;
    span.add(arc);

    const beadGeo = new THREE.SphereGeometry(0.042, 12, 10);
    const beads = [0, 1, 2].map(function () {
      const m = new THREE.Mesh(beadGeo, matBead);
      span.add(m);
      return m;
    });

    scene.add(new THREE.AmbientLight(FOREST, 0.5));
    const hemi = new THREE.HemisphereLight(LIME, VOID, 0.65);
    scene.add(hemi);
    const key = new THREE.DirectionalLight(0xffffff, 0.9);
    key.position.set(-2.2, 4.2, 6.4);
    scene.add(key);
    const rim = new THREE.DirectionalLight(LIME, 0.55);
    rim.position.set(3.4, 0.6, -4.2);
    scene.add(rim);

    function applyQuality() {
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      resize();
    }
    function resize() {
      const w = canvas.clientWidth || window.innerWidth;
      const h = canvas.clientHeight || window.innerHeight;
      if (w < 2 || h < 2) return;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    }
    applyQuality();

    let mx = 0;
    let my = 0;
    let tx = 0;
    let ty = 0;
    function onMove(e) {
      tx = (e.clientX / (window.innerWidth || 1)) * 2 - 1;
      ty = (e.clientY / (window.innerHeight || 1)) * 2 - 1;
    }
    window.addEventListener("pointermove", onMove, { passive: true });
    window.addEventListener("resize", resize);

    function paintGlobe(mesh, pts, fHex, bHex) {
      front.setHex(fHex);
      back.setHex(bHex);
      for (let i = 0; i < pts.length; i++) {
        const facing = Math.max(0, pts[i].z);
        col.copy(back).lerp(front, 0.45 + facing * 0.55);
        mesh.setColorAt(i, col);
      }
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    }

    function applyTheme() {
      const hex = isLight() ? LIME_LIGHT : LIME;
      const dim = isLight() ? 0x6a8a00 : LIME_DIM;
      [mat, matArc, matBead].forEach(function (m) {
        m.color.setHex(hex);
        m.emissive.setHex(hex);
      });
      hemi.color.setHex(hex);
      rim.color.setHex(hex);
      paintGlobe(dotsL, ptsL, hex, dim);
      paintGlobe(dotsR, ptsR, hex, dim);
      front.setHex(hex);
      back.setHex(dim);
      for (let i = 0; i < ARC_N; i++) {
        const t = i / (ARC_N - 1);
        col.copy(back).lerp(front, 0.35 + Math.sin(t * Math.PI) * 0.65);
        arc.setColorAt(i, col);
      }
      if (arc.instanceColor) arc.instanceColor.needsUpdate = true;
    }
    applyTheme();
    const themeObs = new MutationObserver(applyTheme);
    themeObs.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });

    const reduced =
      window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
    const t0 = performance.now();
    const beadPos = new THREE.Vector3();

    function tick(now) {
      if (dead) return;
      mx += (tx - mx) * 0.035;
      my += (ty - my) * 0.035;
      const t = reduced ? 0 : (now - t0) * 0.001;
      left.rotation.y = t * 0.045;
      right.rotation.y = -t * 0.038;
      if (!reduced) {
        for (let i = 0; i < beads.length; i++) {
          const u = (t * 0.12 + i / beads.length) % 1;
          bez(p0, p1, p2, u, beadPos);
          beads[i].position.copy(beadPos);
          beads[i].scale.setScalar(0.7 + Math.sin(u * Math.PI) * 0.55);
        }
      }
      camera.position.set(mx * 0.38, 0.7 - my * 0.16, 13.2);
      camera.lookAt(mx * 0.12, 0.35 - my * 0.05, 0);
      renderer.render(scene, camera);
    }

    function onVis() {
      renderer.setAnimationLoop(!document.hidden && !dead ? tick : null);
    }
    document.addEventListener("visibilitychange", onVis);
    if (!reduced) renderer.setAnimationLoop(tick);
    else tick(performance.now());

    stopInner = function () {
      dead = true;
      renderer.setAnimationLoop(null);
      document.removeEventListener("visibilitychange", onVis);
      window.removeEventListener("resize", resize);
      window.removeEventListener("pointermove", onMove);
      themeObs.disconnect();
      renderer.dispose();
      mat.dispose();
      matArc.dispose();
      matBead.dispose();
      dotGeo.dispose();
      beadGeo.dispose();
      dotsL.dispose();
      dotsR.dispose();
      arc.dispose();
    };
  }

  return function () {
    stopInner();
  };
}

function orbitPts(n, radius, tiltX, tiltZ) {
  const e = new THREE.Euler(tiltX, 0, tiltZ, "XYZ");
  const pts = [];
  for (let i = 0; i < n; i++) {
    const a = (i / n) * Math.PI * 2;
    const p = new THREE.Vector3(Math.cos(a) * radius, 0, Math.sin(a) * radius);
    p.applyEuler(e);
    pts.push(p);
  }
  return pts;
}

function placeCloud(mesh, pts, dummy, front, back, col, scale) {
  for (let i = 0; i < pts.length; i++) {
    const p = pts[i];
    dummy.position.copy(p);
    const facing = Math.max(0, p.z / (p.length() || 1));
    dummy.scale.setScalar((scale || 1) * (0.45 + facing * 0.55));
    dummy.updateMatrix();
    mesh.setMatrixAt(i, dummy.matrix);
    col.copy(back).lerp(front, 0.3 + facing * 0.7);
    mesh.setColorAt(i, col);
  }
  if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
}

/// Nested lime-dot gyre + two counter-rotating orbits. Swap, not a faucet globe.
export function startSwap(canvas) {
  if (!canvas) return function () {};

  let dead = false;
  let stopInner = function () {
    dead = true;
  };

  whenPaint(function () {
    boot().catch(function (err) {
      console.warn("swap scene", err);
    });
  });

  async function boot() {
    const renderer = new THREE.WebGPURenderer({
      canvas: canvas,
      antialias: true,
      alpha: true,
      powerPreference: "high-performance",
      stencil: false,
    });
    renderer.toneMapping = THREE.NoToneMapping;
    renderer.toneMappingExposure = 1;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.setClearColor(0x000000, 0);
    await renderer.init();
    if (dead) {
      renderer.dispose();
      return;
    }
    canvas.dataset.gpu = renderer.backend && renderer.backend.isWebGPUBackend ? "webgpu" : "webgl2";

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(32, 1, 0.4, 70);
    camera.position.set(0, 0.5, 11.6);
    camera.lookAt(0, 0.1, 0);

    const gyre = new THREE.Group();
    gyre.name = "swap.gyre";
    gyre.position.set(0, -0.95, -0.8);
    gyre.rotation.x = 0.28;
    scene.add(gyre);

    const coreR = 2.05;
    const shellR = 3.45;
    const ptsCore = globePts(40, 96, 4, 22);
    const ptsShell = globePts(48, 88, 3, 24);
    const ringA = orbitPts(72, 3.05, 0.62, 0.18);
    const ringB = orbitPts(64, 3.42, -0.95, 0.4);

    const mat = new THREE.MeshStandardMaterial({
      color: LIME,
      emissive: LIME,
      emissiveIntensity: 0.2,
      roughness: 0.42,
      metalness: 0.12,
    });
    const matShell = mat.clone();
    matShell.emissiveIntensity = 0.12;
    const matRing = mat.clone();
    matRing.emissiveIntensity = 0.62;
    const matBead = mat.clone();
    matBead.emissiveIntensity = 1.05;

    const dotGeo = new THREE.SphereGeometry(0.009, 10, 8);
    const beadGeo = new THREE.SphereGeometry(0.046, 12, 10);
    const dummy = new THREE.Object3D();
    const front = new THREE.Color(LIME);
    const back = new THREE.Color(LIME_DIM);
    const col = new THREE.Color();

    const core = new THREE.Group();
    const dotsC = new THREE.InstancedMesh(dotGeo, mat, ptsCore.length);
    placeGlobe(dotsC, ptsCore, coreR, dummy, front, back, col);
    core.add(dotsC);
    gyre.add(core);

    const shell = new THREE.Group();
    const dotsS = new THREE.InstancedMesh(dotGeo, matShell, ptsShell.length);
    placeGlobe(dotsS, ptsShell, shellR, dummy, front, back, col);
    shell.add(dotsS);
    gyre.add(shell);

    const orbA = new THREE.InstancedMesh(dotGeo, matRing, ringA.length);
    placeCloud(orbA, ringA, dummy, front, back, col, 1.15);
    gyre.add(orbA);
    const orbB = new THREE.InstancedMesh(dotGeo, matRing, ringB.length);
    placeCloud(orbB, ringB, dummy, front, back, col, 1.05);
    gyre.add(orbB);

    const eA = new THREE.Euler(0.62, 0, 0.18, "XYZ");
    const eB = new THREE.Euler(-0.95, 0, 0.4, "XYZ");
    const beadsA = [0, 1, 2].map(function () {
      const m = new THREE.Mesh(beadGeo, matBead);
      gyre.add(m);
      return m;
    });
    const beadsB = [0, 1].map(function () {
      const m = new THREE.Mesh(beadGeo, matBead);
      gyre.add(m);
      return m;
    });

    scene.add(new THREE.AmbientLight(FOREST, 0.48));
    const hemi = new THREE.HemisphereLight(LIME, VOID, 0.7);
    scene.add(hemi);
    const key = new THREE.DirectionalLight(0xffffff, 0.95);
    key.position.set(-2.4, 4.4, 6.2);
    scene.add(key);
    const rim = new THREE.DirectionalLight(LIME, 0.62);
    rim.position.set(3.6, 0.4, -4.4);
    scene.add(rim);

    function applyQuality() {
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      resize();
    }
    function resize() {
      const w = canvas.clientWidth || window.innerWidth;
      const h = canvas.clientHeight || window.innerHeight;
      if (w < 2 || h < 2) return;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    }
    applyQuality();

    let mx = 0;
    let my = 0;
    let tx = 0;
    let ty = 0;
    function onMove(e) {
      tx = (e.clientX / (window.innerWidth || 1)) * 2 - 1;
      ty = (e.clientY / (window.innerHeight || 1)) * 2 - 1;
    }
    window.addEventListener("pointermove", onMove, { passive: true });
    window.addEventListener("resize", resize);

    function paintGlobe(mesh, pts, fHex, bHex) {
      front.setHex(fHex);
      back.setHex(bHex);
      for (let i = 0; i < pts.length; i++) {
        const facing = Math.max(0, pts[i].z);
        col.copy(back).lerp(front, 0.45 + facing * 0.55);
        mesh.setColorAt(i, col);
      }
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    }
    function paintCloud(mesh, pts, fHex, bHex) {
      front.setHex(fHex);
      back.setHex(bHex);
      for (let i = 0; i < pts.length; i++) {
        const p = pts[i];
        const facing = Math.max(0, p.z / (p.length() || 1));
        col.copy(back).lerp(front, 0.3 + facing * 0.7);
        mesh.setColorAt(i, col);
      }
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    }

    function applyTheme() {
      const hex = isLight() ? LIME_LIGHT : LIME;
      const dim = isLight() ? 0x6a8a00 : LIME_DIM;
      [mat, matShell, matRing, matBead].forEach(function (m) {
        m.color.setHex(hex);
        m.emissive.setHex(hex);
      });
      hemi.color.setHex(hex);
      rim.color.setHex(hex);
      paintGlobe(dotsC, ptsCore, hex, dim);
      paintGlobe(dotsS, ptsShell, hex, dim);
      paintCloud(orbA, ringA, hex, dim);
      paintCloud(orbB, ringB, hex, dim);
    }
    applyTheme();
    const themeObs = new MutationObserver(applyTheme);
    themeObs.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });

    const reduced =
      window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
    const t0 = performance.now();
    const tmp = new THREE.Vector3();

    function tick(now) {
      if (dead) return;
      mx += (tx - mx) * 0.035;
      my += (ty - my) * 0.035;
      const t = reduced ? 0 : (now - t0) * 0.001;
      core.rotation.y = t * 0.055;
      shell.rotation.y = -t * 0.028;
      gyre.rotation.z = Math.sin(t * 0.11) * 0.06;
      if (!reduced) {
        for (let i = 0; i < beadsA.length; i++) {
          const a = t * 0.55 + (i / beadsA.length) * Math.PI * 2;
          tmp.set(Math.cos(a) * 3.05, 0, Math.sin(a) * 3.05).applyEuler(eA);
          beadsA[i].position.copy(tmp);
        }
        for (let i = 0; i < beadsB.length; i++) {
          const a = -t * 0.42 + (i / beadsB.length) * Math.PI * 2;
          tmp.set(Math.cos(a) * 3.42, 0, Math.sin(a) * 3.42).applyEuler(eB);
          beadsB[i].position.copy(tmp);
        }
      }
      camera.position.set(mx * 0.34, 0.5 - my * 0.14, 11.6);
      camera.lookAt(mx * 0.1, 0.1 - my * 0.05, 0);
      renderer.render(scene, camera);
    }

    function onVis() {
      renderer.setAnimationLoop(!document.hidden && !dead ? tick : null);
    }
    document.addEventListener("visibilitychange", onVis);
    if (!reduced) renderer.setAnimationLoop(tick);
    else tick(performance.now());

    stopInner = function () {
      dead = true;
      renderer.setAnimationLoop(null);
      document.removeEventListener("visibilitychange", onVis);
      window.removeEventListener("resize", resize);
      window.removeEventListener("pointermove", onMove);
      themeObs.disconnect();
      renderer.dispose();
      mat.dispose();
      matShell.dispose();
      matRing.dispose();
      matBead.dispose();
      dotGeo.dispose();
      beadGeo.dispose();
      dotsC.dispose();
      dotsS.dispose();
      orbA.dispose();
      orbB.dispose();
    };
  }

  return function () {
    stopInner();
  };
}

function ringPts(n, radius, y) {
  const pts = [];
  for (let i = 0; i < n; i++) {
    const a = ((i + 0.17) / n) * Math.PI * 2;
    pts.push(new THREE.Vector3(Math.cos(a) * radius, y, Math.sin(a) * radius));
  }
  return pts;
}

function helixPts(turns, n, r0, r1, y0, y1) {
  const pts = [];
  for (let i = 0; i < n; i++) {
    const t = i / Math.max(1, n - 1);
    const a = t * turns * Math.PI * 2;
    const r = r0 + (r1 - r0) * t;
    const y = y0 + (y1 - y0) * t;
    pts.push(new THREE.Vector3(Math.cos(a) * r, y, Math.sin(a) * r));
  }
  return pts;
}

function helixAt(t, turns, r0, r1, y0, y1, out) {
  const a = t * turns * Math.PI * 2;
  const r = r0 + (r1 - r0) * t;
  const y = y0 + (y1 - y0) * t;
  return out.set(Math.cos(a) * r, y, Math.sin(a) * r);
}

/// Stacked lime-dot ranks + a climbing helix. vapurrbid — not a globe, not a gyre.
export function startVapurrbid(canvas) {
  if (!canvas) return function () {};

  let dead = false;
  let stopInner = function () {
    dead = true;
  };

  whenPaint(function () {
    boot().catch(function (err) {
      console.warn("vapurrbid scene", err);
    });
  });

  async function boot() {
    const renderer = new THREE.WebGPURenderer({
      canvas: canvas,
      antialias: true,
      alpha: true,
      powerPreference: "high-performance",
      stencil: false,
    });
    renderer.toneMapping = THREE.NoToneMapping;
    renderer.toneMappingExposure = 1;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.setClearColor(0x000000, 0);
    await renderer.init();
    if (dead) {
      renderer.dispose();
      return;
    }
    canvas.dataset.gpu = renderer.backend && renderer.backend.isWebGPUBackend ? "webgpu" : "webgl2";

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(32, 1, 0.4, 70);
    camera.position.set(0, 0.35, 11.8);
    camera.lookAt(0, 0.15, 0);

    const tower = new THREE.Group();
    tower.name = "vapurrbid.ladder";
    tower.position.set(0, -0.55, -0.7);
    tower.rotation.x = 0.18;
    scene.add(tower);

    const y0 = -2.45;
    const y1 = 2.72;
    const r0 = 3.92;
    const r1 = 0.38;
    const turns = 3.15;
    const levels = 9;
    const ringCloud = [];
    for (let i = 0; i < levels; i++) {
      const t = i / (levels - 1);
      const y = y0 + (y1 - y0) * t;
      const r = r0 + (r1 - r0) * t;
      const n = Math.max(12, Math.round(92 * (1 - t * 0.68)));
      Array.prototype.push.apply(ringCloud, ringPts(n, r, y));
    }
    const helix = helixPts(turns, 118, r0 * 0.92, r1 + 0.18, y0, y1);
    const floor = ringPts(108, r0 * 1.24, y0 - 0.38);

    const mat = new THREE.MeshStandardMaterial({
      color: LIME,
      emissive: LIME,
      emissiveIntensity: 0.18,
      roughness: 0.42,
      metalness: 0.12,
    });
    const matHelix = mat.clone();
    matHelix.emissiveIntensity = 0.55;
    const matFloor = mat.clone();
    matFloor.emissiveIntensity = 0.08;
    const matBead = mat.clone();
    matBead.emissiveIntensity = 1.08;
    const matApex = mat.clone();
    matApex.emissiveIntensity = 1.25;

    const dotGeo = new THREE.SphereGeometry(0.009, 10, 8);
    const beadGeo = new THREE.SphereGeometry(0.05, 12, 10);
    const apexGeo = new THREE.SphereGeometry(0.078, 14, 12);
    const dummy = new THREE.Object3D();
    const front = new THREE.Color(LIME);
    const back = new THREE.Color(LIME_DIM);
    const col = new THREE.Color();

    const rings = new THREE.InstancedMesh(dotGeo, mat, ringCloud.length);
    placeCloud(rings, ringCloud, dummy, front, back, col, 1.05);
    tower.add(rings);

    const climb = new THREE.InstancedMesh(dotGeo, matHelix, helix.length);
    placeCloud(climb, helix, dummy, front, back, col, 1.22);
    tower.add(climb);

    const floorMesh = new THREE.InstancedMesh(dotGeo, matFloor, floor.length);
    placeCloud(floorMesh, floor, dummy, front, back, col, 0.82);
    tower.add(floorMesh);

    const beads = [0, 1, 2].map(function () {
      const m = new THREE.Mesh(beadGeo, matBead);
      tower.add(m);
      return m;
    });
    const apex = new THREE.Mesh(apexGeo, matApex);
    apex.position.set(0, y1 + 0.22, 0);
    tower.add(apex);

    scene.add(new THREE.AmbientLight(FOREST, 0.48));
    const hemi = new THREE.HemisphereLight(LIME, VOID, 0.7);
    scene.add(hemi);
    const key = new THREE.DirectionalLight(0xffffff, 0.95);
    key.position.set(-2.2, 4.6, 6.2);
    scene.add(key);
    const rim = new THREE.DirectionalLight(LIME, 0.62);
    rim.position.set(3.4, 0.5, -4.4);
    scene.add(rim);

    function applyQuality() {
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      resize();
    }
    function resize() {
      const w = canvas.clientWidth || window.innerWidth;
      const h = canvas.clientHeight || window.innerHeight;
      if (w < 2 || h < 2) return;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    }
    applyQuality();

    let mx = 0;
    let my = 0;
    let tx = 0;
    let ty = 0;
    function onMove(e) {
      tx = (e.clientX / (window.innerWidth || 1)) * 2 - 1;
      ty = (e.clientY / (window.innerHeight || 1)) * 2 - 1;
    }
    window.addEventListener("pointermove", onMove, { passive: true });
    window.addEventListener("resize", resize);

    function paintCloudLocal(mesh, pts, fHex, bHex) {
      front.setHex(fHex);
      back.setHex(bHex);
      for (let i = 0; i < pts.length; i++) {
        const p = pts[i];
        const facing = Math.max(0, p.z / (p.length() || 1));
        col.copy(back).lerp(front, 0.3 + facing * 0.7);
        mesh.setColorAt(i, col);
      }
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    }

    function applyTheme() {
      const hex = isLight() ? LIME_LIGHT : LIME;
      const dim = isLight() ? 0x6a8a00 : LIME_DIM;
      [mat, matHelix, matFloor, matBead, matApex].forEach(function (m) {
        m.color.setHex(hex);
        m.emissive.setHex(hex);
      });
      hemi.color.setHex(hex);
      rim.color.setHex(hex);
      paintCloudLocal(rings, ringCloud, hex, dim);
      paintCloudLocal(climb, helix, hex, dim);
      paintCloudLocal(floorMesh, floor, hex, dim);
    }
    applyTheme();
    const themeObs = new MutationObserver(applyTheme);
    themeObs.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });

    const reduced =
      window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
    const t0 = performance.now();
    const tmp = new THREE.Vector3();

    function tick(now) {
      if (dead) return;
      mx += (tx - mx) * 0.035;
      my += (ty - my) * 0.035;
      const t = reduced ? 0 : (now - t0) * 0.001;
      tower.rotation.y = t * 0.055;
      apex.scale.setScalar(1 + Math.sin(t * 1.7) * 0.08);
      if (!reduced) {
        for (let i = 0; i < beads.length; i++) {
          const u = ((t * 0.12 + i / beads.length) % 1 + 1) % 1;
          helixAt(u, turns, r0 * 0.92, r1 + 0.18, y0, y1, tmp);
          beads[i].position.copy(tmp);
        }
      }
      camera.position.set(mx * 0.34, 0.35 - my * 0.14, 11.8);
      camera.lookAt(mx * 0.1, 0.15 - my * 0.05, 0);
      renderer.render(scene, camera);
    }

    function onVis() {
      renderer.setAnimationLoop(!document.hidden && !dead ? tick : null);
    }
    document.addEventListener("visibilitychange", onVis);
    if (!reduced) renderer.setAnimationLoop(tick);
    else tick(performance.now());

    stopInner = function () {
      dead = true;
      renderer.setAnimationLoop(null);
      document.removeEventListener("visibilitychange", onVis);
      window.removeEventListener("resize", resize);
      window.removeEventListener("pointermove", onMove);
      themeObs.disconnect();
      renderer.dispose();
      mat.dispose();
      matHelix.dispose();
      matFloor.dispose();
      matBead.dispose();
      matApex.dispose();
      dotGeo.dispose();
      beadGeo.dispose();
      apexGeo.dispose();
      rings.dispose();
      climb.dispose();
      floorMesh.dispose();
    };
  }

  return function () {
    stopInner();
  };
}

function torusPts(R, r, uN, vN) {
  const pts = [];
  for (let i = 0; i < uN; i++) {
    const u = ((i + 0.13) / uN) * Math.PI * 2;
    const nOn = Math.max(8, Math.round(vN * (0.72 + 0.28 * Math.abs(Math.cos(u)))));
    for (let j = 0; j < nOn; j++) {
      const v = ((j + 0.2) / nOn) * Math.PI * 2;
      const x = (R + r * Math.cos(v)) * Math.cos(u);
      const y = r * Math.sin(v);
      const z = (R + r * Math.cos(v)) * Math.sin(u);
      pts.push(new THREE.Vector3(x, y, z));
    }
  }
  return pts;
}

function torusAt(u, v, R, r, out) {
  const x = (R + r * Math.cos(v)) * Math.cos(u);
  const y = r * Math.sin(v);
  const z = (R + r * Math.cos(v)) * Math.sin(u);
  return out.set(x, y, z);
}

/// Lime-dot halo. PNS — a name ring, not a globe or a ladder.
export function startPns(canvas) {
  if (!canvas) return function () {};

  let dead = false;
  let stopInner = function () {
    dead = true;
  };

  whenPaint(function () {
    boot().catch(function (err) {
      console.warn("pns scene", err);
    });
  });

  async function boot() {
    const renderer = new THREE.WebGPURenderer({
      canvas: canvas,
      antialias: true,
      alpha: true,
      powerPreference: "high-performance",
      stencil: false,
    });
    renderer.toneMapping = THREE.NoToneMapping;
    renderer.toneMappingExposure = 1;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.setClearColor(0x000000, 0);
    await renderer.init();
    if (dead) {
      renderer.dispose();
      return;
    }
    canvas.dataset.gpu = renderer.backend && renderer.backend.isWebGPUBackend ? "webgpu" : "webgl2";

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(32, 1, 0.4, 70);
    camera.position.set(0, 0.4, 11.4);
    camera.lookAt(0, 0.1, 0);

    const halo = new THREE.Group();
    halo.name = "pns.halo";
    halo.position.set(0, -0.35, -0.6);
    halo.rotation.x = 0.72;
    halo.rotation.z = 0.18;
    scene.add(halo);

    const R = 3.35;
    const r = 0.92;
    const cloud = torusPts(R, r, 88, 16);

    const mat = new THREE.MeshStandardMaterial({
      color: LIME,
      emissive: LIME,
      emissiveIntensity: 0.2,
      roughness: 0.42,
      metalness: 0.12,
    });
    const matBead = mat.clone();
    matBead.emissiveIntensity = 1.12;

    const dotGeo = new THREE.SphereGeometry(0.009, 10, 8);
    const beadGeo = new THREE.SphereGeometry(0.048, 12, 10);
    const dummy = new THREE.Object3D();
    const front = new THREE.Color(LIME);
    const back = new THREE.Color(LIME_DIM);
    const col = new THREE.Color();

    const dots = new THREE.InstancedMesh(dotGeo, mat, cloud.length);
    placeCloud(dots, cloud, dummy, front, back, col, 1.08);
    halo.add(dots);

    const beads = [0, 1, 2].map(function () {
      const m = new THREE.Mesh(beadGeo, matBead);
      halo.add(m);
      return m;
    });

    scene.add(new THREE.AmbientLight(FOREST, 0.48));
    const hemi = new THREE.HemisphereLight(LIME, VOID, 0.7);
    scene.add(hemi);
    const key = new THREE.DirectionalLight(0xffffff, 0.95);
    key.position.set(-2.2, 4.4, 6.2);
    scene.add(key);
    const rim = new THREE.DirectionalLight(LIME, 0.58);
    rim.position.set(3.4, 0.5, -4.2);
    scene.add(rim);

    function applyQuality() {
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      resize();
    }
    function resize() {
      const w = canvas.clientWidth || window.innerWidth;
      const h = canvas.clientHeight || window.innerHeight;
      if (w < 2 || h < 2) return;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    }
    applyQuality();

    let mx = 0;
    let my = 0;
    let tx = 0;
    let ty = 0;
    function onMove(e) {
      tx = (e.clientX / (window.innerWidth || 1)) * 2 - 1;
      ty = (e.clientY / (window.innerHeight || 1)) * 2 - 1;
    }
    window.addEventListener("pointermove", onMove, { passive: true });
    window.addEventListener("resize", resize);

    function paintCloudLocal(mesh, pts, fHex, bHex) {
      front.setHex(fHex);
      back.setHex(bHex);
      for (let i = 0; i < pts.length; i++) {
        const p = pts[i];
        const facing = Math.max(0, p.z / (p.length() || 1));
        col.copy(back).lerp(front, 0.3 + facing * 0.7);
        mesh.setColorAt(i, col);
      }
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    }

    function applyTheme() {
      const hex = isLight() ? LIME_LIGHT : LIME;
      const dim = isLight() ? 0x6a8a00 : LIME_DIM;
      [mat, matBead].forEach(function (m) {
        m.color.setHex(hex);
        m.emissive.setHex(hex);
      });
      hemi.color.setHex(hex);
      rim.color.setHex(hex);
      paintCloudLocal(dots, cloud, hex, dim);
    }
    applyTheme();
    const themeObs = new MutationObserver(applyTheme);
    themeObs.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });

    const reduced =
      window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
    const t0 = performance.now();
    const tmp = new THREE.Vector3();

    function tick(now) {
      if (dead) return;
      mx += (tx - mx) * 0.035;
      my += (ty - my) * 0.035;
      const t = reduced ? 0 : (now - t0) * 0.001;
      halo.rotation.y = t * 0.07;
      if (!reduced) {
        for (let i = 0; i < beads.length; i++) {
          const u = t * 0.55 + (i / beads.length) * Math.PI * 2;
          const v = t * 1.15 + i * 2.1;
          torusAt(u, v, R, r, tmp);
          beads[i].position.copy(tmp);
        }
      }
      camera.position.set(mx * 0.34, 0.4 - my * 0.14, 11.4);
      camera.lookAt(mx * 0.1, 0.1 - my * 0.05, 0);
      renderer.render(scene, camera);
    }

    function onVis() {
      renderer.setAnimationLoop(!document.hidden && !dead ? tick : null);
    }
    document.addEventListener("visibilitychange", onVis);
    if (!reduced) renderer.setAnimationLoop(tick);
    else tick(performance.now());

    stopInner = function () {
      dead = true;
      renderer.setAnimationLoop(null);
      document.removeEventListener("visibilitychange", onVis);
      window.removeEventListener("resize", resize);
      window.removeEventListener("pointermove", onMove);
      themeObs.disconnect();
      renderer.dispose();
      mat.dispose();
      matBead.dispose();
      dotGeo.dispose();
      beadGeo.dispose();
      dots.dispose();
    };
  }

  return function () {
    stopInner();
  };
}

function doorPts(rx, ry, n, zJit) {
  const pts = [];
  for (let i = 0; i < n; i++) {
    const a = ((i + 0.17) / n) * Math.PI * 2;
    pts.push(
      new THREE.Vector3(
        Math.cos(a) * rx + (Math.random() - 0.5) * 0.012,
        Math.sin(a) * ry + (Math.random() - 0.5) * 0.012,
        (Math.random() - 0.5) * (zJit || 0.04)
      )
    );
  }
  return pts;
}

/// Standing lime-dot ring. A door, not a globe or a halo.
export function startLogin(canvas) {
  if (!canvas) return function () {};

  let dead = false;
  let stopInner = function () {
    dead = true;
  };

  whenPaint(function () {
    boot().catch(function (err) {
      console.warn("login scene", err);
    });
  });

  async function boot() {
    const renderer = new THREE.WebGPURenderer({
      canvas: canvas,
      antialias: true,
      alpha: true,
      powerPreference: "high-performance",
      stencil: false,
    });
    renderer.toneMapping = THREE.NoToneMapping;
    renderer.toneMappingExposure = 1;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.setClearColor(0x000000, 0);
    await renderer.init();
    if (dead) {
      renderer.dispose();
      return;
    }
    canvas.dataset.gpu = renderer.backend && renderer.backend.isWebGPUBackend ? "webgpu" : "webgl2";

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(32, 1, 0.4, 70);
    camera.position.set(0, 0.2, 12.2);
    camera.lookAt(0, 0.15, 0);

    const door = new THREE.Group();
    door.name = "login.door";
    door.position.set(0, -0.15, -0.5);
    scene.add(door);

    const outer = doorPts(3.55, 4.35, 168, 0.05);
    const inner = doorPts(2.55, 3.2, 128, 0.04);
    const sill = doorPts(1.35, 0.22, 48, 0.03).map(function (p) {
      p.y -= 3.55;
      return p;
    });
    const cloud = outer.concat(inner, sill);

    const mat = new THREE.MeshStandardMaterial({
      color: LIME,
      emissive: LIME,
      emissiveIntensity: 0.2,
      roughness: 0.42,
      metalness: 0.12,
    });
    const matBead = mat.clone();
    matBead.emissiveIntensity = 1.08;

    const dotGeo = new THREE.SphereGeometry(0.009, 10, 8);
    const beadGeo = new THREE.SphereGeometry(0.05, 12, 10);
    const dummy = new THREE.Object3D();
    const front = new THREE.Color(LIME);
    const back = new THREE.Color(LIME_DIM);
    const col = new THREE.Color();

    const dots = new THREE.InstancedMesh(dotGeo, mat, cloud.length);
    placeCloud(dots, cloud, dummy, front, back, col, 1.05);
    door.add(dots);

    const beads = [0, 1, 2].map(function () {
      const m = new THREE.Mesh(beadGeo, matBead);
      door.add(m);
      return m;
    });

    scene.add(new THREE.AmbientLight(FOREST, 0.48));
    const hemi = new THREE.HemisphereLight(LIME, VOID, 0.7);
    scene.add(hemi);
    const keyLight = new THREE.DirectionalLight(0xffffff, 0.95);
    keyLight.position.set(-2.2, 4.4, 6.2);
    scene.add(keyLight);
    const rim = new THREE.DirectionalLight(LIME, 0.58);
    rim.position.set(3.4, 0.5, -4.2);
    scene.add(rim);

    function applyQuality() {
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      resize();
    }
    function resize() {
      const w = canvas.clientWidth || window.innerWidth;
      const h = canvas.clientHeight || window.innerHeight;
      if (w < 2 || h < 2) return;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    }
    applyQuality();

    let mx = 0;
    let my = 0;
    let tx = 0;
    let ty = 0;
    function onMove(e) {
      tx = (e.clientX / (window.innerWidth || 1)) * 2 - 1;
      ty = (e.clientY / (window.innerHeight || 1)) * 2 - 1;
    }
    window.addEventListener("pointermove", onMove, { passive: true });
    window.addEventListener("resize", resize);

    function paintCloudLocal(mesh, pts, fHex, bHex) {
      front.setHex(fHex);
      back.setHex(bHex);
      for (let i = 0; i < pts.length; i++) {
        const p = pts[i];
        const facing = Math.max(0, p.z / (p.length() || 1));
        col.copy(back).lerp(front, 0.3 + facing * 0.7);
        mesh.setColorAt(i, col);
      }
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    }

    function applyTheme() {
      const hex = isLight() ? LIME_LIGHT : LIME;
      const dim = isLight() ? 0x6a8a00 : LIME_DIM;
      [mat, matBead].forEach(function (m) {
        m.color.setHex(hex);
        m.emissive.setHex(hex);
      });
      hemi.color.setHex(hex);
      rim.color.setHex(hex);
      paintCloudLocal(dots, cloud, hex, dim);
    }
    applyTheme();
    const themeObs = new MutationObserver(applyTheme);
    themeObs.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });

    const reduced =
      window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
    const t0 = performance.now();

    function tick(now) {
      if (dead) return;
      mx += (tx - mx) * 0.035;
      my += (ty - my) * 0.035;
      const t = reduced ? 0 : (now - t0) * 0.001;
      door.rotation.y = Math.sin(t * 0.22) * 0.12;
      if (!reduced) {
        for (let i = 0; i < beads.length; i++) {
          const a = t * 0.55 + (i / beads.length) * Math.PI * 2;
          beads[i].position.set(Math.cos(a) * 3.55, Math.sin(a) * 4.35, 0);
        }
      }
      camera.position.set(mx * 0.34, 0.2 - my * 0.14, 12.2);
      camera.lookAt(mx * 0.1, 0.15 - my * 0.05, 0);
      renderer.render(scene, camera);
    }

    function onVis() {
      renderer.setAnimationLoop(!document.hidden && !dead ? tick : null);
    }
    document.addEventListener("visibilitychange", onVis);
    if (!reduced) renderer.setAnimationLoop(tick);
    else tick(performance.now());

    stopInner = function () {
      dead = true;
      renderer.setAnimationLoop(null);
      document.removeEventListener("visibilitychange", onVis);
      window.removeEventListener("resize", resize);
      window.removeEventListener("pointermove", onMove);
      themeObs.disconnect();
      renderer.dispose();
      mat.dispose();
      matBead.dispose();
      dotGeo.dispose();
      beadGeo.dispose();
      dots.dispose();
    };
  }

  return function () {
    stopInner();
  };
}
