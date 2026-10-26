// A repository's app, started from Bridge: several servers held as one run. Mirrored by hand from `crates/ipc/src/app_runs.rs`.
//
// **A process is a server read through**: `server_id` is what `observe_server` takes, and its output is that socket's. The header rules in
// `events.ts` hold: hand-written, and an optional field is left out when empty, never `null`.

import type { RunPlace } from "./run-memory";
import type { ServerLink } from "./servers";

/** Whether a run is opened at an address or is a window of its own. */
export type AppRunOpensIn = "browser" | "app";

/** Where one process stands. A server that ends on its own is `failed`, whatever its code; `exited` is one somebody stopped. */
export type AppRunState = "not_started" | "starting" | "ready" | "exited" | "failed";

/** A repository's main checkout, or the worktree a Job holds. No other checkout has a run. */
export type AppRunCheckout =
  | { kind: "main"; path: string }
  | {
      kind: "job";
      path: string;
      job_id: string;
      /** The branch the Job's bay is on. */
      branch?: string;
      /** The bay's number. */
      bay: number;
    };

/** One server of a run. */
export type AppRunProcess = {
  /** The server's name in the Manifest. */
  name: string;
  state: AppRunState;
  /** The port leased to it on this checkout; main and a Job's bay hold different ones. Absent before it has started once. */
  port?: number;
  /** The number it is pinned to: it is on `port` every time, and a sibling that hardcodes it finds it. Absent: Fleet leases it. */
  pinned_port?: number;
  /** Where it is opened. Only while `ready`. */
  links?: ServerLink[];
  /** The instance behind it, what `observe_server` takes. A restart is a new instance: read it again from the next `app_run.changed`. */
  server_id?: string;
  /** How it ended, in a sentence. */
  ended?: string;
};

/**
 * One process expecting another on a port it does not get: a frontend whose dev proxy points at `127.0.0.1:8731` while the backend is leased another number.
 * At most one for a pair. Found in a file the process's folder holds, or in its output, and kept, so the next draft and start show it.
 */
export type AppRunLink = {
  /** The process that expects it, by its name in the run. */
  from: string;
  /** The process it expects on `port`. */
  to: string;
  port: number;
  /** The file, relative to the repository's root, or `log` where `from`'s output showed a refused connection to `port`. */
  found_in: string;
  /** What to hand a Job to mend it. Bridge opens Dispatch prefilled with it; Fleet starts no Job. */
  brief: string;
};

/**
 * One run on one checkout: a row of `list_app_runs`, what `start_app_run`, `stop_app_run` and `restart_app_run` answer, and `app_run.changed` whole.
 * Keyed by `manifest_id`, `checkout.path` and `name`. Not `kind`, which the event carries beside it.
 */
export type AppRun = {
  /** Its key under `runs:`. What every act on it names. */
  name: string;
  /** What a person is shown: its label, else its name. */
  label: string;
  opens_in: AppRunOpensIn;
  manifest_id: string;
  checkout: AppRunCheckout;
  /** In the order the file wrote them. */
  processes: AppRunProcess[];
  /** When a process of it was last started here, as far as this Fleet has seen since it started. Absent is not "never". */
  last_used?: string;
  /** Where one process expects another on a port it was not given. Left out when there is none. */
  links?: AppRunLink[];
};

/** `GET /app_runs`. */
export type AppRunList = { runs: AppRun[] };

/** The body of `start_app_run`, `stop_app_run` and `restart_app_run`: a name, never a command and never a path. */
export type NamedAppRun = {
  name: string;
  /** The Job whose worktree it runs on. Absent: the repository's main checkout, which `?manifest_id=` names. */
  job_id?: string;
};

/** One process as it is proposed and as a person confirms it. */
export type AppRunDraftProcess = {
  /** Letters, digits and `_`. */
  name: string;
  /** The folder under the repository's root. Absent: the root. */
  dir?: string;
  /** `${port}` stands for the port Fleet leases the checkout: a command with it listens, and one without listens on none. */
  command: string;
  /** The variable Armada sets to the port. Handed back as it was read; never the person's to edit. */
  port_env?: string;
};

/**
 * A Command the checkout runs before the run's processes can start: an install, a `.venv` made.
 * Saved as a Command and named in `setup.requires`, so it runs once on each checkout and again when what it was traced from changes.
 */
export type AppRunDraftSetup = {
  /** Letters, digits and `_`. */
  name: string;
  /** What it runs, from the checkout's root. */
  command: string;
};

/**
 * `GET /app_runs/draft`: what is proposed that no saved run already starts, so *Add a way to run* has a draft where the repository declares runs too. Saves nothing.
 * No `processes` is a repository nothing was learnt of, or one whose saved runs cover what was.
 */
export type AppRunDraft = {
  /** The repository's root, absolute: what a process's `dir` is read from. */
  root: string;
  /** Where saving it puts it: the place the repository's Kit is kept. */
  place: RunPlace;
  processes: AppRunDraftProcess[];
  /** Where a proposed process expects another, proposed or already saved. Left out when there is none. */
  links?: AppRunLink[];
  /** What the checkout runs first, in order: an install for each lockfile beside a `package.json`, a `.venv` for each `requirements.txt` or `pyproject.toml`. Only what no Command declared already runs. Left out when there is nothing to install. */
  setup?: AppRunDraftSetup[];
};

/** The body of `save_app_run`: a confirmed draft and the name its person gave it. */
export type SaveAppRun = {
  name: string;
  processes: AppRunDraftProcess[];
  /** The setup as confirmed: each written as a Command and added to `setup.requires` after what is there. */
  setup?: AppRunDraftSetup[];
  /** The Job whose worktree it then starts on. Absent: the main checkout. */
  job_id?: string;
};

/** What `save_app_run` answers. */
export type SavedAppRun = {
  place: RunPlace;
  /** The file it was written to, as that place names it. */
  file: string;
  run: AppRun;
};

/** The body of `pin_app_run_port`: a process, and the number its siblings expect it on. Written into Kit's copy; nothing is restarted. */
export type PinAppRunPort = {
  /** The process, by its name in the run. It must listen on a leased port. */
  process: string;
  /** From 1024 to 65535. Where something else holds it the next start is refused with `fleet.pinned_port_taken`, naming what. */
  port: number;
  /** The run to answer, by its key. Absent: the first run that lists `process`. */
  name?: string;
  /** The Job whose worktree is read. Absent: the repository's main checkout, which `?manifest_id=` names. */
  job_id?: string;
};

/** The body of `unpin_app_run_port`: a process given back to leasing. Answers the `AppRun`. */
export type UnpinAppRunPort = {
  process: string;
  name?: string;
  job_id?: string;
};
