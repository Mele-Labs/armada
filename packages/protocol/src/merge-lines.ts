// The merge line `armada land` keeps in each repository Fleet serves —
// `crates/ipc/src/merge_line.rs`. Since protocol 22.1; `landed` and `sent_back` since 23.2, and
// one Check's log on its own socket since 23.7.
//
// Read once per connection off `get_merge_lines`, and replaced whole by every
// `merge_lines.changed`. The header rules in `events.ts` hold: hand-written,
// and every closed set left as `string`.

import type { ProtocolVersion } from "./version";
import type { Requester } from "./requester";

/** Every served repository that has a line. One nobody has run `armada land` in is not here. */
export type MergeLines = { lines: MergeLine[] };

/** One repository's line. */
export type MergeLine = {
  /** The repository's root, as `list_repositories` names it. */
  root: string;
  /** In place order. Empty is a line nobody is in. */
  line: MergeLineRow[];
  /** The newest few that left it, newest first. What a Bridge before 23.2 drew; this one reads the two below. */
  off: MergeLineRow[];
  /** The newest three that landed, newest first. Since 23.2. */
  landed: MergeLineRow[];
  /** Red, conflict or stopped and not back in line, in the last three days, newest first. Since 23.2. */
  sent_back: MergeLineRow[];
  /** Main's CI and the open pull requests, from the forge. Absent where Fleet has read neither. Since 23.41. */
  hub?: MergeLineHub;
};

/**
 * What Fleet read off the forge for one repository, beside the queue: where main's CI stands and
 * every open pull request. `crates/ipc/src/merge_hub.rs`. Since 23.41.
 */
export type MergeLineHub = {
  /** Absent where Fleet has not read main yet. */
  main?: MainStanding;
  /** Newest first, as the forge lists them. Absent is none open. */
  pull_requests?: HubPullRequest[];
  /** The Job working on main's red. Fleet sets none yet. */
  fixing?: HubJob;
};

/** A Job, by its id and what it is called. */
export type HubJob = { id: string; title: string };

/**
 * `main_ci_state` is `green`, `red`, `running` or `nothing_ran`. A red a fix is still running for
 * stays `red`. `nothing_ran` is not green: nothing was proved.
 */
export type MainStanding = {
  state: string;
  commit: string;
  read_at: string;
  /** When main first read red, kept while it stays red. */
  red_since?: string;
  /** Each CI job that failed, in the forge's order. */
  failed?: MainFailedJob[];
  /** The pull request that turned main red, where the forge named one. */
  merge?: MainMerge;
};

export type MainFailedJob = {
  /** The job's name as the forge reports it. */
  name: string;
  /** The Manifest Check it maps to. Absent is ordinary. */
  check?: string;
  log_url?: string;
  /** The tests its log names, in the order printed. Absent where none could be read. */
  tests?: string[];
};

export type MainMerge = {
  number: number;
  url?: string;
  branch?: string;
  /** The Job whose pull request that is. Absent for a person's. */
  job?: HubJob;
};

/** `ci` is `passed`, `running`, `failed` or `waiting_on_main`: failed only on the jobs main is failing. */
export type HubPullRequest = {
  number: number;
  title: string;
  branch: string;
  url: string;
  author?: string;
  /** Absent where nothing has run on it. */
  ci?: string;
  /** The Job that opened it. Absent for a person's. */
  job?: HubJob;
};

/** One branch, in line or just off it. */
export type MergeLineRow = {
  branch: string;
  /** 1-based. Absent once off the line. */
  place?: number;
  /** `settled`, landed only: how it ended, the Job's own `merged` or `closed_unmerged`. Since 23.2. */
  pull_request?: { number: number; url: string; settled?: string };
  /** `land_state` in the generated vocabulary. */
  state: string;
  /** The runner's own words, while gating or merging. */
  doing?: string;
  /** The batch, by its first member's branch. */
  batch?: string;
  /** Landed: the merge commit, whole. */
  merge_commit?: string;
  failed?: string[];
  conflicts?: string[];
  /** Gating, red and stopped: each Check the turn runs, as it stands. Since 23.2. */
  checks?: MergeLineCheck[];
};

/** One Check a turn runs. `state` is `waiting`, `running`, `passed`, `failed` or `timed_out`. */
export type MergeLineCheck = {
  name: string;
  state: string;
  /** The merge line, for the entry's branch. Since 23.40; **absent reads as `outside`**. */
  requester?: Requester;
  /**
   * When the runner began it, off the line's own state. **Absent while it waits**,
   * and on a line state written before the field. Since 23.40.
   */
  started_at?: string;
};

/**
 * One message on a merge line Check's log socket, `observe_land_check`. Since 23.7.
 *
 * **`OutputMessage`'s three with its own first one**: a turn's Check belongs to no Job, so the
 * opening names the line. A finished Check's log opens too, sends what it holds and closes
 * `finished`, so this one socket reads a Check that is running and one that has ended.
 */
export type LandOutputMessage =
  | ({ message: "opened" } & LandOutputOpened)
  | ({ message: "lines" } & { lines: string[] })
  | ({ message: "closed" } & { because: string });

/** Whose log this is, by the line's own names. No path: the runner's logs stay off the wire. */
export type LandOutputOpened = {
  protocol_version: ProtocolVersion;
  root: string;
  branch: string;
  name: string;
  /** Older lines the first read left out, because the window is bounded. */
  skipped: number;
};

/** Which merge line Check's log is asked for: the line's three names, and nothing else. */
export type LandCheckAt = { root: string; branch: string; check: string };

/**
 * The merge line Check's log a window is reading, as main holds it. `FollowedLog`'s shape, by the
 * line's names. **One at a time**: opening another replaces the first.
 */
export type FollowedLandLog =
  | { state: "none" }
  | ({ state: "opening" } & LandCheckAt)
  | ({
      state: "following";
      /** The file's own line number of `lines[0]`, counted from one. */
      fromLine: number;
      /** The newest lines, oldest first, bounded. */
      lines: string[];
      /** Why it ended, `finished`, `unreadable` or `broke`, or absent while it is still arriving. */
      ended?: string;
    } & LandCheckAt)
  | ({ state: "failed"; detail: string } & LandCheckAt);
