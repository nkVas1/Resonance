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

export interface RuleView {
  name: string;
  trigger: string;
  profile: string;
  priority: number;
  active: boolean;
}

export interface Snapshot {
  mode: ModeInfo;
  scale: number;
  native: [number, number];
  superRes: boolean;
  guardPending: boolean;
  adapter: string;
  monitor: string;
  vendor: string;
  enableHint: string | null;
  confirmTimeout: number;
  profiles: ProfileView[];
  automationEnabled: boolean;
  activeCause: string | null;
  pinned: string | null;
  rules: RuleView[];
}

export interface RevertTick {
  remaining: number;
}

export type TriggerKind = "foreground" | "running" | "power";

export interface NewRule {
  name: string;
  kind: TriggerKind;
  value: string;
  profile: string;
  priority: number;
}

export const ipc = {
  snapshot: () => invoke<Snapshot>("snapshot"),
  applyProfile: (name: string) => invoke<Snapshot>("apply_profile", { name }),
  confirm: () => invoke<Snapshot>("confirm_state"),
  revert: () => invoke<Snapshot>("revert_now"),
  resumeAutomation: () => invoke<Snapshot>("resume_automation"),
  setAutomation: (enabled: boolean) => invoke<Snapshot>("set_automation", { enabled }),
  addRule: (rule: NewRule) => invoke<Snapshot>("add_rule", { rule }),
  removeRule: (name: string) => invoke<Snapshot>("remove_rule", { name }),

  onSnapshot: (cb: (s: Snapshot) => void): Promise<UnlistenFn> =>
    listen<Snapshot>("snapshot", (e) => cb(e.payload)),
  onRevertTick: (cb: (t: RevertTick) => void): Promise<UnlistenFn> =>
    listen<RevertTick>("revert-tick", (e) => cb(e.payload)),
};
