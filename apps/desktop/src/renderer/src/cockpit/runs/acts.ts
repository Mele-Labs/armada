// What pressing Run does: start the run chosen and open its panel, or ask first where the repository has none saved. The confirm's own
// state is held here and drawn by `RunHost`, so the list's heading and a Job's header press one thing.

import { useSyncExternalStore } from "react";
import type { SavedRun } from "@armada/components";

import { openRun } from "../deck/store";
import type { RunsSource, RunTarget } from "./source";

/** The confirm that is open: what it runs, and whether it adds to the runs already saved or is the first. */
export type Asked = { target: RunTarget; label: string; add: boolean };

let asked: Asked | undefined;
const listeners = new Set<() => void>();
const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));
const ask = (next: Asked | undefined) => {
  asked = next;
  listeners.forEach((one) => one());
};

export const useAsked = (): Asked | undefined => useSyncExternalStore(subscribe, () => asked);
export const dismissAsked = () => ask(undefined);

/** Run, as the face presses it (`run` is the last used, or none where nothing is saved) and as a row of the picker does. */
export function pressRun(source: RunsSource, target: RunTarget, label: string, run: SavedRun | undefined): void {
  if (run === undefined) ask({ target, label, add: false });
  else openRun(source.start(target, run));
}

/** Add a run: the same confirm, over a repository that already has some. */
export const pressAddRun = (target: RunTarget, label: string): void => ask({ target, label, add: true });
