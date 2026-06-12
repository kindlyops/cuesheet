<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { createMachine, type State } from "./lib/machine";
  import { tauriDeps } from "./lib/tauri-deps";

  const machine = createMachine(tauriDeps);

  let ui = $state<State>({ kind: "idle" });
  let dropActive = $state(false);
  let detailsOpen = $state(false);

  machine.subscribe((s) => {
    ui = s;
    if (s.kind !== "error") detailsOpen = false;
  });

  onMount(() => {
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          dropActive = true;
        } else if (event.payload.type === "leave") {
          dropActive = false;
        } else if (event.payload.type === "drop") {
          dropActive = false;
          const path = event.payload.paths[0];
          if (path) void machine.processFile(path);
        }
      })
      .then((fn) => (unlisten = fn))
      .catch(() => {
        /* not running inside Tauri (e.g. plain vite dev) */
      });
    return () => unlisten?.();
  });

  async function browse() {
    let selected: string | null;
    try {
      selected = await open({
        multiple: false,
        directory: false,
        title: "Choose a playlist",
        filters: [
          { name: "Playlist", extensions: ["jwlplaylist", "zip"] },
          { name: "All files", extensions: ["*"] },
        ],
      });
    } catch {
      return;
    }
    if (selected) void machine.processFile(selected);
  }

  async function reveal(path: string) {
    try {
      await revealItemInDir(path);
    } catch {
      /* the file may have been moved; nothing useful to do */
    }
  }

  function fileName(path: string): string {
    const parts = path.split(/[\\/]/);
    return parts[parts.length - 1] ?? path;
  }
</script>

<main class:drop-active={dropActive} class:done={ui.kind === "done"}>
  <div class="column">
    <header>
      <h1>Cuesheet</h1>
      <hr class="rule" />
    </header>

    {#if ui.kind === "idle"}
      <section class="stage" aria-live="polite">
        <button class="primary" onclick={browse}>Choose a playlist…</button>
        <p class="sidenote">…or drop a playlist anywhere on this window</p>
      </section>
    {:else if ui.kind === "generating"}
      <section class="stage" aria-live="polite">
        <div class="working" role="status">
          <span class="working-bar" aria-hidden="true"></span>
          <p>Generating your cuesheet…</p>
        </div>
      </section>
    {:else if ui.kind === "done"}
      <section class="stage" aria-live="polite">
        <p class="saved">Saved&nbsp;✓</p>
        <p class="saved-detail">
          {ui.playlistName} · {ui.cueCount}
          {ui.cueCount === 1 ? "cue" : "cues"} · {fileName(ui.savedPath)}
        </p>
        <p class="actions">
          <button class="link" onclick={() => reveal(ui.kind === "done" ? ui.savedPath : "")}>
            Reveal
          </button>
          <span class="dot" aria-hidden="true">·</span>
          <button class="link" onclick={() => machine.reset()}>Generate another</button>
        </p>
      </section>
    {:else if ui.kind === "error"}
      <section class="stage" aria-live="assertive">
        <p class="error-message">{ui.message}</p>
        <details class="error-details" bind:open={detailsOpen}>
          <summary>Details</summary>
          <p>{ui.details}</p>
        </details>
        <p class="actions">
          <button class="link" onclick={() => machine.reset()}>Try again</button>
        </p>
      </section>
    {/if}

    <footer>
      <hr class="rule" />
      <p class="footnote">Cuesheets are generated entirely offline.</p>
    </footer>
  </div>
</main>

<style>
  main {
    height: 100vh;
    display: flex;
    justify-content: center;
    border: 3px solid transparent;
    transition: border-color 120ms ease, background-color 200ms ease;
    background-color: var(--cream);
  }

  main.drop-active {
    border-color: var(--fox);
  }

  main.done {
    background-color: var(--apple-cider);
  }

  .column {
    width: min(34rem, 88vw);
    display: flex;
    flex-direction: column;
    padding: 2.25rem 0 1.5rem;
  }

  h1 {
    font-size: 1.6rem;
    font-weight: 500;
    font-style: italic;
    letter-spacing: 0.01em;
    margin: 0 0 0.6rem;
  }

  .rule {
    border: none;
    border-top: 1px solid var(--russet);
    opacity: 0.45;
    margin: 0;
    width: 100%;
  }

  .stage {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    gap: 0.4rem;
  }

  .primary {
    appearance: none;
    border: none;
    background: var(--fox);
    color: var(--cream);
    font-size: 1.25rem;
    padding: 0.85rem 2.2rem;
    border-radius: 6px;
    cursor: pointer;
    transition: background-color 120ms ease, transform 120ms ease;
  }

  .primary:hover {
    background: var(--mustard);
    color: var(--ink);
  }

  .primary:active {
    transform: translateY(1px);
  }

  .sidenote {
    font-size: 0.85rem;
    font-style: italic;
    color: var(--russet);
    margin: 0.6rem 0 0;
  }

  .working {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .working p {
    margin: 0;
    font-style: italic;
    color: var(--russet);
  }

  .working-bar {
    display: block;
    width: 11rem;
    height: 3px;
    background: var(--apple-cider);
    border-radius: 2px;
    position: relative;
    overflow: hidden;
  }

  .working-bar::after {
    content: "";
    position: absolute;
    inset: 0;
    width: 40%;
    background: var(--mustard);
    border-radius: 2px;
    animation: sweep 1.1s ease-in-out infinite alternate;
  }

  @keyframes sweep {
    from {
      transform: translateX(-40%);
    }
    to {
      transform: translateX(260%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .working-bar::after {
      animation: none;
      width: 100%;
      transform: none;
    }
  }

  .saved {
    font-size: 1.35rem;
    margin: 0;
  }

  .saved-detail {
    margin: 0;
    font-size: 0.9rem;
    color: var(--russet);
  }

  .actions {
    margin: 0.9rem 0 0;
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
  }

  .dot {
    color: var(--russet);
  }

  .link {
    appearance: none;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    color: var(--fox);
    text-decoration: underline;
    text-underline-offset: 3px;
    font-size: 1rem;
  }

  .link:hover {
    color: var(--mustard);
  }

  .error-message {
    font-size: 1.1rem;
    margin: 0;
  }

  .error-details {
    max-width: 100%;
    font-size: 0.85rem;
    color: var(--russet);
  }

  .error-details summary {
    cursor: pointer;
    font-style: italic;
  }

  .error-details p {
    margin: 0.4rem 0 0;
    user-select: text;
    -webkit-user-select: text;
    overflow-wrap: anywhere;
    text-align: left;
  }

  footer .footnote {
    margin: 0.6rem 0 0;
    font-size: 0.75rem;
    font-style: italic;
    color: var(--russet);
    text-align: center;
  }
</style>
