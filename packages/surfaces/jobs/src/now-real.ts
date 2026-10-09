// What the Now panel draws for a Job on a real Fleet, from what Fleet already serves: the
// Drones `list_job_drones` names, the Checks and Judge call the open step carries, and the
// questions the detail holds. The events that move these (`drone.spawned`, `job.checking`,
// `job.judging`, `job.asking`) re-read the detail, so this is a function of the latest read and
// holds nothing between calls.
//
// **Only what is served.** The Plan interview, issues, a Drone's sketch and a Waiting source that
// is not a queued reason stay on the draft; a row here never stands in for one of them.

import { BUDGET_HOLD, CHECK_ADVANCES, JOB_LIFECYCLE, QUEUED_REASON } from "@armada/components";
import type { CheckUnderway, JobDetail, JobSummary, StepDetail } from "@armada/protocol";

import type { DroneView } from "./draft/drone";
import { isWorking } from "./draft/drone";
import type { NowAskView, NowRunningView, NowView, NowWaitingView } from "./draft/now";
import { basename } from "./phases";
import { entriesOf } from "./story";

/** The two a Job shows no panel at: nothing has run, and the approval draws its own canvas. */
const BEFORE_THE_RUN = new Set(["awaiting_approval", "proposing"]);

/** A quick act on a Check row. The host says what it would do; no Fleet command is behind either. */
const CHECK_ACTS = [
  { key: "retry", glyph: "retry", said: "Retry now" },
  { key: "skip", glyph: "skip", said: "Skip check" },
] as const;

/** How many of a Drone's last calls its tail holds. */
const TAIL = 3;

export type NowRealInput = {
  whole: JobDetail | null;
  /** `droneViewsOf`, which holds the Job's own Drone before the list is read. */
  drones: readonly DroneView[];
  /** The Board, for the title of a Job this one waits on. */
  board?: readonly JobSummary[];
};

/**
 * The panel's data for one Job, or `undefined` where it draws no panel: the detail is unread, the
 * Job is over, or it has not started. **A Job with nothing in any of the four lists returns `{}`**,
 * which draws "Nothing is actively running on this job" rather than no panel, so a live Job is
 * never mistaken for one the panel forgot.
 */
export function nowViewOf({ whole, drones, board = [] }: NowRealInput): NowView | undefined {
  if (whole === null) return undefined;
  const status = whole.job.status;
  if (JOB_LIFECYCLE[status]?.terminal !== false || BEFORE_THE_RUN.has(status)) return undefined;

  const stepOf = (id: string): StepDetail | undefined => whole.steps.find((one) => one.step_id === id);
  const placed = (id: string) => {
    const label = stepOf(id)?.label;
    return label === undefined ? undefined : { id, name: label };
  };
  const droneName = (drone: DroneView): string => {
    const label = stepOf(drone.step)?.label ?? drone.step;
    return drone.task === undefined ? `${label} Drone` : `${label} Drone on ${drone.task}`;
  };

  // **Only a `running` Job has a Drone at work.** The Job's own Drone is read off `assigned_drone`
  // while the list is unread, and a Job held at a gate keeps that field with its Drone at rest.
  const working = whole.job.status === "running" ? drones.filter(isWorking) : [];
  const running: NowRunningView[] = [
    ...working.map((drone) => droneRow(drone, droneName(drone), placed(drone.step))),
    ...whole.steps.flatMap(checkRows(placed)),
    ...whole.steps.flatMap((step): NowRunningView[] =>
      step.judging === undefined
        ? []
        : [{ key: `judge:${step.step_id}`, of: "judge", name: `Judge on ${step.label}`, state: "running", step: { id: step.step_id, name: step.label } }],
    ),
  ];

  // The Drone asking: the one the wire names, else the only one working on that step.
  const askerOn = (stepId: string, named?: string): DroneView | undefined =>
    named !== undefined
      ? drones.find((one) => one.id === named)
      : (() => {
          const on = working.filter((one) => one.step === stepId);
          return on.length === 1 ? on[0] : undefined;
        })();
  const asks: NowAskView[] = [];
  const judging = whole.judge_question;
  if (judging !== undefined) {
    asks.push({ key: `judge:${judging.step_id}:${judging.criterion_id}`, kind: "judge", name: `Judge on ${stepOf(judging.step_id)?.label ?? judging.step_id}`, text: judging.question });
  }
  const asking = whole.asking;
  if (asking !== undefined) {
    const asker = askerOn(asking.step_id);
    asks.push({
      key: `ask:${asking.question_id}`,
      kind: "drone",
      name: asker === undefined ? `${stepOf(asking.step_id)?.label ?? asking.step_id} Drone` : droneName(asker),
      text: asking.question,
      ...(asker === undefined ? {} : { target: asker.id }),
    });
  }
  const command = whole.command_waiting;
  if (command !== undefined) {
    const asker = askerOn(command.step_id, command.drone_id);
    asks.push({
      key: `command:${command.call}`,
      kind: "drone",
      name: asker === undefined ? `${stepOf(command.step_id)?.label ?? command.step_id} Drone` : droneName(asker),
      text: command.detail === "" ? command.tool : `${command.tool} ${command.detail}`,
      ...(asker === undefined ? {} : { target: asker.id }),
    });
  }

  const waiting = waitingOf(whole.job, board);
  return {
    ...(asks.length === 0 ? {} : { asks }),
    ...(running.length === 0 ? {} : { running }),
    ...(waiting.length === 0 ? {} : { waiting }),
  };
}

function droneRow(drone: DroneView, name: string, step: { id: string; name: string } | undefined): NowRunningView {
  // Its own calls, newest last. A transcript the window has not read draws no line, never an empty one.
  const calls = entriesOf(drone.transcript ?? [], undefined).flatMap((row) => (row.called === undefined ? [] : [`${row.called.tool} ${row.called.detail}`.trim()]));
  const tail = calls.slice(-TAIL);
  return {
    key: `drone:${drone.id}`,
    of: "drone",
    name,
    state: "running",
    target: drone.id,
    ...(step === undefined ? {} : { step }),
    ...(tail.length === 0 ? {} : { line: tail[tail.length - 1]!, tail }),
  };
}

/** One step's Checks as the gate has them: running until `ran` lands, then passed or failed. */
function checkRows(placed: (id: string) => { id: string; name: string } | undefined) {
  return (step: StepDetail): NowRunningView[] =>
    (step.checking?.checks ?? []).flatMap((check: CheckUnderway): NowRunningView[] => {
      // A Check still waiting for a place is not running; a skipped one advanced and measured nothing.
      if (check.started_at === undefined || check.ran?.outcome === "skipped") return [];
      const state = check.ran === undefined ? "running" : CHECK_ADVANCES[check.ran.outcome] === true ? "passed" : "failed";
      const produced = check.ran?.produced;
      return [
        {
          key: `check:${step.step_id}:${check.name}`,
          of: "check",
          name: check.name,
          state,
          // The gate's live log, which the socket serves whole once the Check has ended (`plan-board.ts`).
          ...(check.output_path === undefined ? {} : { log: { name: check.name, kept: basename(check.output_path), live: true } }),
          ...(placed(step.step_id) === undefined ? {} : { step: placed(step.step_id)! }),
          ...(state === "failed" && produced !== undefined ? { tail: [produced] } : {}),
          ...(state === "passed" ? {} : { acts: CHECK_ACTS }),
        },
      ];
    });
}

/** Why a queued Job has not started, and who it waits on, as Fleet states them on the row. */
function waitingOf(job: JobSummary, board: readonly JobSummary[]): NowWaitingView[] {
  const reason = job.paused !== undefined ? "paused" : job.status === "queued" ? job.queued_reason : undefined;
  if (reason === undefined) return [];
  if (reason === "blocked_by_dependency") {
    return (job.waits_on ?? []).flatMap((id): NowWaitingView[] => {
      const peer = board.find((one) => one.id === id);
      if (peer !== undefined && JOB_LIFECYCLE[peer.status]?.terminal !== false) return [];
      return [{ key: `job:${id}`, kind: "job", text: peer?.title ?? id, target: id }];
    });
  }
  const said = reason === "over_budget" ? (BUDGET_HOLD[job.budget_hold ?? ""]?.verb ?? QUEUED_REASON[reason]?.verb) : QUEUED_REASON[reason]?.verb;
  return said === undefined || said === null ? [] : [{ key: `held:${reason}`, kind: "resource", text: said.charAt(0).toUpperCase() + said.slice(1) }];
}
