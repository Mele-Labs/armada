// Where a running Job is on each node of its canvas: the steps it has been
// through, the one it is on, and the plan's groups once Plan has made them —
// the canvas drawn as the Overview for the Job's whole life (the owner,
// 4 Oct 2026, prototype).
//
// **Every word and glyph is a registry's.** A step reads `STEP_STATE`, a group
// `GROUP_STATE` and a dispatched Job `JOB_STATUS`, through the card's own mark
// and its tooltip; nothing here writes a status phrase.

import { GROUP_STATE, JOB_STATUS, STEP_STATE } from "@armada/components";
import type { StepActivity } from "@armada/components";
import type { JobDetail as JobWhole } from "@armada/protocol";

import type { GroupRead, LifeRead, MemberRead, NodeLife } from "./approval-canvas";
import { taskGroupsOf } from "./draft/group";
import type { GroupState } from "./draft/group";
import type { WaveView } from "./draft/wave";

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

/**
 * A dispatched Job's status on the card's mark — its sweep, and whether it is
 * done. The glyph, hue and word are `JOB_STATUS`'s row.
 */
const JOB_ACTIVITY: Readonly<Record<string, StepActivity>> = {
  running: "running",
  proposing: "running",
  completed_success: "advanced",
  completed_failed: "failed",
  killed: "killed",
  awaiting_review: "awaiting_human",
  awaiting_approval: "awaiting_human",
  awaiting_repair: "awaiting_human",
  awaiting_attestation: "awaiting_human",
  escalated: "awaiting_human",
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

/** The wave's Jobs on its live pass — an earlier pass is history a loop return replaced. */
function membersOf(wave: WaveView | undefined): MemberRead[] {
  const live = wave?.rounds.find((round) => round.live)?.round;
  return (wave?.jobs ?? [])
    .filter((job) => live === undefined || job.round === live)
    .map((job) => {
      const row = JOB_STATUS[job.status];
      return {
        id: job.job,
        name: job.title,
        waits_on: job.waits_on,
        life: {
          activity: JOB_ACTIVITY[job.status] ?? "not_started",
          said: row?.verb ?? job.status,
          ...(row?.icon && row.statusToken ? { mark: { icon: row.icon, token: row.statusToken } } : {}),
        },
      };
    });
}

export function lifeOf(whole: JobWhole, wave?: WaveView): LifeRead {
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
  const jobs = membersOf(wave);
  return {
    nodes,
    ...(groups.length === 0 ? {} : { groups }),
    ...(jobs.length === 0 ? {} : { jobs }),
  };
}
