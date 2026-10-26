// A fake source of runs for the mock: saved runs and drafts from a scenario's seed, and processes that go starting then ready as time
// passes, writing a few lines each. No Fleet serves any of this yet, so a started process is only this module's timers.

import { listens } from "@armada/components";
import type { RunDraft, RunLink, RunProcess, RunProcessSpec, SavedRun } from "@armada/components";

import type { RunsApp, RunsSource, RunsState, RunTarget, SavedRuns } from "../cockpit/runs/source";

/** What a window starts with: each repository's saved runs and last use, and the run drafted for one with none, by root. */
export type RunsSeed = {
  saved: Readonly<Record<string, SavedRuns>>;
  drafts: Readonly<Record<string, RunDraft>>;
};

/** When each process of a run comes up, counted from the press: the first, then a step later for each after it. */
const FIRST_READY_MS = 900;
const NEXT_READY_MS = 800;
/** How long after the last process is ready a process that expects another on a fixed port is refused, and Fleet reads it from the log. */
const REFUSED_AFTER_MS = 2500;

/** Fleet leases each checkout its own span of ports, so main and a Job's bay can run the same app at once: the lease is what `${port}` is filled with. */
const MAIN_FIRST_PORT = 41210;
const BAY_FIRST_PORT = 42200;

/** The ports a run is leased on one checkout, one for each process that asks for `${port}`, a step apart from the first. A pinned process is on its pin in every checkout. */
function lease(checkout: RunTarget["checkout"], of: readonly RunProcessSpec[]): ReadonlyMap<string, number> {
  const first = checkout.kind === "main" ? MAIN_FIRST_PORT : BAY_FIRST_PORT + checkout.bay * 10;
  return new Map(of.filter(listens).map((spec, at) => [spec.id, spec.pin ?? first + at]));
}

/** A link is met once the process it expects is pinned to the port it expects. */
const unmet = (link: RunLink, of: readonly RunProcessSpec[]): boolean => of.find((spec) => spec.id === link.to)?.pin !== link.port;

/** What the expecting process writes when its connection is refused. Quoted text of a program, never prose of ours. */
function refusal(spec: RunProcessSpec | undefined, link: RunLink): string {
  return spec?.command.includes("vite") ? `[vite] http proxy error: /api/session ECONNREFUSED 127.0.0.1:${link.port}` : `connect ECONNREFUSED 127.0.0.1:${link.port}`;
}

const resolved = (spec: RunProcessSpec, port: number | undefined): string => spec.command.replaceAll("${port}", port === undefined ? "" : String(port));

/** What a process writes as it starts and once it is up, by what it runs. Quoted text of a program, never prose of ours. */
function startLines(spec: RunProcessSpec, port: number | undefined): readonly string[] {
  const command = resolved(spec, port);
  const where = spec.folder === "." ? "" : `${spec.folder}/`;
  return [`${where}$ ${command}`];
}

function upLines(spec: RunProcessSpec, port: number | undefined): readonly string[] {
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
  const leased = new Map<string, ReadonlyMap<string, number>>();
  /** The saved run an app was started from, and the links it can be refused on, by app id. */
  const ranFrom = new Map<string, string>();
  const expected = new Map<string, readonly RunLink[]>();
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

  const updateSaved = (repository: string, runId: string | undefined, edit: (run: SavedRun) => SavedRun) => {
    const held = state.saved[repository];
    if (held === undefined) return;
    set({ ...state, saved: { ...state.saved, [repository]: { ...held, runs: held.runs.map((one) => (one.id === runId ? edit(one) : one)) } } });
  };

  /** Brings each process up in turn, writing its lines a moment apart. A link not met is refused a little after the last one is up, and kept with the saved run from then on. */
  function bringUp(id: string, kind: "browser" | "app", of: readonly RunProcessSpec[]) {
    const ports = leased.get(id);
    of.forEach((spec, at) => {
      const port = ports?.get(spec.id);
      const up = FIRST_READY_MS + at * NEXT_READY_MS;
      later(id, up, () => editing(id, spec.id, (one) => ({ ...one, phase: "ready", ...(kind === "browser" && port !== undefined ? { url: `http://localhost:${port}` } : {}) })));
      upLines(spec, port).forEach((line, n) => later(id, up - 200 + n * 260, () => editing(id, spec.id, (one) => ({ ...one, log: [...one.log, line] }))));
    });
    const refused = FIRST_READY_MS + Math.max(of.length - 1, 0) * NEXT_READY_MS + REFUSED_AFTER_MS;
    (expected.get(id) ?? []).filter((link) => unmet(link, of)).forEach((link) =>
      later(id, refused, () => {
        const line = refusal(of.find((spec) => spec.id === link.from), link);
        editing(id, link.from, (one) => ({ ...one, log: [...one.log, line] }));
        change(id, (app) => ({ ...app, links: [...(app.links ?? []).filter((one) => one.from !== link.from || one.to !== link.to), link] }));
        const app = state.apps.find((one) => one.id === id);
        if (app !== undefined) updateSaved(app.repository, ranFrom.get(id), (run) => ({ ...run, links: [...(run.links ?? []).filter((one) => one.from !== link.from || one.to !== link.to), link] }));
      }),
    );
  }

  function launch(target: RunTarget, run: SavedRun, expects: readonly RunLink[] = run.links ?? []): string {
    const before = state.apps.find((one) => one.repository === target.repository && one.job === target.job);
    if (before !== undefined) clear(before.id);
    count += 1;
    const id = `run-${count}`;
    specs.set(id, run.processes);
    const ports = lease(target.checkout, run.processes);
    leased.set(id, ports);
    ranFrom.set(id, run.id);
    expected.set(id, expects);
    const app: RunsApp = {
      id,
      run: run.name,
      kind: run.kind,
      repository: target.repository,
      ...(target.job === undefined ? {} : { job: target.job }),
      checkout: target.checkout,
      links: (run.links ?? []).filter((link) => unmet(link, run.processes)),
      processes: run.processes.map((spec) => {
        const port = ports.get(spec.id);
        return { id: spec.id, name: spec.name, ...(port === undefined ? {} : { port }), ...(spec.pin === undefined ? {} : { pinned: true }), phase: "starting" as const, log: startLines(spec, port) };
      }),
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
    draft: (repository) => seed.drafts[repository] ?? { root: repository.replace(/^\/Users\/[^/]+/, "~"), processes: [], savedIn: "this Mac" },
    start: launch,
    save: (target, name, processes) => {
      count += 1;
      const run: SavedRun = { id: `saved-${count}`, name, kind: processes.some(listens) ? "browser" : "app", processes };
      const held = state.saved[target.repository];
      set({ ...state, saved: { ...state.saved, [target.repository]: { runs: [...(held?.runs ?? []), run], last: run.id } } });
      const ids = new Set(processes.map((one) => one.id));
      const drafted = (seed.drafts[target.repository]?.links ?? []).filter((link) => ids.has(link.from) && ids.has(link.to));
      return launch(target, run, drafted);
    },
    pin: (id, process, port) => {
      const app = state.apps.find((one) => one.id === id);
      const before = specs.get(id);
      if (app === undefined || before === undefined) return;
      const of: RunProcessSpec[] = before.map((spec) => {
        if (spec.id !== process) return spec;
        const { pin: _released, ...rest } = spec;
        return port === undefined ? rest : { ...rest, pin: port };
      });
      specs.set(id, of);
      updateSaved(app.repository, ranFrom.get(id), (run) => ({ ...run, processes: of }));
      const ports = lease(app.checkout, of);
      leased.set(id, ports);
      clear(id);
      const known = state.saved[app.repository]?.runs.find((one) => one.id === ranFrom.get(id))?.links ?? [];
      change(id, (one) => ({
        ...one,
        links: known.filter((link) => unmet(link, of)),
        processes: one.processes.map((p) => {
          const spec = of.find((one) => one.id === p.id);
          const leasedPort = ports.get(p.id);
          const { pinned: _was, port: _port, ...rest } = p;
          return { ...rest, ...(leasedPort === undefined ? {} : { port: leasedPort }), ...(spec?.pin === undefined ? {} : { pinned: true }), phase: "starting" as const, log: [...p.log, "restarting"] };
        }),
      }));
      bringUp(id, app.kind, of);
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
