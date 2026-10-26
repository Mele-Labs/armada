// What the Cockpit reads of how a repository runs, and what it asks to change. **Mock only for now**: no Fleet route serves it, so a window has no
// source and draws no run control. A wired source maps Fleet's servers (`packages/protocol/src/servers.ts`) onto these shapes.

import { createContext, useContext, useSyncExternalStore, type ReactNode } from "react";
import type { RunCheckout, RunDraft, RunningApp, RunProcessSpec, SavedRun } from "@armada/components";

/** The checkout a run is asked for: a repository by its root, and main or the bay of the Job `job` names. */
export type RunTarget = { repository: string; checkout: RunCheckout; job?: string };

/** A repository's saved runs, and the one used last, by id. */
export type SavedRuns = { runs: readonly SavedRun[]; last?: string };

/** A running app and where it runs: the repository's root, and the Job whose bay it is on where it is not main. */
export type RunsApp = RunningApp & { repository: string; job?: string };

export type RunsState = {
  saved: Readonly<Record<string, SavedRuns>>;
  apps: readonly RunsApp[];
};

export type RunsSource = {
  subscribe: (on: () => void) => () => void;
  /** The state as it stands. A new object only when something changed. */
  get: () => RunsState;
  /** What Fleet drafts for a repository with no saved run, from what Jobs ran there and from its files. */
  draft: (repository: string) => RunDraft;
  /** Starts a saved run on a checkout, and returns the id of the app that is starting. A run already going there is replaced. */
  start: (target: RunTarget, run: SavedRun) => string;
  /** Saves a run under `name` where the repository keeps its runs, then starts it. */
  save: (target: RunTarget, name: string, processes: readonly RunProcessSpec[]) => string;
  /** Pins a process's port to `port` in this repository, or releases the pin where `port` is absent. The run starts again, so the new port is the one it is on. */
  pin: (id: string, process: string, port: number | undefined) => void;
  restart: (id: string) => void;
  stop: (id: string) => void;
};

const Held = createContext<RunsSource | undefined>(undefined);

/** Hands the window a source of runs. Nothing handed, nothing is drawn. */
export function RunsFrom({ source, children }: { source: RunsSource | undefined; children: ReactNode }) {
  return source === undefined ? <>{children}</> : <Held.Provider value={source}>{children}</Held.Provider>;
}

export const useRuns = (): RunsSource | undefined => useContext(Held);

const NOTHING: RunsState = { saved: {}, apps: [] };
const never = () => () => undefined;

/** The state a source holds, read again whenever it changes. */
export function useRunsState(): RunsState {
  const source = useRuns();
  return useSyncExternalStore(source?.subscribe ?? never, source?.get ?? (() => NOTHING));
}

/** The app running now on a checkout, if one is: the repository's main, or a Job. */
export const appOn = (state: RunsState, repository: string, job?: string): RunsApp | undefined => state.apps.find((one) => one.repository === repository && one.job === job);
