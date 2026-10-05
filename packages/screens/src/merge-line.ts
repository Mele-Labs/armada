// The merge line: the branches waiting to land on main, the ones that just landed, and the ones
// sent back, as Overview's panels and the rail's Merge line surface draw them.
//
// Fleet serves it since protocol 22.1 — `get_merge_lines`, kept current by
// `merge_lines.changed` — reading what `armada land` keeps on disk. `landed` and `sent_back` since
// 23.1. This is the one fold from that wire onto the composition's rows.

import type { MergeLineCheck, MergeLineEntry, MergeLineNotice, MergeLineState, MergeLineWaiting } from "@armada/components";
import type { MergeLine, MergeLineRow, MergeLines, RepositorySummary } from "@armada/protocol";
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
  /** The turn's failed Check, while it runs on. */
  notice?: MergeLineNotice;
};

/**
 * A line as the mock serves it ahead of the wire: `notice` is not a field of protocol 23 yet, so a
 * Fleet sends none and nothing draws. Folded here so the panel can be walked before it is built.
 */
type NoticedLine = MergeLine & { notice?: MergeLineNotice };

/** A row as the mock serves it ahead of the wire: `why`, the reason a waiting branch is not in the turn. */
type ReasonedRow = MergeLineRow & { why?: MergeLineWaiting };

/**
 * The line in the order it will merge: the turn that is running first, in place order, then what
 * waits, in place order, which is the order the next turn takes. **Each row's number is its
 * position here**, not the place it joined at, which a branch keeps after a red.
 */
function inOrder(rows: readonly MergeLineRow[]): MergeLineRow[] {
  const ahead = (row: MergeLineRow) => (row.state === "waiting" ? 1 : 0);
  return rows
    .map((row, at) => ({ row, at }))
    .sort((a, b) => ahead(a.row) - ahead(b.row) || a.at - b.at)
    .map(({ row }, at) => ({ ...row, place: at + 1 }));
}

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
    line: inOrder(one.line).map(entryOf),
    landed: one.landed.map(entryOf),
    sentBack: one.sent_back.map(entryOf),
    ...((one as NoticedLine).notice === undefined ? {} : { notice: (one as NoticedLine).notice }),
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
    why: (row as ReasonedRow).why,
    doing: row.doing,
    batch: row.batch,
    merge: row.merge_commit?.slice(0, SHORT),
    failed: row.failed,
    conflicts: row.conflicts,
    checks: row.checks?.map((check) => ({ name: check.name, state: check.state as MergeLineCheck["state"] })),
  };
}
