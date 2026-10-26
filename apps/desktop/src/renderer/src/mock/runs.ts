// A fake source of runs for the mock: saved runs and drafts from a scenario's seed, and processes that go starting then ready as time
// passes, writing a few lines each. No Fleet is behind it, so a started process is only this module's timers.

import { listens } from "@armada/components";
import type { RunDraft, RunLink, RunProcess, RunProcessSpec, RunSetupSpec, SavedRun } from "@armada/components";

import type { RunsApp, RunsSource, RunsState, RunTarget, SavedRuns } from "../cockpit/runs/source";

/** What a window starts with: each repository's saved runs and last use, and the run drafted for one with none, by root. */
export type RunsSeed = {
  saved: Readonly<Record<string, SavedRuns>>;
  drafts: Readonly<Record<string, RunDraft>>;
  /** A setup Command, by id, that fails the first time it runs, as a network that drops does. Restart runs it again. */
  failsOnce?: string;
};

/** How long a setup Command takes from the press, and the beat between the lines it writes. */
const SETUP_MS = 2000;
const SETUP_LINE_MS = 380;

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

/** What a setup Command writes, by what it runs, and the code it exits with. Quoted text of a program, never prose of ours. */
function setupLines(spec: RunSetupSpec, failing: boolean): { lines: readonly string[]; code: number } {
  const head = `${spec.folder === "." ? "" : `${spec.folder}/`}$ ${spec.command}`;
  if (spec.command.startsWith("npm ci") && failing)
    return { lines: [head, "npm warn deprecated inflight@1.0.6: This module is not supported, and leaks memory.", "npm error code ECONNRESET", "npm error network aborted", "npm error A complete log of this run can be found in: ~/.npm/_logs/2026-10-26T09_14_02_118Z-debug-0.log"], code: 1 };
  if (spec.command.startsWith("npm ci")) return { lines: [head, "npm warn deprecated inflight@1.0.6: This module is not supported, and leaks memory.", "added 412 packages, and audited 413 packages in 6s", "118 packages are looking for funding", "  run `npm fund` for details", "found 0 vulnerabilities"], code: 0 };
  if (spec.command.includes("venv"))
    return {
      lines: [head, "Collecting fastapi==0.115.0 (from -r requirements.txt (line 1))", "Collecting uvicorn==0.30.6 (from -r requirements.txt (line 2))", "Collecting httpx==0.27.2 (from -r requirements.txt (line 3))", "Installing collected packages: sniffio, idna, h11, click, anyio, uvicorn, starlette, httpx, fastapi", "Successfully installed anyio-4.6.0 click-8.1.7 fastapi-0.115.0 h11-0.14.0 httpx-0.27.2 idna-3.10 sniffio-1.3.1 starlette-0.38.6 uvicorn-0.30.6"],
      code: failing ? 1 : 0,
    };
  return { lines: [head], code: failing ? 1 : 0 };
}

/** Where Fleet writes a run's logs on this Mac: the repository, then the checkout, then setup's Commands and the processes by name. */
function logFile(repository: string, checkout: RunTarget["checkout"], kind: "setup" | "process", name: string): string {
  const where = checkout.kind === "main" ? "main" : `bay-${checkout.bay}`;
  const file = name.replace(/^\.+/, "").replace(/[^A-Za-z0-9]+/g, "-").toLowerCase() || "log";
  return `~/Library/Application Support/Armada/runs/${repository.split("/").pop()}/${where}/${kind === "setup" ? "setup/" : ""}${file}.log`;
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
  /** Each repository's setup as it was saved, and the checkouts it has finished on, by `repository:checkout`. */
  const setups = new Map<string, readonly RunSetupSpec[]>(Object.entries(seed.drafts).flatMap(([root, draft]) => (draft.setup === undefined ? [] : [[root, draft.setup] as const])));
  const setUp = new Set<string>();
  let failedOnce = false;
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

  const checkoutKey = (app: { repository: string; checkout: RunTarget["checkout"] }) => `${app.repository}:${app.checkout.kind === "main" ? "main" : app.checkout.bay}`;

  /** Runs the setup Commands one after another, writing their lines to setup's log; the processes start when the last ends, and none do after one fails. */
  function setUpThen(id: string, kind: "browser" | "app", of: readonly RunProcessSpec[]) {
    const app = state.apps.find((one) => one.id === id);
    const steps = app?.setup?.steps ?? [];
    const step = (n: number, edit: (one: (typeof steps)[number]) => (typeof steps)[number]) =>
      change(id, (one) => (one.setup === undefined ? one : { ...one, setup: { ...one.setup, steps: one.setup.steps.map((s, at) => (at === n ? edit(s) : s)) } }));
    const write = (line: string) => change(id, (one) => (one.setup === undefined ? one : { ...one, setup: { ...one.setup, log: [...one.setup.log, line] } }));
    let at = 0;
    for (const [n, spec] of steps.entries()) {
      // A Command that finished on this checkout is not run again.
      if (spec.phase === "ready") continue;
      const failing = !failedOnce && seed.failsOnce === spec.id;
      const { lines, code } = setupLines(spec, failing);
      const begins = at;
      later(id, begins, () => step(n, (one) => ({ ...one, phase: "setting-up" })));
      lines.forEach((line, k) => later(id, begins + k * SETUP_LINE_MS, () => write(line)));
      at += Math.max(SETUP_MS, lines.length * SETUP_LINE_MS);
      if (code !== 0) {
        later(id, at, () => {
          failedOnce = true;
          step(n, (one) => ({ ...one, phase: "failed", code }));
          // The Commands after it and the processes never ran, and the run is over.
          change(id, (one) => ({
            ...one,
            ...(one.setup === undefined ? {} : { setup: { ...one.setup, steps: one.setup.steps.map((s, k) => (k > n ? { ...s, phase: "exited" as const } : s)) } }),
            processes: one.processes.map((p) => ({ ...p, phase: "exited" as const })),
          }));
        });
        return;
      }
      later(id, at, () => step(n, (one) => ({ ...one, phase: "ready" })));
    }
    later(id, at, () => {
      const held = state.apps.find((one) => one.id === id);
      if (held !== undefined) setUp.add(checkoutKey(held));
      const ports = leased.get(id);
      change(id, (one) => ({ ...one, processes: one.processes.map((p) => ({ ...p, phase: "starting" as const, log: [...p.log, ...startLines(of.find((spec) => spec.id === p.id)!, ports?.get(p.id))] })) }));
      bringUp(id, kind, of);
    });
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
    const setup = setups.get(target.repository) ?? [];
    const needs = setup.length > 0 && !setUp.has(checkoutKey(target));
    const app: RunsApp = {
      id,
      run: run.name,
      kind: run.kind,
      repository: target.repository,
      ...(target.job === undefined ? {} : { job: target.job }),
      checkout: target.checkout,
      ...(needs ? { setup: { steps: setup.map((spec) => ({ ...spec, phase: "waiting" as const, logFile: logFile(target.repository, target.checkout, "setup", spec.name) })), log: [] } } : {}),
      links: (run.links ?? []).filter((link) => unmet(link, run.processes)),
      processes: run.processes.map((spec) => {
        const port = ports.get(spec.id);
        return { id: spec.id, name: spec.name, folder: spec.folder, command: spec.command, logFile: logFile(target.repository, target.checkout, "process", spec.name), ...(port === undefined ? {} : { port }), ...(spec.pin === undefined ? {} : { pinned: true }), phase: needs ? ("waiting" as const) : ("starting" as const), log: needs ? [] : startLines(spec, port) };
      }),
    };
    const saved = state.saved[target.repository];
    set({
      saved: { ...state.saved, [target.repository]: { runs: saved?.runs ?? [run], last: run.id } },
      apps: [...state.apps.filter((one) => one !== before), app],
    });
    if (needs) setUpThen(id, run.kind, run.processes);
    else bringUp(id, run.kind, run.processes);
    return id;
  }

  return {
    subscribe: (on) => (listeners.add(on), () => void listeners.delete(on)),
    get: () => state,
    draft: async (repository) => seed.drafts[repository] ?? { root: repository.replace(/^\/Users\/[^/]+/, "~"), processes: [], savedIn: "this Mac" },
    start: async (target, run) => launch(target, run),
    save: async (target, name, processes, setup) => {
      count += 1;
      const run: SavedRun = { id: `saved-${count}`, name, kind: processes.some(listens) ? "browser" : "app", processes };
      const held = state.saved[target.repository];
      set({ ...state, saved: { ...state.saved, [target.repository]: { runs: [...(held?.runs ?? []), run], last: run.id } } });
      setups.set(target.repository, setup);
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
      // A checkout setup has not finished on sets up again from the Command that did not finish, before the processes.
      if (app.setup !== undefined && !setUp.has(checkoutKey(app))) {
        change(id, (one) => ({
          ...one,
          setup: { steps: one.setup!.steps.map(({ code: _code, ...step }) => (step.phase === "ready" ? step : { ...step, phase: "waiting" as const })), log: [...one.setup!.log, "restarting"] },
          processes: one.processes.map((p) => ({ ...p, phase: "waiting" as const })),
        }));
        setUpThen(id, app.kind, of);
        return;
      }
      change(id, (one) => ({ ...one, processes: one.processes.map((p) => ({ ...p, phase: "starting" as const, log: [...p.log, "restarting"] })) }));
      bringUp(id, app.kind, of);
    },
    stop: (id) => {
      clear(id);
      change(id, (one) => ({
        ...one,
        ...(one.setup === undefined ? {} : { setup: { ...one.setup, steps: one.setup.steps.map((step) => (step.phase === "setting-up" || step.phase === "waiting" ? { ...step, phase: "exited" as const } : step)) } }),
        processes: one.processes.map((p) => (p.phase === "exited" ? p : { ...p, phase: "exited" as const, log: [...p.log, "stopped"] })),
      }));
    },
    // A Job's title comes with its target, and every log is written here as it goes.
    seen: () => () => undefined,
    reading: () => () => undefined,
  };
}
