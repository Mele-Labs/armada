// What a run is, as the run surfaces draw it. Plain data in, callbacks out: Fleet's servers (`ServerState`, `packages/protocol/src/servers.ts`)
// map onto these when the surface is wired, `serving` to `ready` and an `exited` server that was not stopped to `failed`.

/** A browser run is opened at an address; a desktop app run opens a window of its own and has no address. */
export type RunKind = "browser" | "app";

/** One process of a run, as it is saved and as the confirm edits it. `folder` is a path from the repository's root, `.` at the root. */
export type RunProcessSpec = {
  id: string;
  name: string;
  folder: string;
  /** `${port}` stands for the port Fleet leased the checkout for this process; a command without it listens on none. */
  command: string;
  /** The port this process always runs on in this repository, in place of a leased one. */
  pin?: number;
};

/** Whether a process listens: its command asks for a leased port. */
export const listens = (spec: Pick<RunProcessSpec, "command">): boolean => spec.command.includes("${port}");

/**
 * One process expecting another on a fixed port, found in a config file while the run is drafted (`found` is the file, from the repository's
 * root) or in a refused connection in a log at run time (`found` is `log`). `from` and `to` are process ids.
 */
export type RunLink = { from: string; to: string; port: number; found: string };

/** `found` of a link seen in a log and not in a file. */
export const FOUND_IN_LOG = "log";

/** The variable the other processes read a process's leased port from. */
export const portVariable = (name: string): string => `ARMADA_PORT_${name.trim().toUpperCase().replace(/[^A-Z0-9]+/g, "_") || "NAME"}`;

/** What Fix with a Job starts a Job with. */
export const linkBrief = (link: RunLink, from: string, to: string): string =>
  `Make ${from} read the ${to}'s port from ${portVariable(to)}, falling back to ${link.port}${link.found === FOUND_IN_LOG ? "" : `, in ${link.found}`}`;

/** A run the repository has saved: a name and the processes it starts together. A link, once found, is kept here. */
export type SavedRun = {
  id: string;
  name: string;
  kind: RunKind;
  processes: readonly RunProcessSpec[];
  links?: readonly RunLink[];
};

/**
 * One Command of the repository's setup, the manifest's `setup.requires`: what makes a checkout ready to run, such as `npm ci` in `frontend`.
 * Fleet runs the list in order before a run starts on a checkout that needs it. `folder` is read as a process's is.
 */
export type RunSetupSpec = { id: string; name: string; folder: string; command: string };

/** The run Fleet drafted for a repository that has none saved, and where saving it puts it. */
export type RunDraft = {
  /** The repository's root as it is shown, the home directory already written `~`. A process's folder is read from here. */
  root: string;
  /** The repository's setup: the manifest's where it has one, else drafted from its files. Saved with the processes. */
  setup?: readonly RunSetupSpec[];
  processes: readonly RunProcessSpec[];
  savedIn: "armada.yml" | "this Mac" | "a synced folder" | "cloud storage";
  /** What the repository's files say one process expects of another. */
  links?: readonly RunLink[];
};

/** `setting-up` is a setup Command under way, and `waiting` a process held until setup ends. */
export type RunPhase = "setting-up" | "waiting" | "starting" | "ready" | "exited" | "failed";

/** A process that was started: its state, the address it answers at, and what it wrote. */
export type RunProcess = {
  id: string;
  name: string;
  /** The port leased to it in this checkout; main and a Job's bay hold different ones. */
  port?: number;
  phase: RunPhase;
  /** Its port is pinned, so it is the same in every checkout. */
  pinned?: boolean;
  /** Where a browser run is opened. Absent until the process is ready, and for a desktop app. */
  url?: string;
  log: readonly string[];
};

/** The checkout a run is on: the repository's main, or the bay a Job holds. */
export type RunCheckout = { kind: "main"; branch: string } | { kind: "job"; title: string; branch: string; bay: number };

/** A setup Command as it ran on this checkout: `waiting` for its turn, `setting-up`, `ready` once it is done, or `failed` with its exit code. */
export type RunSetupStep = RunSetupSpec & { phase: RunPhase; code?: number };

/** The setup a run ran before its processes, and what the Commands wrote, one after another. */
export type RunSetup = { steps: readonly RunSetupStep[]; log: readonly string[] };

/** A run that is starting or running now. */
export type RunningApp = {
  id: string;
  /** The saved run it was started from, by name. */
  run: string;
  kind: RunKind;
  checkout: RunCheckout;
  /** Present where this checkout needed setup before the run. */
  setup?: RunSetup;
  processes: readonly RunProcess[];
  /** Links found and not met: a process that expects another on a port it is not on. */
  links?: readonly RunLink[];
};

export const PHASE_SAID: Readonly<Record<RunPhase, string>> = { "setting-up": "Setting up", waiting: "Waiting for setup", starting: "Starting", ready: "Ready", exited: "Exited", failed: "Failed" };

/** What a setup Command's mark says: one that has run is done, not ready. */
export const SETUP_SAID: Readonly<Record<RunPhase, string>> = { ...PHASE_SAID, waiting: "Waiting", ready: "Done", exited: "Stopped" };

/**
 * One state for the whole run: a failure shows first, setup in its own failed Command too, then setup under way, then a start under way,
 * then a run that is up, and exited only once every process is.
 */
export function appPhase(processes: readonly RunProcess[], setup?: RunSetup): RunPhase {
  const steps = setup?.steps ?? [];
  if (steps.some((one) => one.phase === "failed") || processes.some((one) => one.phase === "failed")) return "failed";
  if (steps.some((one) => one.phase === "setting-up") || processes.some((one) => one.phase === "waiting")) return "setting-up";
  if (processes.some((one) => one.phase === "starting")) return "starting";
  if (processes.some((one) => one.phase === "ready")) return "ready";
  return "exited";
}
