// The real-world wiring for the state machine: Tauri commands and the
// native save dialog. Kept separate from machine.ts so the machine stays
// dependency-free and this thin layer is mockable in tests.

import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type { GenerateResponse, MachineDeps } from "./machine";

export const tauriDeps: MachineDeps = {
  generate(path: string): Promise<GenerateResponse> {
    return invoke<GenerateResponse>("generate_cuesheet", { path });
  },

  async chooseSavePath(suggestedFilename: string): Promise<string | null> {
    return await save({
      defaultPath: suggestedFilename,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
  },

  save(targetPath: string): Promise<void> {
    return invoke<void>("save_cuesheet", { targetPath });
  },
};
