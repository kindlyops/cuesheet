// The easter egg: the page "falls away like sand" (a particle snapshot of the
// DOM drains downward), revealing a levitating museum-piece sculpture of the
// app's hexagonal architecture. Lazy-loaded; costs nothing until triggered.

import html2canvas from 'html2canvas';
import {
  AmbientLight,
  BoxGeometry,
  BufferAttribute,
  BufferGeometry,
  CanvasTexture,
  Color,
  CylinderGeometry,
  DirectionalLight,
  EdgesGeometry,
  Group,
  Line,
  LineBasicMaterial,
  LineSegments,
  Mesh,
  MeshPhongMaterial,
  OrthographicCamera,
  PerspectiveCamera,
  PlaneGeometry,
  Points,
  PointsMaterial,
  Scene,
  Sprite,
  SpriteMaterial,
  SRGBColorSpace,
  Vector3,
  WebGLRenderer,
  DoubleSide,
} from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

const INK = 0x33271c;
const CREAM = 0xf7eed7;
const FOX = 0xc95b0c;
const MUSTARD = 0xe3a72f;
const RUSSET = 0x7a4419;
const CIDER = 0xefd9a7;

const SAMPLE_STEP = 6; // sample the snapshot every ~6 px
const SAND_DURATION = 4.2; // seconds until fully drained

export interface EggController {
  dismiss(): void;
}

// ---------------------------------------------------------------------------
// Label sprites drawn onto canvas textures.
// ---------------------------------------------------------------------------
function makeLabelSprite(text: string, options?: { small?: boolean; color?: string }): Sprite {
  const small = options?.small ?? false;
  const fontPx = small ? 26 : 40;
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d')!;
  ctx.font = `italic ${fontPx}px Palatino, Georgia, serif`;
  const metrics = ctx.measureText(text);
  canvas.width = Math.ceil(metrics.width) + 24;
  canvas.height = fontPx + 20;
  // Font resets after resize.
  ctx.font = `italic ${fontPx}px Palatino, Georgia, serif`;
  ctx.textBaseline = 'middle';
  ctx.textAlign = 'center';
  ctx.fillStyle = options?.color ?? '#EFD9A7';
  ctx.fillText(text, canvas.width / 2, canvas.height / 2);

  const texture = new CanvasTexture(canvas);
  texture.colorSpace = SRGBColorSpace;
  const sprite = new Sprite(
    new SpriteMaterial({ map: texture, transparent: true, depthWrite: false }),
  );
  const scale = small ? 0.011 : 0.013;
  sprite.scale.set(canvas.width * scale, canvas.height * scale, 1);
  return sprite;
}

// ---------------------------------------------------------------------------
// Sand: a particle grid sampled from an html2canvas snapshot of the page.
// ---------------------------------------------------------------------------
interface SandField {
  points: Points;
  /** Advance the simulation; returns true while still draining. */
  step(dt: number, elapsed: number): boolean;
}

function buildSand(snapshot: HTMLCanvasElement, vw: number, vh: number): SandField {
  const ctx = snapshot.getContext('2d', { willReadFrequently: true })!;
  const image = ctx.getImageData(0, 0, snapshot.width, snapshot.height);
  const scaleX = snapshot.width / vw;
  const scaleY = snapshot.height / vh;

  const cols = Math.floor(vw / SAMPLE_STEP);
  const rows = Math.floor(vh / SAMPLE_STEP);
  const count = cols * rows;

  const positions = new Float32Array(count * 3);
  const colors = new Float32Array(count * 3);
  const velocities = new Float32Array(count * 2); // vx, vy per particle
  const delays = new Float32Array(count);

  let i = 0;
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const px = c * SAMPLE_STEP + SAMPLE_STEP / 2;
      const py = r * SAMPLE_STEP + SAMPLE_STEP / 2;
      const sx = Math.min(Math.floor(px * scaleX), snapshot.width - 1);
      const sy = Math.min(Math.floor(py * scaleY), snapshot.height - 1);
      const o = (sy * snapshot.width + sx) * 4;

      positions[i * 3] = px;
      positions[i * 3 + 1] = -py;
      positions[i * 3 + 2] = 0;
      colors[i * 3] = image.data[o] / 255;
      colors[i * 3 + 1] = image.data[o + 1] / 255;
      colors[i * 3 + 2] = image.data[o + 2] / 255;

      // Sand drains from the bottom up, with grain-level jitter.
      const fromBottom = 1 - py / vh;
      delays[i] = fromBottom * 1.6 + Math.random() * 0.7;
      i++;
    }
  }

  const geometry = new BufferGeometry();
  geometry.setAttribute('position', new BufferAttribute(positions, 3));
  geometry.setAttribute('color', new BufferAttribute(colors, 3));

  const material = new PointsMaterial({
    size: SAMPLE_STEP,
    sizeAttenuation: false,
    vertexColors: true,
  });
  const points = new Points(geometry, material);

  function step(dt: number, elapsed: number): boolean {
    const pos = geometry.getAttribute('position') as BufferAttribute;
    let alive = false;
    for (let p = 0; p < count; p++) {
      const t = elapsed - delays[p];
      if (t <= 0) {
        alive = true;
        continue;
      }
      let x = pos.getX(p);
      let y = pos.getY(p);
      if (y < -vh - 40) continue; // already drained off-screen

      // Gravity + cheap curl-ish noise for that swirling-sand look.
      velocities[p * 2 + 1] -= 2400 * dt;
      const swirl =
        Math.sin(y * 0.013 + p * 0.37) * 140 +
        Math.cos(x * 0.011 + elapsed * 2.1) * 90;
      velocities[p * 2] += swirl * dt;

      x += velocities[p * 2] * dt;
      y += velocities[p * 2 + 1] * dt;
      pos.setXY(p, x, y);
      if (y >= -vh - 40) alive = true;
    }
    pos.needsUpdate = true;
    return alive && elapsed < SAND_DURATION + 1.5;
  }

  return { points, step };
}

// ---------------------------------------------------------------------------
// The sculpture: hexagonal architecture as a museum piece.
// ---------------------------------------------------------------------------
interface Sculpture {
  scene: Scene;
  camera: PerspectiveCamera;
  controls: OrbitControls;
  update(time: number): void;
  dispose(): void;
}

interface Satellite {
  pivot: Group;
  body: Group;
  line: Line;
  anchor: Vector3; // where the connector touches the hexagon (local space)
  basePos: Vector3;
  phase: number;
}

function buildSatelliteBody(kind: 'pdf' | 'zip' | 'gui' | 'cli'): Group {
  const g = new Group();
  switch (kind) {
    case 'pdf': {
      // A paper-like plane with a faint fold line.
      const sheet = new Mesh(
        new PlaneGeometry(0.9, 1.2),
        new MeshPhongMaterial({ color: CREAM, side: DoubleSide }),
      );
      sheet.rotation.y = 0.4;
      g.add(sheet);
      const rule = new Mesh(
        new BoxGeometry(0.6, 0.025, 0.01),
        new MeshPhongMaterial({ color: FOX }),
      );
      rule.position.set(0, 0.35, 0.02);
      rule.rotation.y = 0.4;
      g.add(rule);
      break;
    }
    case 'zip': {
      const box = new Mesh(
        new BoxGeometry(0.7, 0.7, 0.7),
        new MeshPhongMaterial({ color: MUSTARD }),
      );
      g.add(box);
      const strap = new Mesh(
        new BoxGeometry(0.74, 0.12, 0.74),
        new MeshPhongMaterial({ color: RUSSET }),
      );
      g.add(strap);
      break;
    }
    case 'gui': {
      // A rounded-feeling screen: slim bezel + glowing face.
      const bezel = new Mesh(
        new BoxGeometry(1.1, 0.78, 0.08),
        new MeshPhongMaterial({ color: RUSSET }),
      );
      g.add(bezel);
      const face = new Mesh(
        new PlaneGeometry(0.98, 0.64),
        new MeshPhongMaterial({ color: CREAM, emissive: new Color(0x40331f) }),
      );
      face.position.z = 0.045;
      g.add(face);
      const dot = new Mesh(
        new CylinderGeometry(0.07, 0.07, 0.02, 12),
        new MeshPhongMaterial({ color: FOX }),
      );
      dot.rotation.x = Math.PI / 2;
      dot.position.set(0, 0, 0.06);
      g.add(dot);
      break;
    }
    case 'cli': {
      const slab = new Mesh(
        new BoxGeometry(1.0, 0.62, 0.1),
        new MeshPhongMaterial({ color: 0x241a11 }),
      );
      g.add(slab);
      const prompt = new Mesh(
        new BoxGeometry(0.34, 0.05, 0.012),
        new MeshPhongMaterial({ color: CIDER, emissive: new Color(0x6b5a32) }),
      );
      prompt.position.set(-0.22, 0.12, 0.06);
      g.add(prompt);
      break;
    }
  }
  return g;
}

function buildSculpture(renderer: WebGLRenderer): Sculpture {
  const scene = new Scene();
  scene.background = new Color(INK);

  const camera = new PerspectiveCamera(
    45,
    window.innerWidth / window.innerHeight,
    0.1,
    60,
  );
  camera.position.set(0, 1.6, 8.5);

  // Museum lighting: dim ambient + warm rim lights.
  scene.add(new AmbientLight(0xfff3dd, 0.18));
  const rimA = new DirectionalLight(0xffc98a, 1.6);
  rimA.position.set(-6, 4, -3);
  scene.add(rimA);
  const rimB = new DirectionalLight(0xe3a72f, 0.9);
  rimB.position.set(6, -2, -4);
  scene.add(rimB);
  const fill = new DirectionalLight(0xf7eed7, 0.5);
  fill.position.set(0, 3, 8);
  scene.add(fill);

  const sculpture = new Group();
  scene.add(sculpture);

  // Core: translucent hexagonal prism.
  const hexRadius = 1.5;
  const core = new Mesh(
    new CylinderGeometry(hexRadius, hexRadius, 1.1, 6),
    new MeshPhongMaterial({
      color: FOX,
      transparent: true,
      opacity: 0.35,
      shininess: 80,
      specular: new Color(0xffe9c2),
    }),
  );
  sculpture.add(core);
  const edges = new LineSegments(
    new EdgesGeometry(core.geometry),
    new LineBasicMaterial({ color: CIDER, transparent: true, opacity: 0.8 }),
  );
  core.add(edges);

  const coreLabel = makeLabelSprite('cuesheet-core');
  coreLabel.position.set(0, 1.15, 0);
  sculpture.add(coreLabel);

  // Satellites at hexagon edge midpoints (flat faces of the prism).
  const satDefs: Array<{
    kind: 'pdf' | 'zip' | 'gui' | 'cli';
    label: string;
    port: string;
    angle: number;
  }> = [
    { kind: 'gui', label: 'Tauri GUI', port: 'driving · command', angle: 30 },
    { kind: 'cli', label: 'CLI', port: 'driving · command', angle: 150 },
    { kind: 'zip', label: 'Playlist ZIP', port: 'PlaylistSource', angle: 270 },
    { kind: 'pdf', label: 'Typst PDF', port: 'PdfCompiler', angle: 330 },
  ];

  const lineMat = new LineBasicMaterial({
    color: CIDER,
    transparent: true,
    opacity: 0.55,
  });

  // For a hexagon cylinder in three.js, flat-face normals sit at odd 30° angles;
  // we place anchors on the prism's outer wall at each satellite's bearing.
  const satellites: Satellite[] = satDefs.map((def, idx) => {
    const a = (def.angle * Math.PI) / 180;
    const dir = new Vector3(Math.cos(a), 0, Math.sin(a));
    const anchor = dir.clone().multiplyScalar(hexRadius * 0.92);
    const basePos = dir.clone().multiplyScalar(3.4);
    basePos.y = idx % 2 === 0 ? 0.5 : -0.4;

    const pivot = new Group();
    sculpture.add(pivot);

    const body = buildSatelliteBody(def.kind);
    body.position.copy(basePos);
    body.lookAt(0, basePos.y, 0);
    pivot.add(body);

    const label = makeLabelSprite(def.label);
    label.position.set(basePos.x, basePos.y + 0.85, basePos.z);
    pivot.add(label);

    // Port label where the connector touches the hexagon.
    const portLabel = makeLabelSprite(def.port, { small: true, color: '#E3A72F' });
    const portPos = dir.clone().multiplyScalar(hexRadius + 0.32);
    portLabel.position.set(portPos.x, anchor.y + 0.28, portPos.z);
    pivot.add(portLabel);

    // Connector line, updated every frame as the satellite bobs.
    const lineGeo = new BufferGeometry().setFromPoints([anchor, basePos]);
    const line = new Line(lineGeo, lineMat);
    pivot.add(line);

    return { pivot, body, line, anchor, basePos, phase: idx * 1.7 };
  });

  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  controls.dampingFactor = 0.06;
  controls.enablePan = false;
  controls.minDistance = 4;
  controls.maxDistance = 16;
  controls.autoRotate = true;
  controls.autoRotateSpeed = 0.6;
  controls.target.set(0, 0, 0);

  function update(time: number): void {
    // The whole piece levitates.
    sculpture.position.y = Math.sin(time * 0.5) * 0.18;
    core.rotation.y = time * 0.12; // edges are a child of core and rotate with it

    for (const sat of satellites) {
      const bobY = sat.basePos.y + Math.sin(time * 0.8 + sat.phase) * 0.16;
      sat.body.position.y = bobY;
      // Refresh the connector endpoints.
      const posAttr = sat.line.geometry.getAttribute('position') as BufferAttribute;
      posAttr.setXYZ(0, sat.anchor.x, sat.anchor.y, sat.anchor.z);
      posAttr.setXYZ(1, sat.basePos.x, bobY, sat.basePos.z);
      posAttr.needsUpdate = true;
    }
    controls.update();
  }

  function dispose(): void {
    controls.dispose();
    scene.traverse((obj) => {
      const mesh = obj as Mesh;
      if (mesh.geometry) mesh.geometry.dispose();
      const mat = (mesh as Mesh).material;
      if (Array.isArray(mat)) mat.forEach((m) => m.dispose());
      else if (mat) mat.dispose();
    });
  }

  return { scene, camera, controls, update, dispose };
}

// ---------------------------------------------------------------------------
// Orchestration.
// ---------------------------------------------------------------------------
export async function runEasterEgg(onDone: () => void): Promise<EggController> {
  const reducedMotion = window.matchMedia(
    '(prefers-reduced-motion: reduce)',
  ).matches;

  const vw = window.innerWidth;
  const vh = window.innerHeight;

  // Snapshot the current viewport before anything changes.
  let snapshot: HTMLCanvasElement | null = null;
  if (!reducedMotion) {
    try {
      snapshot = await html2canvas(document.body, {
        x: window.scrollX,
        y: window.scrollY,
        width: vw,
        height: vh,
        scale: 1,
        backgroundColor: '#F7EED7',
        logging: false,
      });
    } catch {
      snapshot = null; // fall through: skip the sand, go straight to the sculpture
    }
  }

  const canvas = document.createElement('canvas');
  canvas.id = 'egg-canvas';
  document.body.appendChild(canvas);
  document.body.classList.add('egg-active');

  const renderer = new WebGLRenderer({ canvas, antialias: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.setSize(vw, vh);

  const hint = document.createElement('p');
  hint.id = 'egg-hint';
  hint.textContent =
    'the hexagonal architecture of cuesheet — drag to orbit, tap here or Esc to return';
  // Tapping the hint returns to the page — the only dismissal touch devices
  // have, since there is no Esc key.
  hint.addEventListener('click', () => dismiss());
  document.body.appendChild(hint);

  const page = document.getElementById('page');

  // Phase 1: sand. An orthographic camera maps particles 1:1 to CSS pixels.
  let sand: SandField | null = null;
  let sandScene: Scene | null = null;
  let sandCamera: OrthographicCamera | null = null;

  if (snapshot) {
    sand = buildSand(snapshot, vw, vh);
    sandScene = new Scene();
    sandScene.add(sand.points);
    sandCamera = new OrthographicCamera(0, vw, 0, -vh, -10, 10);
    // Let the real DOM fade out underneath while the sand falls.
    window.setTimeout(() => page?.classList.add('draining'), 350);
  } else {
    page?.classList.add('draining');
  }

  // Phase 2: the sculpture (built up-front; it is cheap).
  const sculpture = buildSculpture(renderer);
  if (reducedMotion) sculpture.controls.autoRotate = false;

  let phase: 'sand' | 'sculpture' = sand ? 'sand' : 'sculpture';
  if (phase === 'sculpture') hint.classList.add('visible');

  let rafId = 0;
  let disposed = false;
  let last = performance.now();
  const start = last;

  function frame(now: number): void {
    if (disposed) return;
    rafId = requestAnimationFrame(frame);
    const dt = Math.min((now - last) / 1000, 0.05);
    last = now;
    const elapsed = (now - start) / 1000;

    if (phase === 'sand' && sand && sandScene && sandCamera) {
      const stillFalling = sand.step(dt, elapsed);
      // Cross-fade: render the sculpture behind once most sand is gone.
      if (elapsed > SAND_DURATION * 0.55) {
        sculpture.update(elapsed);
        renderer.autoClear = true;
        renderer.render(sculpture.scene, sculpture.camera);
        renderer.autoClear = false;
        renderer.render(sandScene, sandCamera);
        renderer.autoClear = true;
      } else {
        renderer.render(sandScene, sandCamera);
      }
      if (!stillFalling) {
        phase = 'sculpture';
        hint.classList.add('visible');
      }
    } else {
      sculpture.update(elapsed);
      renderer.render(sculpture.scene, sculpture.camera);
    }
  }
  rafId = requestAnimationFrame(frame);

  function onVisibility(): void {
    if (disposed) return;
    if (document.hidden) {
      cancelAnimationFrame(rafId);
    } else {
      last = performance.now();
      rafId = requestAnimationFrame(frame);
    }
  }
  document.addEventListener('visibilitychange', onVisibility);

  function onResize(): void {
    if (disposed) return;
    renderer.setSize(window.innerWidth, window.innerHeight);
    sculpture.camera.aspect = window.innerWidth / window.innerHeight;
    sculpture.camera.updateProjectionMatrix();
  }
  window.addEventListener('resize', onResize);

  function dismiss(): void {
    if (disposed) return;
    disposed = true;
    cancelAnimationFrame(rafId);
    document.removeEventListener('visibilitychange', onVisibility);
    window.removeEventListener('resize', onResize);

    // Quick fade back to the page.
    canvas.style.transition = 'opacity 0.45s ease';
    canvas.style.opacity = '0';
    hint.classList.remove('visible');
    page?.classList.remove('draining');
    window.setTimeout(() => {
      sculpture.dispose();
      sand?.points.geometry.dispose();
      (sand?.points.material as PointsMaterial | undefined)?.dispose();
      renderer.dispose();
      canvas.remove();
      hint.remove();
      document.body.classList.remove('egg-active');
      onDone();
    }, 470);
  }

  return { dismiss };
}
