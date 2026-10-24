// Mock-only: the mods each Mod Job made, held in memory beside the mock's themes. A theme mod applies when it is switched on and
// Bridge falls back to Dark when it is switched off, which is what `mockThemes` does with an active theme that is no longer offered.

import type { ModMade, ModsMadeSource } from "../mods-made";
import { mockThemes } from "./themes";

let made: Readonly<Record<string, readonly ModMade[]>> = {};
const listeners = new Set<() => void>();
const NONE: readonly ModMade[] = [];

export const mockModsMade: ModsMadeSource & {
  /** A mod the Job wrote into the mod folder, on and in force, as it applies when saved. */
  add(jobId: string, mod: ModMade): void;
  reset(): void;
} = {
  get: (jobId) => made[jobId] ?? NONE,
  subscribe: (on) => {
    listeners.add(on);
    return () => void listeners.delete(on);
  },
  setEnabled: (jobId, name, enabled) => {
    made = { ...made, [jobId]: (made[jobId] ?? NONE).map((one) => (one.name === name ? { ...one, enabled } : one)) };
    mockThemes.setEnabled(name, enabled);
    if (enabled) mockThemes.setActive(name);
    listeners.forEach((on) => on());
  },
  add: (jobId, mod) => {
    made = { ...made, [jobId]: [...(made[jobId] ?? NONE), mod] };
    listeners.forEach((on) => on());
  },
  reset: () => {
    made = {};
    listeners.forEach((on) => on());
  },
};
