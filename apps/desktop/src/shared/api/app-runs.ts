// A repository's app, run from Bridge: the runs Fleet lists, the acts on them, and what each process prints. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type {
  AppRun,
  AppRunDraft,
  AppRunList,
  NamedAppRun,
  Outcome,
  PinAppRunPort,
  SaveAppRun,
  SavedAppRun,
  ServerMessage,
  UnpinAppRunPort,
} from "@armada/protocol";

/** What an app-run read or act came to: Fleet's answer, or the refusal to word. */
export type AppRunActed<T> = { ok: true; value: T } | { ok: false; outcome: Outcome };

/** Which checkout `listAppRuns` reads: a repository's main by its Manifest, or the worktree a Job holds. */
export type AppRunsOf = { manifestId: string } | { jobId: string };

export type AppRunsApi = {
  /** `GET /app_runs`: every run the checkout declares, up or not. */
  listAppRuns: (of: AppRunsOf) => Promise<AppRunActed<AppRunList>>;
  /** `GET /app_runs/draft`: what Fleet proposes that no saved run starts. Saves nothing. */
  getAppRunDraft: (manifestId: string) => Promise<AppRunActed<AppRunDraft>>;
  /** `start_app_run`, `stop_app_run` or `restart_app_run`, by the run's name and the Job whose worktree it is on. */
  actOnAppRun: (act: "start" | "stop" | "restart", manifestId: string, named: NamedAppRun) => Promise<AppRunActed<AppRun>>;
  /** `save_app_run`: written where the repository's Kit is kept, then started. */
  saveAppRun: (manifestId: string, save: SaveAppRun) => Promise<AppRunActed<SavedAppRun>>;
  /** `pin_app_run_port`. Nothing is restarted. */
  pinAppRunPort: (manifestId: string, pin: PinAppRunPort) => Promise<AppRunActed<AppRun>>;
  /** `unpin_app_run_port`. */
  unpinAppRunPort: (manifestId: string, unpin: UnpinAppRunPort) => Promise<AppRunActed<AppRun>>;
  /** `app_run.changed`: one run on one checkout, whole. */
  onAppRunChanged: (onChanged: (run: AppRun) => void) => () => void;
  /**
   * Reads one server's output on `observe_server`, for this window alone, until `unfollowServer`. Its messages arrive on
   * `onServerOutput`: `opened`, then the log as it stands, then each line printed.
   */
  followServer: (serverId: string) => Promise<Outcome>;
  unfollowServer: (serverId: string) => void;
  onServerOutput: (onMessage: (serverId: string, message: ServerMessage) => void) => () => void;
};

export const APP_RUNS_CHANNELS = {
  listAppRuns: "bridge:list-app-runs",
  getAppRunDraft: "bridge:get-app-run-draft",
  actOnAppRun: "bridge:act-on-app-run",
  saveAppRun: "bridge:save-app-run",
  pinAppRunPort: "bridge:pin-app-run-port",
  unpinAppRunPort: "bridge:unpin-app-run-port",
  /** Main to every window, not a request. */
  appRunChanged: "bridge:app-run-changed",
  followServer: "bridge:follow-server",
  unfollowServer: "bridge:unfollow-server",
  /** Main to the window that follows the server, not a request. */
  serverOutput: "bridge:server-output",
} as const;
