// The build Fleet runs on, where it stands against `origin/main`, and the
// request that restarts onto another. `crates/ipc/src/building.rs`.
//
// **The restart is not served here.** `POST /fleet/build/change` starts
// `scripts/restart-build` detached and answers 202 at once, because Fleet is
// the thing the script stops. What it came to is the next `GET /fleet/build`:
// `restarting` while it works, `failed` with one line if it did not take.
//
// The header rules in `protocol.ts` hold here: these are hand-written, and they
// drift the day a field moves.

/** The tree a build came from: `main` as the checkout holds it, or the preview, which is main with every in-flight branch merged. */
export type BuildSource = "main" | "preview";

/**
 * Where a restart under way has got to. `merging` is the preview's alone and `fetching_main` is
 * main's; `retrying` is the build run again after a stale build script was cleaned.
 */
export type BuildStage =
  | "merging"
  | "fetching_main"
  | "building_fleet"
  | "building_bridge"
  | "restarting_fleet"
  | "reopening_bridge"
  | "retrying";

/** `GET /fleet/build`. */
export type FleetBuildReport = {
  /** The tree the running build came from. */
  on: BuildSource;
  /** The commit it was built from. Absent for a Fleet nothing restarted. */
  commit?: string;
  /**
   * Commits the build holds that `origin/main` lacks, and commits `origin/main`
   * holds that it lacks. Both nought is aligned. **Absent where it cannot be
   * counted**, which draws no mark: it is not aligned.
   */
  position?: {
    ahead: number;
    behind: number;
  };
  /** The build a restart under way is moving Fleet onto. Absent when none is. */
  restarting?: BuildSource;
  /** Where that restart has got to. Absent until the restart says. */
  stage?: BuildStage;
  /** Why the last restart did not take, in one line of plain facts. Absent when it took. */
  failed?: string;
  /**
   * The branches the preview left out of this build, and why each was. Omitted when none were,
   * on `main`, and where the record was written for another build. Sorted by branch. `moved` is a branch
   * whose tip has moved past the one it was left out at; absent is false.
   */
  left_out?: {
    branch: string;
    why: string;
    moved?: boolean;
  }[];
  /** The kept build Fleet runs. Absent where none is kept. */
  running?: KeptBuildRunning;
  /**
   * The kept build a revert swaps onto, and what that changes. Absent where no previous build is
   * kept, and where launchd does not run Fleet from the kept builds.
   */
  previous?: KeptBuildPrevious;
  /** The titles of the hosted Jobs with a turn under way, which their keepers carry across a restart. Omitted when none. */
  carried?: string[];
  /** A revert is under way. Omitted when false. */
  reverting?: boolean;
};

/** A kept build: when it was built (RFC 3339) and the full commit it was built from. */
export type KeptBuildRunning = {
  built_at: string;
  commit: string;
};

/** The build before the running one. */
export type KeptBuildPrevious = KeptBuildRunning & {
  /** Its Bridge differs from the running one's, or either lacks the hash. Omitted when false. */
  bridge_reopens?: boolean;
  /** The Jobs whose change the running build holds and this one does not, by title, or by branch where no Job holds it. Omitted when none. */
  takes_out?: string[];
  /** The Jobs whose change this one holds and the running build does not, named the same way. Omitted when none. */
  brings_back?: string[];
};

/** `POST /fleet/build/change`. */
export type ChangeFleetBuild = {
  build: BuildSource;
  /** Restart with a Drone working, and adopt it. A person's say-so; omitted is no. */
  adopt?: boolean;
};

/** The 202 answer: the restart has begun, onto this build. */
export type FleetBuildChanging = {
  build: BuildSource;
};

/** The 202 answer to `POST /fleet/build/revert`, which takes no body: the undo has begun, onto the build with this commit. */
export type FleetBuildReverting = {
  commit: string;
};
