import './style.css';
import { wireDownloadButtons } from './downloads';

// ---------------------------------------------------------------------------
// Download buttons: resolve latest release assets right away (cheap fetch).
// ---------------------------------------------------------------------------
wireDownloadButtons();

// ---------------------------------------------------------------------------
// In-browser app: the wasm-compiled core behind the drop zone.
// ---------------------------------------------------------------------------
import('./webapp').then(({ initWebapp }) => initWebapp());

// ---------------------------------------------------------------------------
// Hero scene: lazy-init after first paint so the page is interactive first.
// ---------------------------------------------------------------------------
function webglAvailable(): boolean {
  try {
    const canvas = document.createElement('canvas');
    return Boolean(
      window.WebGLRenderingContext &&
        (canvas.getContext('webgl2') || canvas.getContext('webgl')),
    );
  } catch {
    return false;
  }
}

function initHero(): void {
  const hero = document.getElementById('hero');
  if (!hero) return;
  if (!webglAvailable()) {
    const fallback = document.getElementById('hero-fallback');
    if (fallback) fallback.hidden = false;
    return;
  }
  import('./scene')
    .then(({ startDiorama }) => startDiorama(hero))
    .catch(() => {
      const fallback = document.getElementById('hero-fallback');
      if (fallback) fallback.hidden = false;
    });
}

// Wait for first paint, then init during idle time.
requestAnimationFrame(() => {
  if (typeof window.requestIdleCallback === 'function') {
    window.requestIdleCallback(initHero, { timeout: 1500 });
  } else {
    setTimeout(initHero, 50);
  }
});

// ---------------------------------------------------------------------------
// Easter egg trigger: type "plt" or press Shift+H. Module is lazy-loaded.
// ---------------------------------------------------------------------------
let eggBusy = false;
let eggController: { dismiss(): void } | null = null;
let typed = '';

async function triggerEgg(): Promise<void> {
  if (eggBusy) return;
  if (eggController) {
    eggController.dismiss();
    return;
  }
  eggBusy = true;
  try {
    const { runEasterEgg } = await import('./easterEgg');
    eggController = await runEasterEgg(() => {
      eggController = null;
    });
  } catch (err) {
    console.warn('easter egg failed to load', err);
  } finally {
    eggBusy = false;
  }
}

window.addEventListener('keydown', (ev: KeyboardEvent) => {
  const target = ev.target as HTMLElement | null;
  if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) {
    return;
  }

  if (ev.key === 'Escape' && eggController) {
    eggController.dismiss();
    return;
  }

  if (ev.shiftKey && (ev.key === 'H' || ev.key === 'h')) {
    void triggerEgg();
    typed = '';
    return;
  }

  if (ev.key.length === 1) {
    typed = (typed + ev.key.toLowerCase()).slice(-3);
    if (typed === 'plt') {
      typed = '';
      void triggerEgg();
    }
  }
});
