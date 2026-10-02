// The merge line: the branches waiting to land on main, the ones that just landed, and the ones
// sent back, as Overview's panels and the rail's Merge line surface draw them.
//
// Fleet serves it since protocol 22.1 — `get_merge_lines`, kept current by
// `merge_lines.changed` — reading what `armada land` keeps on disk. `landed` and `sent_back` since
// 23.1. This is the one fold from that wire onto the composition's rows.

import type { MergeLineCheck, MergeLineEntry, MergeLineState } from "@armada/components";
import type { MergeLineRow, MergeLines, RepositorySummary } from "@armada/protocol";
import { repositoryLabel } from "@armada/shell";

import { settledBadgeOf } from "./facts";

/** One repository's line, as a panel takes it. */
export type MergeLineView = {
  /** The repository's root: which panel this is. */
  root: string;
  /** The repository, by its label. Only where more than one panel draws. */
  name?: string;
  /** In place order: `--status`'s numbered rows. */
  line: readonly MergeLineEntry[];
  /** Landed, newest first. */
  landed: readonly MergeLineEntry[];
  /** Red, conflict or stopped and not back in line, newest first. */
  sentBack: readonly MergeLineEntry[];
};

/** How much of a merge commit a row shows. */
const SHORT = 10;

/**
 * The panels to draw for this pick, one per repository, in the order Fleet serves them.
 *
 * **A repository draws wherever Fleet serves it a line**, whoever is in it: an empty line still
 * shows what landed and what was sent back. A picked repository draws its own. All draws every
 * one, each named by its repository once there is more than one. None before Fleet answers, or
 * where it serves no line for the pick.
 */
export function mergeLineViews(
  lines: MergeLines | null,
  picked: string | null,
  repositories: readonly RepositorySummary[],
): readonly MergeLineView[] {
  const served = lines?.lines ?? [];
  const chosen = picked === null ? served : served.filter((one) => one.root === picked);
  const named = chosen.length > 1;
  return chosen.map((one) => ({
    root: one.root,
    ...(named ? { name: nameOf(one.root, repositories) } : {}),
    line: one.line.map(entryOf),
    landed: one.landed.map(entryOf),
    sentBack: one.sent_back.map(entryOf),
  }));
}

/** The repository's label, or its root where Fleet's listing has not caught up with its line. */
function nameOf(root: string, repositories: readonly RepositorySummary[]): string {
  const served = repositories.find((one) => one.root === root);
  return served === undefined ? root : repositoryLabel(served, repositories);
}

/**
 * The state a row draws. **A turn that has run no Check yet is `preparing`**, Bridge's own: Fleet
 * serves `gating` for the whole turn, and its Check list is what tells the two parts apart.
 */
function stateOf(row: MergeLineRow): MergeLineState {
  if (row.state === "gating" && (row.checks ?? []).length === 0) return "preparing";
  return row.state as MergeLineState;
}

/**
 * A row's pull request, with the Job's own badge for how it ended (`settledBadgeOf`), so the merge
 * line and a Job's detail read a settled pull request alike. Nothing known draws the number alone.
 */
function pullRequestOf(pr: NonNullable<MergeLineRow["pull_request"]>): NonNullable<MergeLineEntry["pr"]> {
  const settled = settledBadgeOf(pr.settled);
  return { number: pr.number, url: pr.url, ...(settled === undefined ? {} : { settled }) };
}

function entryOf(row: MergeLineRow): MergeLineEntry {
  return {
    branch: row.branch,
    place: row.place,
    pr: row.pull_request === undefined ? undefined : pullRequestOf(row.pull_request),
    state: stateOf(row),
    doing: row.doing,
    batch: row.batch,
    merge: row.merge_commit?.slice(0, SHORT),
    failed: row.failed,
    conflicts: row.conflicts,
    checks: row.checks?.map((check) => ({ name: check.name, state: check.state as MergeLineCheck["state"] })),
  };
}
