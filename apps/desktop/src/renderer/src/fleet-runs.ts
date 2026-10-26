// A repository's app runs over Fleet: each served repository's runs on its main checkout, read once Fleet is connected and again on
// every resync, a Job's while its header is drawn, and `app_run.changed` carrying one run whole in between. A process's output, and
// the checkout's setup's, is `observe_server`'s, read only while the app's pane is open. The mock has its own source (`mock/runs.ts`);
// this one is the real wire.

import type { RunCheckout, RunDraft, RunLink, RunPhase, RunProcess, RunProcessSpec, RunSetup, RunSetupSpec, RunSetupStep, SavedRun } from "@armada/components";
import type {
  AppRun,
  AppRunDraftProcess,
  AppRunDraftSetup,
  AppRunLink,
  AppRunProcess,
  AppRunSetup,
  AppRunState,
  Outcome,
  RepositorySummary,
  ServerMessage,
} from "@armada/protocol";

import type { AppRunsApi } from "../../shared/api/app-runs";
import type { BridgeState } from "../../shared/bridge";
import type { RunsApp, RunsSource, RunsState, RunTarget, SavedRuns } from "./cockpit/runs/source";
import { raiseOutcome } from "./tell";

type Facts = Pick<BridgeState, "connection" | "readAt" | "holds">;

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetRuns = AppRunsApi & {
  state: () => Promise<Facts>;
  subscribe: (on: (state: Facts) => void) => () => void;
};

/** How many lines of one process's output are held, as `following.ts` holds a Check's. */
const HELD = 2_000;

const NO_MANIFEST: Outcome = { ok: false, why: "no_manifest" };

const PHASE: Readonly<Record<AppRunState, RunPhase>> = { not_started: "exited", starting: "starting", ready: "ready", exited: "exited", failed: "failed" };

/** One run on one checkout, as Fleet keys it. */
const keyOf = (run: AppRun) => `${run.manifest_id}\n${run.checkout.path}\n${run.name}`;
/** What the Cockpit opens a run's pane by: its repository, its checkout and its name. */
export const appIdOf = (run: AppRun) => `${run.manifest_id}:${run.checkout.kind === "job" ? run.checkout.job_id : "main"}:${run.name}`;

const linkOf = (link: AppRunLink): RunLink => ({ from: link.from, to: link.to, port: link.port, found: link.found_in });
const linksOf = (links: readonly AppRunLink[] | undefined): { links?: RunLink[] } => (links === undefined || links.length === 0 ? {} : { links: links.map(linkOf) });

const specOf = (one: AppRunProcess): RunProcessSpec => ({ id: one.name, name: one.name, folder: one.dir ?? ".", command: one.command, ...(one.pinned_port === undefined ? {} : { pin: one.pinned_port }) });

/** A setup Command as the panel edits it. Fleet's takes no folder, so its line names one where it needs one. */
const setupSpecOf = (one: AppRunDraftSetup): RunSetupSpec => ({ id: one.name, name: one.name, folder: ".", command: one.command });

/** A setup row as Fleet saves it: a folder typed in the panel is gone into before the line, since a setup Command has none of its own. */
export const setupSaved = (one: RunSetupSpec): AppRunDraftSetup => ({
  name: one.name,
  command: one.folder === "." ? one.command : `cd '${one.folder.replace(/'/g, "'\\''")}' && ${one.command}`,
});

/** A run as the picker offers it, and as Edit the run reopens it: each process's command, folder and pin as Fleet saved them. */
export function savedRunOf(run: AppRun): SavedRun {
  return { id: run.name, name: run.label, kind: run.opens_in, processes: run.processes.map(specOf), ...linksOf(run.links) };
}

/** A process held back by its checkout's setup waits; one that never started after setup failed is over with the run. */
function processOf(one: AppRunProcess, log: readonly string[], browser: boolean, setup?: AppRunSetup): RunProcess {
  const port = one.port ?? one.pinned_port;
  const url = browser && one.state === "ready" ? one.links?.[0]?.url : undefined;
  return {
    id: one.name,
    name: one.name,
    phase: one.state === "not_started" && setup?.state === "setting_up" ? "waiting" : PHASE[one.state],
    ...(port === undefined ? {} : { port }),
    ...(one.pinned_port === undefined ? {} : { pinned: true }),
    ...(url === undefined ? {} : { url }),
    log,
    folder: one.dir ?? ".",
    command: one.command,
    ...(one.exit_code === undefined ? {} : { code: one.exit_code }),
    ...(one.log_path === undefined ? {} : { logFile: one.log_path }),
  };
}

/**
 * The checkout's setup as the panel draws it: the Commands before the one running or failed are done, the one running is setting up, the one
 * that failed has its code, and those after wait, or never ran once one failed. Each step's file is setup's one log, which holds every Command's
 * output in turn. A setup that failed with no Command failed is a run that would not start once setup passed, so every Command is done.
 */
export function setupOf(setup: AppRunSetup, lines: ReadonlyMap<string, string>, log: readonly string[]): RunSetup {
  const at = setup.commands.indexOf(setup.failed ?? setup.running ?? "");
  const phaseOf = (n: number): RunPhase => {
    if (setup.state === "ready" || (setup.state === "failed" && at < 0)) return "ready";
    if (setup.state === "setting_up") return at < 0 || n > at ? "waiting" : n === at ? "setting-up" : "ready";
    return n < at ? "ready" : n === at ? "failed" : "exited";
  };
  const steps = setup.commands.map((name, n): RunSetupStep => {
    const phase = phaseOf(n);
    return {
      id: name,
      name,
      folder: ".",
      command: lines.get(name) ?? "",
      phase,
      ...(phase === "failed" && setup.exit_code !== undefined ? { code: setup.exit_code } : {}),
      logFile: setup.log_path,
    };
  });
  return { steps, log };
}

/** A run that was started on its checkout. `job` is what the Cockpit knows of the Job, where it has drawn its header. */
export function appOf(run: AppRun, repository: string, logs: ReadonlyMap<string, readonly string[]>, job?: RunCheckout): RunsApp {
  const checkout: RunCheckout =
    run.checkout.kind === "main"
      ? { kind: "main", branch: "main" }
      : { kind: "job", title: job?.kind === "job" ? job.title : "Job", branch: run.checkout.branch ?? (job?.kind === "job" ? job.branch : ""), bay: run.checkout.bay };
  const lines = new Map((run.setup_commands ?? []).map((one) => [one.name, one.command] as const));
  return {
    id: appIdOf(run),
    run: run.label,
    kind: run.opens_in,
    repository,
    ...(run.checkout.kind === "job" ? { job: run.checkout.job_id } : {}),
    checkout,
    ...(run.setup === undefined ? {} : { setup: setupOf(run.setup, lines, logs.get(run.setup.id) ?? []) }),
    processes: run.processes.map((one) => processOf(one, (one.server_id === undefined ? undefined : logs.get(one.server_id)) ?? [], run.opens_in === "browser", run.setup)),
    ...linksOf(run.links),
  };
}

/** Fleet's place for a repository's Kit, in the words Kit's place picker uses. */
const SAVED_IN: Readonly<Record<string, RunDraft["savedIn"]>> = { repository: "armada.yml", this_mac: "this Mac", synced: "a synced folder", cloud: "cloud storage" };

/** A draft as the running panel edits it. A process is known by its proposed name, which is how its `port_env` goes back with it. */
export function draftOf(processes: readonly AppRunDraftProcess[], root: string, place: string, links?: readonly AppRunLink[], setup?: readonly AppRunDraftSetup[]): RunDraft {
  return {
    root: root.replace(/^\/Users\/[^/]+/, "~"),
    ...(setup === undefined || setup.length === 0 ? {} : { setup: setup.map(setupSpecOf) }),
    processes: processes.map((one) => ({ id: one.name, name: one.name, folder: one.dir ?? ".", command: one.command })),
    savedIn: SAVED_IN[place] ?? "this Mac",
    ...linksOf(links),
  };
}

/** A run with something to show on its checkout: a setup run for it, or a process that has started. */
const started = (run: AppRun) => run.setup !== undefined || run.processes.some((one) => one.state !== "not_started");

export function createFleetRuns(fleet: FleetRuns, tell: (outcome: Outcome) => void = raiseOutcome): RunsSource {
  const runs = new Map<string, AppRun>();
  /** The repositories whose main checkout has been listed this connection, by Manifest. */
  const listed = new Set<string>();
  const jobs = new Map<string, { checkout: RunCheckout; count: number }>();
  const reading = new Map<string, number>();
  const followed = new Set<string>();
  const logs = new Map<string, readonly string[]>();
  /** What a draft's processes carried that a person does not edit, by repository root and then process. */
  const portEnvs = new Map<string, ReadonlyMap<string, string>>();
  let repositories: readonly RepositorySummary[] = [];
  let connected = false;
  let readKey: string | undefined;
  let state: RunsState = { saved: {}, apps: [] };
  const listeners = new Set<() => void>();
  let stop: (() => void) | null = null;

  const rootOf = (manifestId: string) => repositories.find((one) => one.manifest?.id === manifestId || one.implicit_id === manifestId)?.root;
  const manifestOf = (root: string) => {
    const served = repositories.find((one) => one.root === root);
    return served?.manifest?.id ?? served?.implicit_id;
  };

  const derive = (): void => {
    const saved: Record<string, { runs: SavedRun[]; newest?: { name: string; at: string } }> = {};
    for (const manifestId of listed) {
      const root = rootOf(manifestId);
      if (root !== undefined) saved[root] = { runs: [] };
    }
    const apps: { app: RunsApp; at: string }[] = [];
    for (const run of runs.values()) {
      const root = rootOf(run.manifest_id);
      if (root === undefined) continue;
      if (run.checkout.kind === "main") {
        const held = (saved[root] ??= { runs: [] });
        held.runs.push(savedRunOf(run));
        if (run.last_used !== undefined && (held.newest === undefined || run.last_used > held.newest.at)) held.newest = { name: run.name, at: run.last_used };
      }
      if (started(run)) apps.push({ app: appOf(run, root, logs, run.checkout.kind === "job" ? jobs.get(run.checkout.job_id)?.checkout : undefined), at: run.last_used ?? "" });
    }
    // The newest first, so the app a checkout shows is the one started there last.
    apps.sort((a, b) => (a.at < b.at ? 1 : a.at > b.at ? -1 : 0));
    state = {
      saved: Object.fromEntries(Object.entries(saved).map(([root, held]): [string, SavedRuns] => [root, { runs: held.runs, ...(held.newest === undefined ? {} : { last: held.newest.name }) }])),
      apps: apps.map((one) => one.app),
    };
    listeners.forEach((on) => on());
  };

  /** Follows what each process of an open pane prints, and lets go of what no open pane shows. A restart is a new server, so it is followed afresh. */
  const follow = (): void => {
    const wanted = new Set<string>();
    for (const run of runs.values()) {
      if (!reading.has(appIdOf(run))) continue;
      if (run.setup !== undefined) wanted.add(run.setup.id);
      for (const one of run.processes) if (one.server_id !== undefined) wanted.add(one.server_id);
    }
    for (const serverId of wanted) {
      if (followed.has(serverId)) continue;
      followed.add(serverId);
      void fleet.followServer(serverId);
    }
    for (const serverId of [...followed]) {
      if (wanted.has(serverId)) continue;
      followed.delete(serverId);
      logs.delete(serverId);
      fleet.unfollowServer(serverId);
    }
  };

  const fold = (run: AppRun): void => {
    runs.set(keyOf(run), run);
    follow();
    derive();
  };

  const printed = (serverId: string, message: ServerMessage): void => {
    if (!followed.has(serverId)) return;
    if (message.message === "opened") logs.set(serverId, []);
    else if (message.message === "lines") logs.set(serverId, [...(logs.get(serverId) ?? []), ...message.lines].slice(-HELD));
    else return;
    derive();
  };

  /** One checkout's runs, in place of what was held for it. A refusal leaves what stands. */
  const list = async (of: { manifestId: string } | { jobId: string }): Promise<void> => {
    const answer = await fleet.listAppRuns(of);
    if (!answer.ok) return;
    for (const [key, run] of runs) {
      const mine = "manifestId" in of ? run.manifest_id === of.manifestId && run.checkout.kind === "main" : run.checkout.kind === "job" && run.checkout.job_id === of.jobId;
      if (mine) runs.delete(key);
    }
    for (const run of answer.value.runs) runs.set(keyOf(run), run);
    if ("manifestId" in of) listed.add(of.manifestId);
    follow();
    derive();
  };

  const readAll = (): void => {
    for (const one of repositories) if (one.manifest !== undefined) void list({ manifestId: one.manifest.id });
    for (const [jobId, held] of jobs) if (held.count > 0) void list({ jobId });
  };

  const changed = (facts: Facts): void => {
    repositories = facts.holds.repositories ?? [];
    const now = facts.connection.state === "connected";
    if (!now) {
      if (connected) {
        runs.clear();
        listed.clear();
        readKey = undefined;
        follow();
        derive();
      }
      connected = false;
      return;
    }
    connected = true;
    // Read again on a resync, which is how a Bridge that missed an event catches up, and when a repository is added.
    const key = `${facts.readAt ?? ""}\n${repositories.map((one) => one.manifest?.id ?? "").join("\n")}`;
    if (key === readKey) return;
    readKey = key;
    readAll();
  };

  const start = (): void => {
    const offState = fleet.subscribe(changed);
    const offRuns = fleet.onAppRunChanged(fold);
    const offOutput = fleet.onServerOutput(printed);
    void fleet.state().then(changed);
    stop = () => {
      offState();
      offRuns();
      offOutput();
      for (const serverId of followed) fleet.unfollowServer(serverId);
      followed.clear();
      logs.clear();
      connected = false;
      readKey = undefined;
    };
  };

  const byId = (id: string) => [...runs.values()].find((run) => appIdOf(run) === id);
  const jobOf = (run: AppRun) => (run.checkout.kind === "job" ? { job_id: run.checkout.job_id } : {});
  const took = (answer: { ok: true; value: AppRun } | { ok: false; outcome: Outcome }): AppRun | undefined => {
    if (!answer.ok) return void tell(answer.outcome);
    fold(answer.value);
    return answer.value;
  };
  const remember = (target: RunTarget) => {
    if (target.job === undefined) return;
    jobs.set(target.job, { checkout: target.checkout, count: jobs.get(target.job)?.count ?? 0 });
  };

  return {
    subscribe: (on) => {
      listeners.add(on);
      if (stop === null) start();
      return () => {
        listeners.delete(on);
        if (listeners.size === 0 && stop !== null) {
          stop();
          stop = null;
        }
      };
    },
    get: () => state,
    draft: async (repository) => {
      const manifestId = manifestOf(repository);
      if (manifestId === undefined) return void tell(NO_MANIFEST);
      const answer = await fleet.getAppRunDraft(manifestId);
      if (!answer.ok) return void tell(answer.outcome);
      portEnvs.set(repository, new Map(answer.value.processes.flatMap((one) => (one.port_env === undefined ? [] : [[one.name, one.port_env] as const]))));
      return draftOf(answer.value.processes, answer.value.root, answer.value.place, answer.value.links, answer.value.setup);
    },
    start: async (target, run) => {
      const manifestId = manifestOf(target.repository);
      if (manifestId === undefined) return void tell(NO_MANIFEST);
      remember(target);
      const answer = await fleet.actOnAppRun("start", manifestId, { name: run.id, ...(target.job === undefined ? {} : { job_id: target.job }) });
      const ran = took(answer);
      return ran === undefined ? undefined : appIdOf(ran);
    },
    // The run an app was started from, whole as Fleet answers it, with the root and the place a draft reads.
    saved: async (id) => {
      const run = byId(id);
      if (run === undefined) return undefined;
      const answer = await fleet.getAppRunDraft(run.manifest_id);
      if (!answer.ok) return void tell(answer.outcome);
      const root = rootOf(run.manifest_id);
      if (root !== undefined) portEnvs.set(root, new Map(run.processes.flatMap((one) => (one.port_env === undefined ? [] : [[one.name, one.port_env] as const]))));
      const draft = { ...draftOf([], answer.value.root, answer.value.place, run.links, run.setup_commands), processes: run.processes.map(specOf) };
      return { run: run.name, name: run.label, draft };
    },
    save: async (target, name, processes, setup, replacing) => {
      const manifestId = manifestOf(target.repository);
      if (manifestId === undefined) return void tell(NO_MANIFEST);
      remember(target);
      const envs = portEnvs.get(target.repository);
      const job = target.job === undefined ? {} : { job_id: target.job };
      // The pins the replaced run had are Fleet's to keep by process name, so only a pin that changed is written.
      const was = new Map([...runs.values()].filter((one) => one.manifest_id === manifestId && one.name === replacing).flatMap((one) => one.processes).map((one) => [one.name, one.pinned_port] as const));
      const answer = await fleet.saveAppRun(manifestId, {
        name,
        processes: processes.map((spec) => {
          const portEnv = envs?.get(spec.id);
          return { name: spec.name, command: spec.command, ...(spec.folder === "." ? {} : { dir: spec.folder }), ...(portEnv === undefined ? {} : { port_env: portEnv }) };
        }),
        ...(setup.length === 0 ? {} : { setup: setup.map(setupSaved) }),
        ...job,
        ...(replacing === undefined ? {} : { replaces: replacing }),
      });
      if (!answer.ok) return void tell(answer.outcome);
      const saved = answer.value.run;
      // A run renamed by the save is gone under its old name, on every checkout it was held for.
      if (replacing !== undefined && replacing !== saved.name) for (const [key, one] of runs) if (one.manifest_id === manifestId && one.name === replacing) runs.delete(key);
      fold(saved);
      // A pin made while the run is edited is Fleet's to write once the run exists, and then the run starts again on it.
      void (async () => {
        let any = false;
        for (const spec of processes) {
          const before = was.get(spec.name);
          if (spec.pin === before) continue;
          const answer =
            spec.pin === undefined
              ? await fleet.unpinAppRunPort(manifestId, { process: spec.name, name: saved.name, ...job })
              : await fleet.pinAppRunPort(manifestId, { process: spec.name, port: spec.pin, name: saved.name, ...job });
          any = took(answer) !== undefined || any;
        }
        if (any) took(await fleet.actOnAppRun("restart", manifestId, { name: saved.name, ...job }));
        // A Job's run is saved for its repository, which the picker reads from main.
        if (target.job !== undefined) await list({ manifestId });
      })();
      return appIdOf(saved);
    },
    pin: (id, process, port) => {
      const run = byId(id);
      if (run === undefined) return;
      void (async () => {
        const named = { process, name: run.name, ...jobOf(run) };
        const answer = port === undefined ? await fleet.unpinAppRunPort(run.manifest_id, named) : await fleet.pinAppRunPort(run.manifest_id, { ...named, port });
        // Fleet writes the pin and restarts nothing; the run starts again so the port it is on is the one pinned.
        if (took(answer) !== undefined) took(await fleet.actOnAppRun("restart", run.manifest_id, { name: run.name, ...jobOf(run) }));
      })();
    },
    restart: (id) => {
      const run = byId(id);
      if (run !== undefined) void fleet.actOnAppRun("restart", run.manifest_id, { name: run.name, ...jobOf(run) }).then(took);
    },
    stop: (id) => {
      const run = byId(id);
      if (run !== undefined) void fleet.actOnAppRun("stop", run.manifest_id, { name: run.name, ...jobOf(run) }).then(took);
    },
    seen: (target) => {
      const job = target.job;
      if (job === undefined) return () => undefined;
      const first = !jobs.has(job) || jobs.get(job)?.count === 0;
      jobs.set(job, { checkout: target.checkout, count: (jobs.get(job)?.count ?? 0) + 1 });
      if (first && connected) void list({ jobId: job });
      derive();
      return () => {
        const held = jobs.get(job);
        if (held !== undefined) held.count -= 1;
      };
    },
    reading: (id) => {
      reading.set(id, (reading.get(id) ?? 0) + 1);
      follow();
      return () => {
        const count = (reading.get(id) ?? 1) - 1;
        if (count === 0) reading.delete(id);
        else reading.set(id, count);
        follow();
        derive();
      };
    },
  };
}
