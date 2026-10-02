// The merge line `armada land` keeps in each repository Fleet serves —
// `crates/ipc/src/merge_line.rs`. Since protocol 22.1.
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
  /** The newest few that left it, newest first. */
  off: MergeLineRow[];
};

/** One branch, in line or just off it. */
export type MergeLineRow = {
  branch: string;
  /** 1-based. Absent once off the line. */
  place?: number;
  pull_request?: { number: number; url: string };
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
};
