// Landing's pull requests in the order a person reads them: a pull request stacked on another sits under it, so the stack reads parent first.

import type { PullRequest } from "./IssuesBlock";

export type StackRow = {
  pr: PullRequest;
  /** A stacked row draws the lane's line; `first` and `last` end it at the tick. Unstacked rows draw no lane at all. */
  lane: "none" | { first: boolean; last: boolean };
  /** The number of the pull request it is stacked on. */
  parent?: number;
  /** A stack's root that has children: the branch it is based on, drawn as a muted tag. */
  rootBase?: string;
  /** The parent's number, where the parent is not the row directly above. */
  chip?: number;
};

/**
 * The rows for `prs`, which come oldest first, as the ledger holds them.
 * **A pull request is stacked where its `base` is another open pull request's `branch`.** A merged parent has been retargeted to the
 * default branch by the forge, so its children stand alone. Unstacked pull requests and roots keep newest first; children follow their
 * parent depth-first, lowest number first.
 */
export function stackedRows(prs: readonly PullRequest[]): StackRow[] {
  const parentOf = (one: PullRequest): PullRequest | undefined =>
    one.base === undefined ? undefined : prs.find((other) => other !== one && other.state !== "merged" && other.branch !== "" && other.branch === one.base);
  const children = new Map<PullRequest, PullRequest[]>();
  for (const one of prs) {
    const parent = parentOf(one);
    if (parent !== undefined) children.set(parent, [...(children.get(parent) ?? []), one].sort((a, b) => a.number - b.number));
  }
  const out: StackRow[] = [];
  const seen = new Set<PullRequest>();
  const walk = (one: PullRequest, parent: PullRequest | undefined, rows: StackRow[]) => {
    seen.add(one);
    const above = rows[rows.length - 1]?.pr;
    const kids = children.get(one) ?? [];
    rows.push({
      pr: one,
      lane: { first: false, last: false },
      ...(parent === undefined ? {} : { parent: parent.number }),
      ...(parent === undefined && kids.length > 0 && one.base !== undefined ? { rootBase: one.base } : {}),
      ...(parent !== undefined && parent !== above ? { chip: parent.number } : {}),
    });
    for (const kid of kids) if (!seen.has(kid)) walk(kid, one, rows);
  };
  for (const one of [...prs].reverse()) {
    if (parentOf(one) !== undefined || seen.has(one)) continue;
    const stack: StackRow[] = [];
    walk(one, undefined, stack);
    if (stack.length === 1) stack[0] = { ...stack[0]!, lane: "none" };
    else stack.forEach((row, at) => (row.lane = { first: at === 0, last: at === stack.length - 1 }));
    out.push(...stack);
  }
  // Pull requests each on the other have no root: they stand alone.
  for (const one of [...prs].reverse()) if (!seen.has(one)) out.push({ pr: one, lane: "none" });
  return out;
}
