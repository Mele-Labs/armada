// What Fleet remembers of how a repository runs, and where it keeps it.
// `crates/ipc/src/run_memory.rs`, `docs/practices/protocol.md`, *Onboarding*. Since 19 Oct 2026.
//
// The header rules in `protocol.ts` hold here. `RunFact` is `session-import.ts`'s, which the import and the
// live Jobs produce alike and Fleet merges under `workspace` + `kind` + `command`.

import type { RunFact, RunFactKind } from "./session-import";

/**
 * Where a repository's remembered runs are kept. **Absent a choice it is `this_mac`.** `cloud` is not built:
 * choosing it is a 422 `fleet.run_place_not_yet`.
 */
export type RunPlace = "this_mac" | "repository" | "synced" | "cloud";

/**
 * `get_run_memory_place` (`GET /repositories/runs/place?repository=`), and the body and answer of
 * `set_run_memory_place` (`POST /repositories/runs/place`). `synced_folder` is required to set `synced`, and
 * present only for it; a 422 `fleet.run_place_refused` where it is missing or not a folder.
 */
export type RunMemoryPlace = {
  /** A root `list_repositories` names. */
  repository: string;
  place: RunPlace;
  synced_folder?: string;
};

/** `set_run_memory_place`' body. */
export type SetRunMemoryPlace = RunMemoryPlace;

/** `kept`: the place holds it, and `where` names which. `waiting`: the place has not taken it, and `why` says what it waits on. */
export type RunFactState = "kept" | "waiting";

/** One fact and where it stands: a `RunFact` with `state` beside it. */
export type RememberedRun = RunFact & {
  state: RunFactState;
  /** The place that holds it, where `state` is `kept`. */
  where?: RunPlace;
  /** What it waits on, where `state` is `waiting`. */
  why?: string;
};

/** One workspace's facts. */
export type RunWorkspace = {
  /** Relative to the repository root; `.` is the root. */
  workspace: string;
  runs: RememberedRun[];
};

/** `get_run_memory` (`GET /repositories/runs?repository=`), and `drop_run_fact`' answer. A fact a person dropped is not listed. */
export type RunMemory = {
  repository: string;
  place: RunPlace;
  workspaces: RunWorkspace[];
};

/** `drop_run_fact`' body (`POST /repositories/runs/drop`). The line is never offered or written again. */
export type DropRunFact = {
  repository: string;
  workspace: string;
  kind: RunFactKind;
  command: string;
};
