// Where a running Job is on each node of its canvas: the steps it has been
// through, the one it is on, and the plan's groups once Plan has made them —
// the canvas drawn as the Overview for the Job's whole life (the owner,
// 4 Oct 2026, prototype).
//
// **Every word and glyph is a registry's.** A step reads `STEP_STATE`, a group
// `GROUP_STATE` and a dispatched Job `JOB_STATUS`, through the card's own mark
// and its tooltip; nothing here writes a status phrase.

import { GROUP_STATE, JOB_STATUS, STEP_STATE } from "@armada/components";
import type { GateCommandOutcome, PanelMark, StepActivity } from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import type { ChangedSince, GateRun, LifeRead, MemberRead, NodeLife, PlanGroupRead } from "./approval-canvas";
import { WHEN_BLOCKED_LABEL, WHEN_REFUSED_LABEL } from "@armada/screens/src/copy";
import { onlyCurrentAttempt } from "@armada/screens/src/facts";
import { REFUSED_STARTS_AT, STARTS_AT } from "./settings";
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

/** What a person moved since the approval: the Settings tab's own count, read field by field. */
function changedOf(whole: JobWhole): ChangedSince | undefined {
  const changed: ChangedSince = {
    ...(whole.model_override === undefined ? {} : { model: whole.model_override }),
    ...(whole.review_model_override === undefined ? {} : { reviewModel: whole.review_model_override }),
    ...(whole.when_blocked === undefined || whole.when_blocked === STARTS_AT
      ? {}
      : { whenBlocked: { now: WHEN_BLOCKED_LABEL[whole.when_blocked], was: WHEN_BLOCKED_LABEL[STARTS_AT] } }),
    ...(whole.when_refused === undefined || whole.when_refused === REFUSED_STARTS_AT
      ? {}
      : { whenRefused: { now: WHEN_REFUSED_LABEL[whole.when_refused], was: WHEN_REFUSED_LABEL[REFUSED_STARTS_AT] } }),
    ...((whole.allowed_commands?.length ?? 0) === 0
      ? {}
      : { allowed: (whole.allowed_commands ?? []).map((one) => one.run) }),
  };
  return Object.keys(changed).length === 0 ? undefined : changed;
}

/** `4m 12s`, as the Workflow card says how long. */
function sinceOf(from: string | undefined, now: number): string | undefined {
  const at = from === undefined ? NaN : Date.parse(from);
  if (Number.isNaN(at)) return undefined;
  const seconds = Math.max(0, Math.floor((now - at) / 1000));
  const [h, m, s] = [Math.floor(seconds / 3600), Math.floor((seconds % 3600) / 60), seconds % 60];
  return h > 0 ? `${h}h ${m}m` : m > 0 ? `${m}m ${s}s` : `${s}s`;
}

const OUTCOME_OF: Readonly<Record<string, GateCommandOutcome>> = { passed: "passed", running: "running", waiting: "waiting", never_ran: "waiting", skipped: "waiting" };

/** A gate stage's life, in the step machine's states. Absent is a stage nothing reached. */
const stageLife = (activity: StepActivity, run?: GateRun): NodeLife => ({
  ...stepLife(activity === "failed" ? "stopped" : activity, false),
  activity,
  ...(activity === "failed" ? { said: "failed" } : {}),
  ...(run === undefined ? {} : { run }),
});

/**
 * Where each stage of a step's gate is, off what Fleet serves of the step's
 * latest run: its Checks, the Judge's answers by member, and whether it is
 * held for a person. A stage nothing reached is absent, which draws upcoming.
 */
function gatesOf(step: StepDetail, now: number): Record<string, NodeLife> {
  const out: Record<string, NodeLife> = {};
  const advanced = step.state === "advanced";
  const attempts = step.attempts ?? [];
  const latest = attempts.reduce((most, one) => Math.max(most, one.attempt), 1);
  const started = attempts.find((one) => one.attempt === latest)?.started_at;
  // The latest attempt with Check rows, not the step's latest attempt: asking
  // the gate again records an attempt without re-running the Checks, and the
  // Checks box would otherwise read as never run beside a panel that says passed.
  const runs = onlyCurrentAttempt(step.check_runs ?? []);
  const declared = step.checks ?? [];
  let checksPassed = declared.length === 0 || advanced;
  if (declared.length > 0) {
    const outcomes = runs.map((one) => OUTCOME_OF[one.outcome] ?? "failed");
    if (advanced) {
      out[`${step.step_id}:checks`] = PAST;
    } else if (runs.length > 0) {
      const failed = outcomes.includes("failed");
      const running = outcomes.includes("running") || runs.length < declared.length;
      checksPassed = !failed && !running;
      const live = runs.find((one) => one.outcome === "running");
      const elapsed = sinceOf(started, now);
      const run: GateRun = {
        commands: runs.map((one, at) => ({ name: one.name, outcome: outcomes[at]! })),
        ...(elapsed === undefined ? {} : { elapsed }),
        ...(live?.produced === undefined ? {} : { output: live.produced }),
      };
      out[`${step.step_id}:checks`] = stageLife(running ? "running" : failed ? "failed" : "advanced", run);
    }
  }
  const panel = step.judge_checks?.[0]?.panel_size ?? 1;
  if ((step.judge_checks ?? []).length > 0) {
    const answers = (step.judged ?? []).filter((one) => one.attempt === latest);
    const marks: PanelMark[] = Array.from({ length: panel }, (_, member): PanelMark => {
      const mine = answers.filter((one) => (one.member ?? 0) === member);
      if (mine.length === 0) return advanced ? "met" : "pending";
      return mine.some((one) => one.verdict !== "met") ? "not_met" : "met";
    });
    const refused = answers.find((one) => one.verdict !== "met");
    const landed = marks.filter((one) => one !== "pending").length;
    if (advanced) out[`${step.step_id}:judge`] = PAST;
    else if (refused !== undefined) {
      const first = (refused.consequence ?? refused.produced ?? "").split("\n")[0] ?? "";
      out[`${step.step_id}:judge`] = stageLife("failed", { panel: marks, ...(first === "" ? {} : { refusal: first }) });
    } else if (landed > 0 || (declared.length > 0 && runs.length > 0 && checksPassed)) {
      const reading = marks.map((one): PanelMark => (one === "pending" ? "judging" : one));
      out[`${step.step_id}:judge`] = stageLife(landed === panel ? "advanced" : "running", { panel: reading });
    }
  }
  if (step.advance_gate === "human_always") {
    if (advanced) out[`${step.step_id}:you`] = PAST;
    else if (step.state === "awaiting_human") {
      const waited = sinceOf(attempts.find((one) => one.attempt === latest)?.ended_at ?? started, now);
      out[`${step.step_id}:you`] = stageLife("awaiting_human", waited === undefined ? {} : { waited });
    }
  }
  return out;
}

export function lifeOf(
  whole: JobWhole,
  wave?: WaveView,
  /** Each step's line as the Workflow card draws it, by step id. */
  lines?: Readonly<Record<string, string>>,
  now: number = Date.now(),
): LifeRead {
  const nodes: Record<string, NodeLife> = { studio: PAST, brief: PAST, base: PAST };
  for (const step of whole.steps) {
    const current = step.step_id === whole.job.current_step_id && AT.has(step.state);
    nodes[step.step_id] = stepLife(step.state, current);
    // Each stage of a step's gate, where it has got to.
    Object.assign(nodes, gatesOf(step, now));
  }
  const served = whole.work_plan === undefined ? [] : taskGroupsOf(whole);
  const groups: PlanGroupRead[] = served.map((group) => {
    const row = GROUP_STATE[group.state];
    const activity = GROUP_ACTIVITY[group.state];
    return {
      id: group.id,
      name: `Group ${group.ordinal}`,
      tasks: group.tasks,
      life: {
        activity,
        said: row?.verb ?? group.state,
        ...(row?.icon && row.statusToken ? { mark: { icon: row.icon, token: row.statusToken } } : {}),
        ...(activity === "running" || activity === "retrying" ? { current: true } : {}),
      },
    };
  });
  const jobs = membersOf(wave);
  const changed = changedOf(whole);
  return {
    nodes,
    ...(changed === undefined ? {} : { changed }),
    ...(lines === undefined ? {} : { lines }),
    ...(groups.length === 0 ? {} : { groups }),
    ...(jobs.length === 0 ? {} : { jobs }),
  };
}
