// Bringing in the folders a person already works in with another agent CLI, and installing Fleet's mod into it.
// `crates/ipc/src/session_import.rs`, `docs/practices/protocol.md`, *Onboarding*. Since 19 Oct 2026.
//
// The header rules in `protocol.ts` hold here. **The agent's name is data**: `ImportFolders.agent` and
// `RunFact.source` are tags Fleet supplies, never a literal Bridge matches on.

/** Whether a folder is a git repository, read from the folder alone. Tagged on `state`. */
export type ImportGit =
  /** The folder is a repository's root. */
  | { state: "root" }
  /** Under a repository's root, which is the folder to serve. */
  | { state: "inside"; root: string }
  | { state: "not_a_repository" }
  /** The folder is gone, so there is nothing to look in. */
  | { state: "unknown" };

/** One folder an agent CLI has kept sessions for. */
export type ImportFolder = {
  /** Absolute. */
  path: string;
  /** Whether it is still there. */
  exists: boolean;
  /** RFC 3339: when the newest session's record was last written. */
  last_session_at: string;
  sessions: number;
  git: ImportGit;
};

/** `list_import_folders` (`GET /import/folders`): newest session first. An empty `folders` is an answer, not a refusal. */
export type ImportFolders = {
  /** The CLI the folders were read from, in its own name. */
  agent: string;
  folders: ImportFolder[];
};

/** What a command was run for. A closed set Bridge draws a label for, so it is spelled out. */
export type RunFactKind = "tests" | "lint" | "build" | "dev_server" | "typecheck" | "format";

/** One thing a repository was seen to run. The shape past sessions and live Jobs both produce. */
export type RunFact = {
  /** Relative to the repository root, `/`-separated; `.` is the root. */
  workspace: string;
  kind: RunFactKind;
  command: string;
  /** Where a dev server was seen listening. */
  port?: number;
  /** Whether the latest run of it worked. */
  succeeded: boolean;
  /** RFC 3339. */
  last_ran: string;
  /** Who saw it run: a tag the reader supplies (`claude_code_sessions`, `jobs`). */
  source: string;
};

/**
 * `get_import_runs` (`GET /import/runs?path=`) for one folder `list_import_folders` answered. A 422
 * `fleet.not_an_import_folder` for any other path.
 */
export type ImportRuns = {
  path: string;
  /** What the workspaces are relative to: the repository's root when the folder is inside one. */
  root: string;
  facts: RunFact[];
};

/**
 * `install_agent_mod`'s body (`POST /import/install_mod`). `requested` must be `true`: `false` is a 422
 * `fleet.install_not_requested`, and the install runs only for a person who pressed it.
 */
export type InstallAgentMod = { requested: boolean };

/** Which step of the install stopped. */
export type ModInstallStep = "populate" | "add_marketplace" | "install";

/** How the install went, answered 200 either way. Tagged on `result`; `said` is the CLI's own words. */
export type ModInstalled =
  | { result: "installed"; said: string }
  | { result: "failed"; step: ModInstallStep; said: string };
