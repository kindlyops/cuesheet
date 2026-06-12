// The whole UI state machine, kept DOM-free so it is unit-testable.
//
//   idle → generating → (save dialog) → done | error
//
// Cancelling the save dialog returns quietly to idle.

export interface GenerateResponse {
  playlistName: string;
  suggestedFilename: string;
  cueCount: number;
}

export type State =
  | { kind: "idle" }
  | { kind: "generating" }
  | { kind: "done"; playlistName: string; cueCount: number; savedPath: string }
  | { kind: "error"; message: string; details: string };

/**
 * Everything the machine needs from the outside world. The real app wires
 * these to Tauri (`invoke`, plugin-dialog); tests pass stubs.
 */
export interface MachineDeps {
  /** Run the core pipeline on a playlist file. Rejects with a user-friendly string. */
  generate(path: string): Promise<GenerateResponse>;
  /** Show the native save dialog; resolves null when the user cancels. */
  chooseSavePath(suggestedFilename: string): Promise<string | null>;
  /** Write the held PDF bytes to the chosen location. */
  save(targetPath: string): Promise<void>;
}

export interface Machine {
  readonly state: State;
  /** Drive a playlist file through generate → save dialog → save. */
  processFile(path: string): Promise<void>;
  /** Return to idle (from done or error). */
  reset(): void;
  subscribe(listener: (state: State) => void): () => void;
}

function errorString(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

export function createMachine(deps: MachineDeps): Machine {
  let state: State = { kind: "idle" };
  const listeners = new Set<(state: State) => void>();

  function setState(next: State) {
    state = next;
    for (const listener of listeners) listener(state);
  }

  return {
    get state() {
      return state;
    },

    async processFile(path: string) {
      // Ignore drops/clicks while a generation is already in flight.
      if (state.kind === "generating") return;

      setState({ kind: "generating" });

      let generated: GenerateResponse;
      try {
        generated = await deps.generate(path);
      } catch (e) {
        setState({
          kind: "error",
          message: "Couldn't generate a cuesheet from that file.",
          details: errorString(e),
        });
        return;
      }

      let target: string | null;
      try {
        target = await deps.chooseSavePath(generated.suggestedFilename);
      } catch (e) {
        setState({
          kind: "error",
          message: "The save dialog could not be opened.",
          details: errorString(e),
        });
        return;
      }

      if (target === null) {
        // User cancelled the save dialog — back to idle, no fuss.
        setState({ kind: "idle" });
        return;
      }

      try {
        await deps.save(target);
      } catch (e) {
        setState({
          kind: "error",
          message: "The cuesheet was generated but could not be saved.",
          details: errorString(e),
        });
        return;
      }

      setState({
        kind: "done",
        playlistName: generated.playlistName,
        cueCount: generated.cueCount,
        savedPath: target,
      });
    },

    reset() {
      if (state.kind === "generating") return;
      setState({ kind: "idle" });
    },

    subscribe(listener: (s: State) => void) {
      listeners.add(listener);
      listener(state);
      return () => listeners.delete(listener);
    },
  };
}
