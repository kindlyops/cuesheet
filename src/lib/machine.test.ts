import { describe, expect, it, vi, beforeEach } from "vitest";
import { createMachine, type MachineDeps, type State } from "./machine";

const RESPONSE = {
  playlistName: "Autumn Service",
  suggestedFilename: "Autumn-Service-cuesheet.pdf",
  cueCount: 7,
};

function makeDeps(overrides: Partial<MachineDeps> = {}): MachineDeps {
  return {
    generate: vi.fn().mockResolvedValue(RESPONSE),
    chooseSavePath: vi.fn().mockResolvedValue("/tmp/Autumn-Service-cuesheet.pdf"),
    save: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

describe("cuesheet state machine", () => {
  it("starts idle", () => {
    expect(createMachine(makeDeps()).state).toEqual({ kind: "idle" });
  });

  it("walks idle → generating → done on the happy path", async () => {
    const deps = makeDeps();
    const machine = createMachine(deps);
    const seen: State["kind"][] = [];
    machine.subscribe((s) => seen.push(s.kind));

    await machine.processFile("/playlists/autumn.jwlplaylist");

    expect(seen).toEqual(["idle", "generating", "done"]);
    expect(machine.state).toEqual({
      kind: "done",
      playlistName: "Autumn Service",
      cueCount: 7,
      savedPath: "/tmp/Autumn-Service-cuesheet.pdf",
    });
    expect(deps.generate).toHaveBeenCalledWith("/playlists/autumn.jwlplaylist");
    expect(deps.chooseSavePath).toHaveBeenCalledWith("Autumn-Service-cuesheet.pdf");
    expect(deps.save).toHaveBeenCalledWith("/tmp/Autumn-Service-cuesheet.pdf");
  });

  it("returns quietly to idle when the save dialog is cancelled", async () => {
    const deps = makeDeps({ chooseSavePath: vi.fn().mockResolvedValue(null) });
    const machine = createMachine(deps);

    await machine.processFile("/playlists/autumn.jwlplaylist");

    expect(machine.state).toEqual({ kind: "idle" });
    expect(deps.save).not.toHaveBeenCalled();
  });

  it("surfaces backend error strings in the error state", async () => {
    const deps = makeDeps({
      generate: vi
        .fn()
        .mockRejectedValue("unsupported schema version 15 (expected 14)"),
    });
    const machine = createMachine(deps);

    await machine.processFile("/playlists/future.jwlplaylist");

    expect(machine.state).toEqual({
      kind: "error",
      message: "Couldn't generate a cuesheet from that file.",
      details: "unsupported schema version 15 (expected 14)",
    });
  });

  it("reports save failures after a successful generate", async () => {
    const deps = makeDeps({
      save: vi.fn().mockRejectedValue("Could not save the PDF: disk full"),
    });
    const machine = createMachine(deps);

    await machine.processFile("/playlists/autumn.jwlplaylist");

    expect(machine.state.kind).toBe("error");
    if (machine.state.kind === "error") {
      expect(machine.state.details).toContain("disk full");
    }
  });

  it("ignores a second file while already generating", async () => {
    let resolveGenerate!: (r: typeof RESPONSE) => void;
    const deps = makeDeps({
      generate: vi.fn().mockReturnValue(
        new Promise<typeof RESPONSE>((resolve) => (resolveGenerate = resolve)),
      ),
    });
    const machine = createMachine(deps);

    const first = machine.processFile("/a.jwlplaylist");
    await machine.processFile("/b.jwlplaylist");

    expect(deps.generate).toHaveBeenCalledTimes(1);
    resolveGenerate(RESPONSE);
    await first;
    expect(machine.state.kind).toBe("done");
  });

  it("resets from done and error back to idle, but not mid-generate", async () => {
    const machine = createMachine(makeDeps());
    await machine.processFile("/a.jwlplaylist");
    expect(machine.state.kind).toBe("done");
    machine.reset();
    expect(machine.state).toEqual({ kind: "idle" });

    let resolveGenerate!: (r: typeof RESPONSE) => void;
    const slow = createMachine(
      makeDeps({
        generate: vi.fn().mockReturnValue(
          new Promise<typeof RESPONSE>((resolve) => (resolveGenerate = resolve)),
        ),
      }),
    );
    const inFlight = slow.processFile("/a.jwlplaylist");
    slow.reset();
    expect(slow.state.kind).toBe("generating");
    resolveGenerate(RESPONSE);
    await inFlight;
  });

  it("unsubscribe stops notifications", async () => {
    const machine = createMachine(makeDeps());
    const listener = vi.fn();
    const unsubscribe = machine.subscribe(listener);
    expect(listener).toHaveBeenCalledTimes(1); // initial state
    unsubscribe();
    await machine.processFile("/a.jwlplaylist");
    expect(listener).toHaveBeenCalledTimes(1);
  });
});

describe("tauriDeps wiring", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it("maps generate/save onto the Tauri invoke commands", async () => {
    const invoke = vi.fn().mockResolvedValue(RESPONSE);
    const save = vi.fn().mockResolvedValue("/out.pdf");
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));
    vi.doMock("@tauri-apps/plugin-dialog", () => ({ save }));

    const { tauriDeps } = await import("./tauri-deps");

    await expect(tauriDeps.generate("/p.jwlplaylist")).resolves.toEqual(RESPONSE);
    expect(invoke).toHaveBeenCalledWith("generate_cuesheet", {
      path: "/p.jwlplaylist",
    });

    await expect(tauriDeps.chooseSavePath("x-cuesheet.pdf")).resolves.toBe(
      "/out.pdf",
    );
    expect(save).toHaveBeenCalledWith({
      defaultPath: "x-cuesheet.pdf",
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });

    await tauriDeps.save("/out.pdf");
    expect(invoke).toHaveBeenCalledWith("save_cuesheet", {
      targetPath: "/out.pdf",
    });
  });
});
