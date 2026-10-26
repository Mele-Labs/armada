// What the Cockpit reads of how a repository runs, and what it asks to change. Fleet's app runs are the wired source (`fleet-runs.ts`);
// the mock's is `mock/runs.ts`. A window handed neither draws no run control.

import { createContext, useContext, useSyncExternalStore, type ReactNode } from "react";
import type { RunCheckout, RunDraft, RunningApp, RunProcessSpec, RunSetupSpec, SavedRun } from "@armada/components";

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
  /** What Fleet drafts for a repository with no saved run, from what Jobs ran there and from its files. None where Fleet refused. */
  draft: (repository: string) => Promise<RunDraft | undefined>;
  /** Starts a saved run on a checkout, and answers the id of the app that is starting, or none where Fleet refused. */
  start: (target: RunTarget, run: SavedRun) => Promise<string | undefined>;
  /** Saves a run under `name` where the repository keeps its runs, and its setup with it, then starts it, and answers as `start` does. */
  save: (target: RunTarget, name: string, processes: readonly RunProcessSpec[], setup: readonly RunSetupSpec[]) => Promise<string | undefined>;
  /** Pins a process's port to `port` in this repository, or releases the pin where `port` is absent. The run starts again, so the new port is the one it is on. */
  pin: (id: string, process: string, port: number | undefined) => void;
  restart: (id: string) => void;
  stop: (id: string) => void;
  /** A Job's run control is drawn, until the answer is called: its checkout's runs are read, and its title is what the running panel names. */
  seen: (target: RunTarget) => () => void;
  /** An app's pane is open, until the answer is called: what each of its processes prints is read into `log`. */
  reading: (id: string) => () => void;
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
