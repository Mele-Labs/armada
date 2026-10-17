// A plan recorded again after work was done, handed to the thread's card as its diff. `viewOf` with no marks of his already names what the
// agent added, renamed and took out; this lays it out per group for the card. Mock scaffolding: `?take=folded` picks the second take.

import type { PlanRevision } from "@armada/components";
import type { WorkPlan } from "@armada/screens/src/draft/sessions";

import { viewOf } from "./plan-marks";

const takeOf = (): PlanRevision["take"] => (new URLSearchParams(window.location.search).get("take") === "folded" ? "folded" : "diff");

/** The card's revision for a plan that is a later version and says what it changed; nothing for a first plan. */
export function planRevision(plan: WorkPlan): { revision?: PlanRevision } {
  const changes = plan.changes;
  if ((plan.version ?? 1) < 2 || changes === undefined || changes.added.length + changes.renamed.length + changes.removed.length === 0) return {};
  const groups = viewOf(plan, []).map((group) => ({
    id: group.id,
    agent: group.agent,
    title: group.title,
    ...(group.since === undefined ? {} : { since: group.since }),
    gone: group.gone,
    tasks: group.tasks.map((task) => ({ id: task.id, text: task.text, state: task.state, ...(task.since === undefined ? {} : { since: task.since }) })),
  }));
  return { revision: { take: takeOf(), groups } };
}
