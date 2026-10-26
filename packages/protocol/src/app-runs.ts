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
  /** Where it is opened. Only while `ready`. */
  links?: ServerLink[];
  /** The instance behind it, what `observe_server` takes. A restart is a new instance: read it again from the next `app_run.changed`. */
  server_id?: string;
  /** How it ended, in a sentence. */
  ended?: string;
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

/** `GET /app_runs/draft`: the run proposed for a repository that declares none. Saves nothing. No `processes` is a repository nothing was learnt of. */
export type AppRunDraft = {
  /** The repository's root, absolute: what a process's `dir` is read from. */
  root: string;
  /** Where saving it puts it: the place the repository's Kit is kept. */
  place: RunPlace;
  processes: AppRunDraftProcess[];
};

/** The body of `save_app_run`: a confirmed draft and the name its person gave it. */
export type SaveAppRun = {
  name: string;
  processes: AppRunDraftProcess[];
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
