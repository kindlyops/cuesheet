/// <reference types="vite/client" />
// "Use it right here": the browser version of Cuesheet. Lazy-loads the
// wasm-pack bundle from public/wasm/ the first time the drop zone is used,
// generates the PDF client-side, and triggers a download. The wasm bundle is
// produced by `wasm-pack build crates/cuesheet-wasm` (see pages.yml / justfile)
// and is absent in plain dev builds — the UI degrades gracefully.

type WasmModule = {
  default: (input?: { module_or_path?: string } | string) => Promise<unknown>;
  generate: (bytes: Uint8Array) => {
    playlistName: string;
    suggestedFilename: string;
    pdf: Uint8Array;
  };
};

let wasmModule: WasmModule | null = null;

async function loadWasm(): Promise<WasmModule> {
  if (wasmModule) return wasmModule;
  const base = import.meta.env.BASE_URL;
  const jsUrl = `${base}wasm/cuesheet_wasm.js`;
  // Probe first so a missing bundle yields a friendly message, not a thrown
  // module-resolution error in the console.
  const probe = await fetch(jsUrl, { method: 'HEAD' });
  if (!probe.ok) {
    throw new Error('the browser engine is not included in this build');
  }
  const mod = (await import(/* @vite-ignore */ jsUrl)) as WasmModule;
  await mod.default(`${base}wasm/cuesheet_wasm_bg.wasm`);
  wasmModule = mod;
  return mod;
}

function setStatus(zone: HTMLElement, text: string, kind: 'busy' | 'done' | 'error' | '') {
  const prompt = zone.querySelector<HTMLElement>('.webapp-prompt')!;
  const status = zone.querySelector<HTMLElement>('.webapp-status')!;
  if (!text) {
    prompt.hidden = false;
    status.hidden = true;
  } else {
    prompt.hidden = true;
    status.hidden = false;
    status.textContent = text;
  }
  zone.dataset.state = kind;
}

async function handleFile(zone: HTMLElement, file: File) {
  if (zone.dataset.state === 'busy') return;
  setStatus(zone, `Typesetting “${file.name}”…`, 'busy');
  try {
    const mod = await loadWasm();
    const bytes = new Uint8Array(await file.arrayBuffer());
    const generated = mod.generate(bytes);
    const blob = new Blob([generated.pdf.slice().buffer], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = generated.suggestedFilename;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 30_000);
    setStatus(zone, `Saved ${generated.suggestedFilename} — drop another?`, 'done');
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    setStatus(zone, `Sorry — ${message}`, 'error');
  }
}

export function initWebapp() {
  const zone = document.getElementById('webapp');
  const input = document.getElementById('webapp-file') as HTMLInputElement | null;
  if (!zone || !input) return;

  zone.addEventListener('click', () => input.click());
  zone.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      input.click();
    }
  });
  input.addEventListener('change', () => {
    const file = input.files?.[0];
    if (file) void handleFile(zone, file);
    input.value = '';
  });

  zone.addEventListener('dragover', (e) => {
    e.preventDefault();
    zone.classList.add('webapp-hover');
  });
  zone.addEventListener('dragleave', () => zone.classList.remove('webapp-hover'));
  zone.addEventListener('drop', (e) => {
    e.preventDefault();
    zone.classList.remove('webapp-hover');
    const file = e.dataTransfer?.files?.[0];
    if (file) void handleFile(zone, file);
  });
}
