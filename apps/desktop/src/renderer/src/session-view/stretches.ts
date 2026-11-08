// The plan read as gates: each run of groups up to the next gate is one stretch, and the Job is in the first stretch whose gate is open. Fleet places a Job only by its gates, never
// inside a stretch, so a stretch of several groups is named for all of them and lit as one.

import type { GateChecks, GateClose, PlanGate } from "@armada/protocol";
import type { WorkGroup } from "@armada/screens/src/draft/sessions";

export type GateKind = Exclude<PlanGate, "none">;

export type StretchState = "done" | "now" | "next";

/** A stretch's gate as its last group carries it. */
export type StretchGate = {
  kind: GateKind;
  closed: GateClose | undefined;
  checks: GateChecks | undefined;
  /** The Job is held on this gate, waiting on him: since when. */
  held: string | undefined;
};

export type Stretch<G extends WorkGroup = WorkGroup> = {
  key: string;
  /** Every group's title, in plan order: the stretch is named as a whole. */
  name: string;
  groups: readonly G[];
  /** The gate that ends it. Absent on a last stretch that ends on none. */
  gate: StretchGate | undefined;
  state: StretchState;
};

/** Whether the plan carries gates at all. A plan without one keeps the zones it had. */
export const gated = (groups: readonly WorkGroup[]): boolean =>
  groups.some((group) => group.gate !== undefined);

export function stretchesOf<G extends WorkGroup>(
  groups: readonly G[],
  held?: {
    group: string;
    since: string;
  },
): Stretch<G>[] {
  const runs: G[][] = [];
  let run: G[] = [];
  for (const group of groups) {
    run.push(group);
    if (group.gate !== undefined) {
      runs.push(run);
      run = [];
    }
  }
  if (run.length > 0) runs.push(run);
  let live = false;
  return runs.map((one) => {
    const end = one[one.length - 1]!;
    const gate: StretchGate | undefined =
      end.gate === undefined
        ? undefined
        : {
            kind: end.gate,
            closed: end.gateClosed,
            checks: end.gateChecks,
            held: held?.group === end.id && end.gateClosed === undefined ? held.since : undefined,
          };
    const closed = gate?.closed !== undefined;
    const state: StretchState = closed ? "done" : live ? "next" : "now";
    if (!closed) live = true;
    return {
      key: one.map((group) => group.id).join("+"),
      name: one.map((group) => group.title).join(" · "),
      groups: one,
      gate,
      state,
    };
  });
}

/** What each gate waits for, as he reads it on the line and at the foot of its stretch. */
export const GATE_SAID: Record<GateKind, string> = {
  plan_approved: "You approve the plan",
  walk_approved: "You approve the walk",
  sketch_approved: "You approve the sketch",
  document_approved: "You approve the document",
  checks_green: "Checks pass at the pushed commit",
  merged: "Merged into main",
};
