// The merge line `armada land` keeps in each repository Fleet serves —
// `crates/ipc/src/merge_line.rs`. Since protocol 22.1; `landed` and `sent_back` since 23.2.
//
// Read once per connection off `get_merge_lines`, and replaced whole by every
// `merge_lines.changed`. The header rules in `events.ts` hold: hand-written,
// and every closed set left as `string`.

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
