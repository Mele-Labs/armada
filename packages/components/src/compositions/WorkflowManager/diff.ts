// A proposed change as the steps it touches, matched by step id: what the Proposed change region draws a card for,
// and the names of the rest.

import type { ManagedStep } from "./types";

/** The four facts a step carries, in the order a card's rows draw them. */
export const STEP_FIELDS = [
  { name: "Name", of: (one: ManagedStep) => one.label },
  { name: "Ends when", of: (one: ManagedStep) => one.ends },
  { name: "Passed over when", of: (one: ManagedStep) => one.passedOver ?? "" },
  { name: "Words", of: (one: ManagedStep) => one.words },
] as const;

export type StepFieldName = (typeof STEP_FIELDS)[number]["name"];

/** One step the change touches. `n` is its place in the workflow it is in: after the change, or before it where removed. */
export type StepChange =
  | { kind: "added"; n: number; after: ManagedStep }
  | { kind: "removed"; n: number; before: ManagedStep }
  | { kind: "changed"; n: number; before: ManagedStep; after: ManagedStep };

const same = (one: ManagedStep, other: ManagedStep) => STEP_FIELDS.every((field) => field.of(one) === field.of(other));

/** The steps a change touches, in the order the workflow reads after it, a removed step where it stood; and the labels of the rest. */
export function stepChangesOf(before: readonly ManagedStep[], after: readonly ManagedStep[]): { changes: StepChange[]; unchanged: string[] } {
  const kept = new Set(after.map((one) => one.id));
  const was = new Map(before.map((one) => [one.id, one]));
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
    else if (!same(had, one)) changes.push({ kind: "changed", n: at + 1, before: had, after: one });
    else unchanged.push(one.label);
    changes.push(...(removedAfter.get(one.id) ?? []));
  });
  return { changes, unchanged };
}
