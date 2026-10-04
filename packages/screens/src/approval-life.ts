// Where a running Job is on each node of its canvas: the steps it has been
// through, the one it is on, and the plan's groups once Plan has made them —
// the canvas drawn as the Overview for the Job's whole life (the owner,
// 4 Oct 2026, prototype).
//
// **Every word and glyph is a registry's.** A step reads `STEP_STATE`, a group
// `GROUP_STATE`, through the card's own mark and its tooltip; nothing here
// writes a status phrase.

import { GROUP_STATE, STEP_STATE } from "@armada/components";
import type { StepActivity } from "@armada/components";
import type { JobDetail as JobWhole } from "@armada/protocol";

import type { GroupRead, LifeRead, NodeLife } from "./approval-canvas";
import { taskGroupsOf } from "./draft/group";
import type { GroupState } from "./draft/group";

/** The step machine's states the card's mark draws as they are. */
const STEP_ACTIVITY: ReadonlySet<string> = new Set([
  "not_started",
  "running",
  "awaiting_human",
  "retrying",
  "advanced",
  "stopped",
]);

/** A group's state on the card's mark: its sweep, and done once it passed. */
const GROUP_ACTIVITY: Readonly<Record<GroupState, StepActivity>> = {
  pending: "not_started",
  running: "running",
  joining: "running",
  checking: "running",
  passed: "advanced",
  landed: "advanced",
  failed: "failed",
  retrying: "retrying",
};

/** The step states a Job is *at*, rather than past or short of. */
const AT: ReadonlySet<string> = new Set(["running", "awaiting_human", "retrying", "stopped"]);

const stepLife = (state: string, current: boolean): NodeLife => ({
  activity: STEP_ACTIVITY.has(state) ? (state as StepActivity) : "not_started",
  said: STEP_STATE[state]?.verb ?? state,
  ...(current ? { current: true } : {}),
});

/** Done, in the step machine's word: what Brief, the base and the start read once the Job is past them. */
const PAST = stepLife("advanced", false);

export function lifeOf(whole: JobWhole): LifeRead {
  const nodes: Record<string, NodeLife> = { brief: PAST, base: PAST, start: PAST };
  for (const step of whole.steps) {
    const current = step.step_id === whole.job.current_step_id && AT.has(step.state);
    nodes[step.step_id] = stepLife(step.state, current);
    // A step's gate is behind it once the step advanced past it.
    if (step.state === "advanced") nodes[`${step.step_id}:checks`] = PAST;
  }
  const served = whole.work_plan === undefined ? [] : taskGroupsOf(whole);
  const groups: GroupRead[] = served.map((group) => {
    const row = GROUP_STATE[group.state];
    const activity = GROUP_ACTIVITY[group.state];
    return {
      id: group.id,
      name: `Group ${group.ordinal}`,
      life: {
        activity,
        said: row?.verb ?? group.state,
        ...(row?.icon && row.statusToken ? { mark: { icon: row.icon, token: row.statusToken } } : {}),
        ...(activity === "running" || activity === "retrying" ? { current: true } : {}),
      },
    };
  });
  return groups.length === 0 ? { nodes } : { nodes, groups };
}
