// The mods a Mod Job made, and each one's switch. **The seam between the Job's panel and whatever knows which mods a Job made**:
// the mock provides a source; where none is provided the panel is not drawn. Turning a mod off is the undo, and a mod applies as
// it is saved, so the switch is the only act.

import { useSyncExternalStore } from "react";
import type { ModKind } from "@armada/components";

export type ModMade = { name: string; title: string; kind: ModKind; enabled: boolean };

export interface ModsMadeSource {
  /** The same array until a mod of this Job changes. */
  get(jobId: string): readonly ModMade[];
  subscribe(on: () => void): () => void;
  setEnabled(jobId: string, name: string, enabled: boolean): void;
}

let held: ModsMadeSource | undefined;

/** Set by the mock's mount for the scenario that plays Mod Jobs, and cleared by every other. */
export function provideModsMade(source: ModsMadeSource | undefined): void {
  held = source;
}

const NONE: readonly ModMade[] = [];
const NEVER = () => () => undefined;

/** The Job's mods and the switch for each, or `undefined` where nothing serves them. */
export function useModsMade(jobId: string): { mods: readonly ModMade[]; setEnabled: (name: string, enabled: boolean) => void } | undefined {
  const source = held;
  const mods = useSyncExternalStore(source?.subscribe ?? NEVER, () => source?.get(jobId) ?? NONE);
  return source === undefined ? undefined : { mods, setEnabled: (name, enabled) => source.setEnabled(jobId, name, enabled) };
}
