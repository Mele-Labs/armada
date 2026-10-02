// The merge line: the branches waiting to land on main, and the ones that just
// left it, as Overview's panel and the rail's Merge line surface draw them.
//
// Fleet serves it since protocol 22.1 — `get_merge_lines`, kept current by
// `merge_lines.changed` — reading what `armada land` keeps on disk. This is the
// one fold from that wire onto the composition's rows.

import type { MergeLineEntry, MergeLineState } from "@armada/components";
import type { MergeLineRow, MergeLines } from "@armada/protocol";

/** One repository's line, as the panel takes it. */
export type MergeLineView = {
  /** In place order: `--status`'s numbered rows. */
  line: readonly MergeLineEntry[];
  /** Off the line with an outcome on disk, newest first. */
  off: readonly MergeLineEntry[];
};

/** How much of a merge commit a row shows. */
const SHORT = 10;

/**
 * The line to draw for this pick, or `undefined` for none.
 *
 * **Nothing draws without somebody in line**, whatever just left it: a line
 * nobody is in is not a line. A picked repository draws its own. All draws
 * the one repository that has a line, and none where several do, rather than
 * one of them unnamed.
 */
export function mergeLineView(lines: MergeLines | null, picked: string | null): MergeLineView | undefined {
  const waiting = (lines?.lines ?? []).filter((one) => one.line.length > 0);
  const chosen = picked === null ? (waiting.length === 1 ? waiting[0] : undefined) : waiting.find((one) => one.root === picked);
  if (chosen === undefined) return undefined;
  return { line: chosen.line.map(entryOf), off: chosen.off.map(entryOf) };
}

function entryOf(row: MergeLineRow): MergeLineEntry {
  return {
    branch: row.branch,
    place: row.place,
    pr: row.pull_request,
    state: row.state as MergeLineState,
    doing: row.doing,
    batch: row.batch,
    merge: row.merge_commit?.slice(0, SHORT),
    failed: row.failed,
    conflicts: row.conflicts,
  };
}
