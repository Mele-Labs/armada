// A fake source of runs for the mock: saved runs and drafts from a scenario's seed, and processes that go starting then ready as time
// passes, writing a few lines each. No Fleet serves any of this yet, so a started process is only this module's timers.

import type { RunDraft, RunProcess, RunProcessSpec, SavedRun } from "@armada/components";

import type { RunsApp, RunsSource, RunsState, RunTarget, SavedRuns } from "../cockpit/runs/source";

/** What a window starts with: each repository's saved runs and last use, and the run drafted for one with none, by root. */
export type RunsSeed = {
  saved: Readonly<Record<string, SavedRuns>>;
  drafts: Readonly<Record<string, RunDraft>>;
};

/** When each process of a run comes up, counted from the press: the first, then a step later for each after it. */
const FIRST_READY_MS = 900;
const NEXT_READY_MS = 800;

const resolved = (spec: RunProcessSpec): string => spec.command.replaceAll("${port}", spec.port === undefined ? "" : String(spec.port));

/** What a process writes as it starts and once it is up, by what it runs. Quoted text of a program, never prose of ours. */
function startLines(spec: RunProcessSpec): readonly string[] {
  const command = resolved(spec);
  const where = spec.folder === "." ? "" : `${spec.folder}/`;
  return [`${where}$ ${command}`];
}

function upLines(spec: RunProcessSpec): readonly string[] {
  const port = spec.port;
  if (spec.command.includes("uvicorn")) return ["INFO:     Started server process [48213]", "INFO:     Waiting for application startup.", "INFO:     Application startup complete.", `INFO:     Uvicorn running on http://127.0.0.1:${port} (Press CTRL+C to quit)`];
  if (spec.command.includes("vite")) return ["VITE v5.4.2  ready in 412 ms", `Local:   http://localhost:${port}/`, "Network: use --host to expose"];
  if (spec.command.includes("storybook")) return ["Storybook 9.1.2 for react-vite started", `Local:            http://localhost:${port}/`];
  if (spec.command.includes("pnpm dev")) return ["electron-vite dev server running for the renderer process", "start electron app...", "Fleet connected, protocol 46.3"];
  return port === undefined ? ["Started"] : [`Listening on http://localhost:${port}`];
}

export function createMockRuns(seed: RunsSeed): RunsSource {
  let state: RunsState = { saved: seed.saved, apps: [] };
  const listeners = new Set<() => void>();
  const timers = new Map<string, number[]>();
  const specs = new Map<string, readonly RunProcessSpec[]>();
  let count = 0;

  const set = (next: RunsState) => {
    state = next;
    listeners.forEach((one) => one());
  };
  const change = (id: string, edit: (app: RunsApp) => RunsApp) => set({ ...state, apps: state.apps.map((one) => (one.id === id ? edit(one) : one)) });
  const later = (id: string, ms: number, run: () => void) => {
    const timer = window.setTimeout(run, ms);
    timers.set(id, [...(timers.get(id) ?? []), timer]);
  };
  const clear = (id: string) => {
    timers.get(id)?.forEach((one) => window.clearTimeout(one));
    timers.delete(id);
  };
  const editing = (id: string, process: string, edit: (one: RunProcess) => RunProcess) => change(id, (app) => ({ ...app, processes: app.processes.map((one) => (one.id === process ? edit(one) : one)) }));

  /** Brings each process up in turn, writing its lines a moment apart. */
  function bringUp(id: string, kind: "browser" | "app", of: readonly RunProcessSpec[]) {
    of.forEach((spec, at) => {
      const up = FIRST_READY_MS + at * NEXT_READY_MS;
      later(id, up, () => editing(id, spec.id, (one) => ({ ...one, phase: "ready", ...(kind === "browser" && spec.port !== undefined ? { url: `http://localhost:${spec.port}` } : {}) })));
      upLines(spec).forEach((line, n) => later(id, up - 200 + n * 260, () => editing(id, spec.id, (one) => ({ ...one, log: [...one.log, line] }))));
    });
  }

  function launch(target: RunTarget, run: SavedRun): string {
    const before = state.apps.find((one) => one.repository === target.repository && one.job === target.job);
    if (before !== undefined) clear(before.id);
    count += 1;
    const id = `run-${count}`;
    specs.set(id, run.processes);
    const app: RunsApp = {
      id,
      run: run.name,
      kind: run.kind,
      repository: target.repository,
      ...(target.job === undefined ? {} : { job: target.job }),
      checkout: target.checkout,
      processes: run.processes.map((spec) => ({ id: spec.id, name: spec.name, ...(spec.port === undefined ? {} : { port: spec.port }), phase: "starting" as const, log: startLines(spec) })),
    };
    const saved = state.saved[target.repository];
    set({
      saved: { ...state.saved, [target.repository]: { runs: saved?.runs ?? [run], last: run.id } },
      apps: [...state.apps.filter((one) => one !== before), app],
    });
    bringUp(id, run.kind, run.processes);
    return id;
  }

  return {
    subscribe: (on) => (listeners.add(on), () => void listeners.delete(on)),
    get: () => state,
    draft: (repository) => seed.drafts[repository] ?? { processes: [], savedIn: "this Mac" },
    start: launch,
    save: (target, name, processes) => {
      count += 1;
      const run: SavedRun = { id: `saved-${count}`, name, kind: processes.some((one) => one.port !== undefined) ? "browser" : "app", processes };
      const held = state.saved[target.repository];
      set({ ...state, saved: { ...state.saved, [target.repository]: { runs: [...(held?.runs ?? []), run], last: run.id } } });
      return launch(target, run);
    },
    restart: (id) => {
      const of = specs.get(id);
      const app = state.apps.find((one) => one.id === id);
      if (of === undefined || app === undefined) return;
      clear(id);
      change(id, (one) => ({ ...one, processes: one.processes.map((p) => ({ ...p, phase: "starting" as const, log: [...p.log, "restarting"] })) }));
      bringUp(id, app.kind, of);
    },
    stop: (id) => {
      clear(id);
      change(id, (one) => ({ ...one, processes: one.processes.map((p) => (p.phase === "exited" ? p : { ...p, phase: "exited" as const, log: [...p.log, "stopped"] })) }));
    },
  };
}
