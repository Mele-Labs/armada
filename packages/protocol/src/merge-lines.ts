// The merge line `armada land` keeps in each repository Fleet serves —
// `crates/ipc/src/merge_line.rs`. Since protocol 22.1; `landed` and `sent_back` since 23.2, and
// one Check's log on its own socket since 23.7.
//
// Read once per connection off `get_merge_lines`, and replaced whole by every
// `merge_lines.changed`. The header rules in `events.ts` hold: hand-written,
// and every closed set left as `string`.

import type { ProtocolVersion } from "./version";

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
export type MergeLineCheck = { name: string; state: string };

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
