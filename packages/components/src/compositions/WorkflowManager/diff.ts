// A proposed change as the steps it touches, matched by step id: what the Proposed change region draws a card for,
// and the names of the rest. A step that only moves is touched; one that only shifts because another came or went is not.

import type { ManagedStep } from "./types";

/** The four facts a step carries, in the order a card's rows draw them. */
export const STEP_FIELDS = [
  { name: "Name", of: (one: ManagedStep) => one.label },
  // A step with several ends is read as all of them, one a line, so a change to the list is a change.
  { name: "Ends when", of: (one: ManagedStep) => (one.endsAny !== undefined && one.endsAny.length > 1 ? one.endsAny.join("\n") : one.ends) },
  { name: "Passed over when", of: (one: ManagedStep) => one.passedOver ?? "" },
  { name: "Words", of: (one: ManagedStep) => one.words },
] as const;

export type StepFieldName = (typeof STEP_FIELDS)[number]["name"];

/** One step the change touches. `n` is its place in the workflow it is in: after the change, or before it where removed; `was` is a changed step's place before. */
export type StepChange =
  | { kind: "added"; n: number; after: ManagedStep }
  | { kind: "removed"; n: number; before: ManagedStep }
  | { kind: "changed"; n: number; was: number; before: ManagedStep; after: ManagedStep };

const same = (one: ManagedStep, other: ManagedStep) => STEP_FIELDS.every((field) => field.of(one) === field.of(other));

/**
 * The ids of steps present on both sides that moved: those outside the longest run that keeps its order. Where two runs
 * tie, the one keeping the steps that stood earlier stays put, so the later step is the one said to move.
 */
function movedIds(before: readonly ManagedStep[], after: readonly ManagedStep[]): Set<string> {
  const inAfter = new Set(after.map((one) => one.id));
  const common = before.filter((one) => inAfter.has(one.id)).map((one) => one.id);
  const stood = new Map(common.map((id, at) => [id, at]));
  // Where each common step stood before, read in the order they now come.
  const seq = after.filter((one) => stood.has(one.id)).map((one) => stood.get(one.id)!);
  const run: number[] = [];
  const prev: number[] = [];
  // The run ending at a step: longest wins, and at a tie the one through the step that stood earlier.
  const stronger = (one: number, over: number) => over < 0 || run[one]! > run[over]! || (run[one] === run[over] && seq[one]! < seq[over]!);
  seq.forEach((value, at) => {
    let from = -1;
    for (let each = 0; each < at; each++) if (seq[each]! < value && stronger(each, from)) from = each;
    run[at] = from < 0 ? 1 : run[from]! + 1;
    prev[at] = from;
  });
  let end = -1;
  seq.forEach((_, at) => {
    if (stronger(at, end)) end = at;
  });
  const kept = new Set<number>();
  for (let at = end; at >= 0; at = prev[at]!) kept.add(seq[at]!);
  return new Set(common.filter((_, at) => !kept.has(at)));
}

/** The steps a change touches, in the order the workflow reads after it, a removed step where it stood; and the labels of the rest. */
export function stepChangesOf(before: readonly ManagedStep[], after: readonly ManagedStep[]): { changes: StepChange[]; unchanged: string[] } {
  const kept = new Set(after.map((one) => one.id));
  const was = new Map(before.map((one) => [one.id, one]));
  const placeWas = new Map(before.map((one, at) => [one.id, at + 1]));
  const moved = movedIds(before, after);
  // A removed step sits after the last step before it that stays, or first where none does.
  const removedAfter = new Map<string | null, StepChange[]>();
  let anchor: string | null = null;
  before.forEach((one, at) => {
    if (kept.has(one.id)) {
      anchor = one.id;
      return;
    }
    removedAfter.set(anchor, [...(removedAfter.get(anchor) ?? []), { kind: "removed", n: at + 1, before: one }]);
  });
  const changes: StepChange[] = [...(removedAfter.get(null) ?? [])];
  const unchanged: string[] = [];
  after.forEach((one, at) => {
    const had = was.get(one.id);
    if (had === undefined) changes.push({ kind: "added", n: at + 1, after: one });
    else if (moved.has(one.id) || !same(had, one)) changes.push({ kind: "changed", n: at + 1, was: placeWas.get(one.id)!, before: had, after: one });
    else unchanged.push(one.label);
    changes.push(...(removedAfter.get(one.id) ?? []));
  });
  return { changes, unchanged };
}
