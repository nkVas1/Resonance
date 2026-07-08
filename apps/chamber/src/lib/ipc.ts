// Typed IPC surface between Chamber (UI) and the Tauri backend.
// The UI is a pure function of `Snapshot`; every mutation returns/broadcasts one.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface ModeInfo {
  width: number;
  height: number;
  hz: number;
}

export interface ProfileView {
  name: string;
  available: boolean;
  active: boolean;
  mode: ModeInfo | null;
  scale: number | null;
  /** Linear factor vs native width, e.g. 1.5 for "fifth". */
  ratio: number;
}

export interface Snapshot {
  mode: ModeInfo;
  scale: number;
  native: [number, number];
  superRes: boolean;
  guardPending: boolean;
  adapter: string;
  monitor: string;
  confirmTimeout: number;
  profiles: ProfileView[];
}

export interface RevertTick {
  remaining: number;
}

export const ipc = {
  snapshot: () => invoke<Snapshot>("snapshot"),
  applyProfile: (name: string) => invoke<Snapshot>("apply_profile", { name }),
  confirm: () => invoke<Snapshot>("confirm_state"),
  revert: () => invoke<Snapshot>("revert_now"),

  onSnapshot: (cb: (s: Snapshot) => void): Promise<UnlistenFn> =>
    listen<Snapshot>("snapshot", (e) => cb(e.payload)),
  onRevertTick: (cb: (t: RevertTick) => void): Promise<UnlistenFn> =>
    listen<RevertTick>("revert-tick", (e) => cb(e.payload)),
};
