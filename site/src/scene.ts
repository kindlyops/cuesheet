// The hero diorama: a paper-craft autumn island with a low-poly red panda,
// animated at a stepped ~12 fps cadence for a stop-motion feel.

import {
  AmbientLight,
  Color,
  ConeGeometry,
  CylinderGeometry,
  DirectionalLight,
  DodecahedronGeometry,
  Group,
  HemisphereLight,
  InstancedMesh,
  Mesh,
  MeshLambertMaterial,
  Object3D,
  PerspectiveCamera,
  PlaneGeometry,
  Scene,
  SphereGeometry,
  WebGLRenderer,
  DoubleSide,
  Fog,
} from 'three';

const PALETTE = {
  cream: 0xf7eed7,
  fox: 0xc95b0c,
  mustard: 0xe3a72f,
  russet: 0x7a4419,
  ink: 0x33271c,
  cider: 0xefd9a7,
};

const STEP_FPS = 12; // stop-motion cadence

function paper(color: number): MeshLambertMaterial {
  return new MeshLambertMaterial({ color, flatShading: true });
}

// ---------------------------------------------------------------------------
// Red panda, built entirely from primitives.
// ---------------------------------------------------------------------------
interface Panda {
  group: Group;
  head: Group;
  tail: Group;
  tailSegments: Mesh[];
}

function buildRedPanda(): Panda {
  const rust = paper(PALETTE.fox);
  const cream = paper(PALETTE.cream);
  const dark = paper(0x47301d);
  const ink = paper(PALETTE.ink);

  const group = new Group();

  // Body: squat ellipsoid with a dark belly underneath.
  const body = new Mesh(new SphereGeometry(0.55, 9, 7), rust);
  body.scale.set(1.0, 0.85, 1.25);
  body.position.y = 0.55;
  group.add(body);

  const belly = new Mesh(new SphereGeometry(0.42, 9, 7), dark);
  belly.scale.set(0.95, 0.7, 1.15);
  belly.position.set(0, 0.38, 0.05);
  group.add(belly);

  // Head: charming and slightly oversized.
  const head = new Group();
  head.position.set(0, 1.18, 0.55);
  group.add(head);

  const skull = new Mesh(new SphereGeometry(0.42, 9, 7), rust);
  skull.scale.set(1.1, 0.95, 0.95);
  head.add(skull);

  // Cream face mask: muzzle + brow patches + cheeks.
  const muzzle = new Mesh(new SphereGeometry(0.2, 8, 6), cream);
  muzzle.scale.set(1.1, 0.8, 0.9);
  muzzle.position.set(0, -0.08, 0.33);
  head.add(muzzle);

  for (const side of [-1, 1]) {
    const brow = new Mesh(new SphereGeometry(0.09, 6, 5), cream);
    brow.position.set(0.16 * side, 0.22, 0.34);
    head.add(brow);

    const cheek = new Mesh(new SphereGeometry(0.13, 6, 5), cream);
    cheek.position.set(0.3 * side, -0.06, 0.22);
    head.add(cheek);

    // Ears: rust cones with cream tips.
    const ear = new Mesh(new ConeGeometry(0.14, 0.26, 5), rust);
    ear.position.set(0.3 * side, 0.4, -0.02);
    ear.rotation.z = -0.35 * side;
    head.add(ear);

    const earTip = new Mesh(new ConeGeometry(0.07, 0.12, 5), cream);
    earTip.position.set(0.34 * side, 0.5, -0.02);
    earTip.rotation.z = -0.35 * side;
    head.add(earTip);

    // Eyes.
    const eye = new Mesh(new SphereGeometry(0.045, 6, 5), ink);
    eye.position.set(0.15 * side, 0.08, 0.38);
    head.add(eye);

    // Tear-track stripes, the red panda signature.
    const stripe = new Mesh(new SphereGeometry(0.05, 5, 4), paper(0x8a4a16));
    stripe.scale.set(0.7, 1.8, 0.5);
    stripe.position.set(0.2 * side, -0.05, 0.36);
    head.add(stripe);
  }

  const nose = new Mesh(new SphereGeometry(0.06, 6, 5), ink);
  nose.position.set(0, -0.02, 0.52);
  head.add(nose);

  // Legs: short dark stubs.
  const legGeo = new CylinderGeometry(0.1, 0.12, 0.36, 6);
  const legPositions: Array<[number, number]> = [
    [-0.28, 0.42],
    [0.28, 0.42],
    [-0.3, -0.42],
    [0.3, -0.42],
  ];
  for (const [x, z] of legPositions) {
    const leg = new Mesh(legGeo, dark);
    leg.position.set(x, 0.18, z);
    group.add(leg);
  }

  // Long ringed tail: alternating rust/cream segments curving upward.
  const tail = new Group();
  tail.position.set(0, 0.62, -0.62);
  group.add(tail);

  const tailSegments: Mesh[] = [];
  const segs = 8;
  for (let i = 0; i < segs; i++) {
    const t = i / (segs - 1);
    const radius = 0.18 * (1 - t * 0.45);
    const mat = i % 2 === 0 ? rust : cream;
    const seg = new Mesh(new SphereGeometry(radius, 7, 5), mat);
    // Curve back and up.
    seg.position.set(0, Math.sin(t * 1.7) * 0.85, -t * 0.95);
    tail.add(seg);
    tailSegments.push(seg);
  }

  return { group, head, tail, tailSegments };
}

// ---------------------------------------------------------------------------
// Trees, island, leaves.
// ---------------------------------------------------------------------------
function buildTree(
  height: number,
  canopyColor: number,
  round: boolean,
): Group {
  const tree = new Group();
  const trunk = new Mesh(
    new CylinderGeometry(0.07, 0.1, height * 0.45, 5),
    paper(PALETTE.russet),
  );
  trunk.position.y = height * 0.225;
  tree.add(trunk);

  if (round) {
    const canopy = new Mesh(
      new DodecahedronGeometry(height * 0.34, 0),
      paper(canopyColor),
    );
    canopy.position.y = height * 0.62;
    tree.add(canopy);
  } else {
    for (let i = 0; i < 3; i++) {
      const r = height * (0.32 - i * 0.08);
      const cone = new Mesh(new ConeGeometry(r, height * 0.32, 6), paper(canopyColor));
      cone.position.y = height * (0.42 + i * 0.22);
      tree.add(cone);
    }
  }
  return tree;
}

function buildIsland(): Group {
  const island = new Group();

  // Floating rock base: an upside-down faceted cone.
  const base = new Mesh(new ConeGeometry(2.4, 2.2, 8), paper(PALETTE.russet));
  base.rotation.x = Math.PI;
  base.position.y = -1.1;
  island.add(base);

  // Grass top.
  const top = new Mesh(new CylinderGeometry(2.4, 2.45, 0.35, 8), paper(PALETTE.mustard));
  top.position.y = 0.0;
  island.add(top);

  const meadow = new Mesh(new CylinderGeometry(2.25, 2.4, 0.12, 8), paper(PALETTE.cider));
  meadow.position.y = 0.22;
  island.add(meadow);

  // Paper trees around the rim.
  const treeSpots: Array<[number, number, number, number, boolean]> = [
    [-1.5, -0.6, 2.4, PALETTE.fox, false],
    [1.6, -0.9, 1.9, PALETTE.mustard, true],
    [-0.6, -1.6, 1.6, PALETTE.fox, true],
    [1.1, 1.3, 1.5, PALETTE.russet, false],
    [-1.7, 0.9, 1.2, PALETTE.mustard, true],
  ];
  for (const [x, z, h, color, round] of treeSpots) {
    const tree = buildTree(h, color, round);
    tree.position.set(x, 0.25, z);
    tree.rotation.y = x * 2.1;
    island.add(tree);
  }

  // A few pebbles.
  for (let i = 0; i < 5; i++) {
    const pebble = new Mesh(new DodecahedronGeometry(0.1 + (i % 3) * 0.04, 0), paper(PALETTE.cream));
    const a = i * 1.9;
    pebble.position.set(Math.cos(a) * 1.9, 0.3, Math.sin(a) * 1.9);
    island.add(pebble);
  }

  return island;
}

interface LeafField {
  mesh: InstancedMesh;
  update(time: number): void;
}

function buildLeaves(count: number): LeafField {
  const geo = new PlaneGeometry(0.13, 0.13);
  const mat = new MeshLambertMaterial({ side: DoubleSide, flatShading: true });
  const mesh = new InstancedMesh(geo, mat, count);

  const colors = [PALETTE.fox, PALETTE.mustard, PALETTE.russet, PALETTE.cider];
  const color = new Color();
  const seeds: Array<{ x: number; z: number; phase: number; speed: number; spin: number }> = [];
  for (let i = 0; i < count; i++) {
    seeds.push({
      x: (Math.random() - 0.5) * 7,
      z: (Math.random() - 0.5) * 7,
      phase: Math.random() * 100,
      speed: 0.55 + Math.random() * 0.5,
      spin: Math.random() * Math.PI * 2,
    });
    mesh.setColorAt(i, color.setHex(colors[i % colors.length]));
  }
  if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;

  const dummy = new Object3D();
  const SPAN = 7; // vertical travel before wrapping

  function update(time: number): void {
    for (let i = 0; i < count; i++) {
      const s = seeds[i];
      const fall = (s.phase + time * s.speed) % SPAN;
      const y = 4.2 - fall;
      const sway = Math.sin(time * 0.9 + s.phase) * 0.5;
      dummy.position.set(s.x + sway, y, s.z + Math.cos(time * 0.7 + s.phase) * 0.3);
      dummy.rotation.set(
        time * 1.3 + s.spin,
        time * 0.9 + s.phase,
        s.spin,
      );
      dummy.updateMatrix();
      mesh.setMatrixAt(i, dummy.matrix);
    }
    mesh.instanceMatrix.needsUpdate = true;
  }

  update(0);
  return { mesh, update };
}

// ---------------------------------------------------------------------------
// Scene assembly + loop.
// ---------------------------------------------------------------------------
export function startDiorama(container: HTMLElement): void {
  const reducedMotion = window.matchMedia(
    '(prefers-reduced-motion: reduce)',
  ).matches;

  const renderer = new WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.setSize(container.clientWidth, container.clientHeight);
  container.prepend(renderer.domElement);

  const scene = new Scene();
  scene.background = new Color(PALETTE.cream);
  scene.fog = new Fog(PALETTE.cream, 9, 17);

  const camera = new PerspectiveCamera(
    42,
    container.clientWidth / container.clientHeight,
    0.1,
    50,
  );
  camera.position.set(0, 2.2, 8.2);
  camera.lookAt(0, 0.6, 0);

  // Warm, soft lighting.
  scene.add(new HemisphereLight(PALETTE.cider, PALETTE.russet, 0.9));
  scene.add(new AmbientLight(PALETTE.cream, 0.35));
  const key = new DirectionalLight(0xffe2b8, 1.6);
  key.position.set(4, 6, 5);
  scene.add(key);
  const rim = new DirectionalLight(PALETTE.fox, 0.7);
  rim.position.set(-5, 3, -4);
  scene.add(rim);

  // Diorama group (everything that parallax-rotates together).
  const diorama = new Group();
  scene.add(diorama);

  const island = buildIsland();
  diorama.add(island);

  const panda = buildRedPanda();
  panda.group.position.set(0.15, 0.28, 0.35);
  panda.group.rotation.y = -0.35;
  panda.group.scale.setScalar(0.95);
  diorama.add(panda.group);

  const leaves = buildLeaves(reducedMotion ? 0 : 90);
  if (!reducedMotion) diorama.add(leaves.mesh);

  // ------------------------------------------------------------------
  // Animation: quantized to ~12 fps steps for the stop-motion feel.
  // ------------------------------------------------------------------
  function animateStep(qt: number): void {
    // Island bobbing.
    diorama.position.y = Math.sin(qt * 0.7) * 0.12;

    // Panda body bob + head tilt.
    panda.group.position.y = 0.28 + Math.sin(qt * 1.4) * 0.04;
    panda.head.rotation.z = Math.sin(qt * 0.6) * 0.08;
    panda.head.rotation.y = Math.sin(qt * 0.35) * 0.18;

    // Tail sway: whole tail waves, segments ripple.
    panda.tail.rotation.x = -0.15 + Math.sin(qt * 1.1) * 0.12;
    panda.tail.rotation.z = Math.sin(qt * 0.8) * 0.22;
    for (let i = 0; i < panda.tailSegments.length; i++) {
      const seg = panda.tailSegments[i];
      const t = i / (panda.tailSegments.length - 1);
      seg.position.x = Math.sin(qt * 1.6 + t * 2.2) * 0.12 * t;
    }

    leaves.update(qt);
  }

  // ------------------------------------------------------------------
  // Parallax from mouse + scroll (smooth, not quantized).
  // ------------------------------------------------------------------
  let targetRotY = 0;
  let targetRotX = 0;
  let scrollTilt = 0;

  if (!reducedMotion) {
    window.addEventListener('pointermove', (ev) => {
      targetRotY = (ev.clientX / window.innerWidth - 0.5) * 0.3;
      targetRotX = (ev.clientY / window.innerHeight - 0.5) * 0.15;
    });
    window.addEventListener(
      'scroll',
      () => {
        scrollTilt = Math.min(window.scrollY / window.innerHeight, 1) * 0.35;
      },
      { passive: true },
    );
  }

  // ------------------------------------------------------------------
  // Render loop with visibility / in-view pausing.
  // ------------------------------------------------------------------
  let rafId = 0;
  let running = false;
  let lastQt = -1;
  const start = performance.now();

  function frame(): void {
    rafId = requestAnimationFrame(frame);
    const elapsed = (performance.now() - start) / 1000;
    const qt = Math.floor(elapsed * STEP_FPS) / STEP_FPS;
    if (qt !== lastQt) {
      lastQt = qt;
      animateStep(qt);
    }
    // Smooth parallax easing.
    diorama.rotation.y += (targetRotY - diorama.rotation.y) * 0.06;
    diorama.rotation.x += (targetRotX + scrollTilt - diorama.rotation.x) * 0.06;
    renderer.render(scene, camera);
  }

  function play(): void {
    if (running || reducedMotion) return;
    running = true;
    rafId = requestAnimationFrame(frame);
  }

  function pause(): void {
    if (!running) return;
    running = false;
    cancelAnimationFrame(rafId);
  }

  if (reducedMotion) {
    // Static scene: a single render, no parallax, no loop.
    animateStep(0.4);
    renderer.render(scene, camera);
  } else {
    play();
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) pause();
      else play();
    });
    // Also pause when the hero is scrolled out of view.
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting && !document.hidden) play();
        else pause();
      }
    });
    observer.observe(container);
  }

  window.addEventListener('resize', () => {
    const w = container.clientWidth;
    const h = container.clientHeight;
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    renderer.setSize(w, h);
    if (reducedMotion) renderer.render(scene, camera);
  });
}
