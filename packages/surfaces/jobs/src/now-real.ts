// What the Now panel draws for a Job on a real Fleet, from what Fleet already serves: the
// Drones `list_job_drones` names, the Checks and Judge call the open step carries, and the
// questions the detail holds. The events that move these (`drone.spawned`, `job.checking`,
// `job.judging`, `job.asking`) re-read the detail, so this is a function of the latest read and
// holds nothing between calls.
//
// **Only what is served.** The Plan interview and a Drone's sketch stay on the draft; a row here
// never stands in for one of them. A Drone's line is the activity `list_job_drones` carries, else
// its transcript; Issues and Waiting are `JobDetail.issues` and `waiting`.

import { BUDGET_HOLD, CHECK_ADVANCES, JOB_LIFECYCLE, QUEUED_REASON } from "@armada/components";
import type { CheckUnderway, JobDetail, JobIssue, JobSummary, JobWaiting, StepDetail } from "@armada/protocol";
import { span } from "@armada/screens/src/duration";

import type { DroneView } from "./draft/drone";
import { isWorking } from "./draft/drone";
import type { NowAskView, NowIssueView, NowRunningView, NowView, NowWaitingView } from "./draft/now";
import { basename } from "./phases";
import { entriesOf } from "./story";

/** The two a Job shows no panel at: nothing has run, and the approval draws its own canvas. */
const BEFORE_THE_RUN = new Set(["awaiting_approval", "proposing"]);

/** A quick act on a Check row: `retry_check` and `skip_check`. */
const CHECK_ACTS = [
  { key: "retry", glyph: "retry", said: "Retry now" },
  { key: "skip", glyph: "skip", said: "Skip check" },
] as const;

const RETRY_STEP = { key: "restart", glyph: "retry_step", said: "Retry step" } as const;

/** What fixes a stuck Drone: a redirect it is told, or its step run again. */
const STUCK_ACTS = [{ key: "redirect", glyph: "redirect", said: "Redirect Drone" }, RETRY_STEP] as const;

/** How many of a Drone's last calls its tail holds. */
const TAIL = 3;

export type NowRealInput = {
  whole: JobDetail | null;
  /** `droneViewsOf`, which holds the Job's own Drone before the list is read. */
  drones: readonly DroneView[];
  /** The Board, for the title of a Job this one waits on. */
  board?: readonly JobSummary[];
  /** Epoch milliseconds a quiet Drone is measured to. Absent is the clock. */
  at?: number;
};

/**
 * The panel's data for one Job, or `undefined` where it draws no panel: the detail is unread, the
 * Job is over, or it has not started. **A Job with nothing in any of the four lists returns `{}`**,
 * which draws "Nothing is actively running on this job" rather than no panel, so a live Job is
 * never mistaken for one the panel forgot.
 */
export function nowViewOf({ whole, drones, board = [], at = Date.now() }: NowRealInput): NowView | undefined {
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

  // What Fleet names it waits on; the queued row's own reason for a wait Fleet leaves unnamed, such as a pause.
  const served = (whole.waiting ?? []).map((one) => waitingRow(one, board, stepOf, placed));
  const waiting = served.length === 0 ? waitingOf(whole.job, board) : served;
  const issues = (whole.issues ?? []).map((one) => issueRow(one, whole, drones, droneName, placed, at));
  return {
    ...(asks.length === 0 ? {} : { asks }),
    ...(issues.length === 0 ? {} : { issues }),
    ...(running.length === 0 ? {} : { running }),
    ...(waiting.length === 0 ? {} : { waiting }),
  };
}

function droneRow(drone: DroneView, name: string, step: { id: string; name: string } | undefined): NowRunningView {
  // What Fleet served first, else its own calls from the transcript in hand, newest last. A Drone
  // that has said nothing draws no line, never an empty one.
  const calls = entriesOf(drone.transcript ?? [], undefined).flatMap((row) => (row.called === undefined ? [] : [`${row.called.tool} ${row.called.detail}`.trim()]));
  const tail = drone.activity === undefined ? calls.slice(-TAIL) : (drone.activity.tail ?? [drone.activity.line]).slice(-TAIL);
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

/** One entry of `JobDetail.waiting` as a row. A resource is spelled the way the queued reason is. */
function waitingRow(
  one: JobWaiting,
  board: readonly JobSummary[],
  stepOf: (id: string) => StepDetail | undefined,
  placed: (id: string) => { id: string; name: string } | undefined,
): NowWaitingView {
  if (one.kind === "job") return { key: `job:${one.job_id}`, kind: "job", text: one.title ?? board.find((row) => row.id === one.job_id)?.title ?? one.job_id, target: one.job_id };
  if (one.kind === "transition") {
    const step = placed(one.step_id);
    return { key: `transition:${one.step_id}`, kind: "transition", text: stepOf(one.step_id)?.label ?? one.step_id, ...(step === undefined ? {} : { step }) };
  }
  const said = BUDGET_HOLD[one.name]?.verb ?? QUEUED_REASON[one.name]?.verb ?? one.name;
  return { key: `held:${one.name}`, kind: "resource", text: said.charAt(0).toUpperCase() + said.slice(1) };
}

/** One entry of `JobDetail.issues` as a row. */
function issueRow(
  one: JobIssue,
  whole: JobDetail,
  drones: readonly DroneView[],
  droneName: (drone: DroneView) => string,
  placed: (id: string) => { id: string; name: string } | undefined,
  at: number,
): NowIssueView {
  switch (one.kind) {
    case "breakage": {
      const claim = whole.breakages?.find((row) => row.check === one.check && row.test === one.test);
      return { key: `breakage:${one.check}:${one.test}`, of: "check", name: one.check, text: claim?.failure ?? one.test, said: "Test broken on main" };
    }
    case "write_scope_overlap": {
      const peer = whole.write_scope_overlaps?.find((row) => row.job_id === one.job_id);
      return {
        key: `overlap:${one.job_id}`,
        of: "drone",
        name: peer?.title ?? one.job_id,
        text: peer === undefined ? "Writes where this Job writes" : peer.paths.map((path) => path.path).join(", "),
        said: "Write scope overlaps",
      };
    }
    case "failed_verdict": {
      const step = placed(one.step_id);
      return {
        key: `verdict:${one.step_id}:${one.attempt}`,
        of: "judge",
        name: `Judge on ${step?.name ?? one.step_id}`,
        text: one.trigger === undefined ? `Attempt ${one.attempt} failed` : `Attempt ${one.attempt} failed: ${one.trigger}`,
        said: "Verdict failed",
        ...(step === undefined ? {} : { step }),
        acts: [RETRY_STEP],
      };
    }
    case "stuck_drone": {
      const drone = drones.find((row) => row.id === one.drone_id);
      const step = placed(one.step_id);
      const quiet = span(one.quiet_since, at);
      return {
        key: `stuck:${one.drone_id}`,
        of: "drone",
        name: drone === undefined ? `${step?.name ?? one.step_id} Drone` : droneName(drone),
        text: quiet === null ? "No output" : `No output for ${quiet}`,
        said: "Drone stuck",
        target: one.drone_id,
        ...(step === undefined ? {} : { step }),
        acts: STUCK_ACTS,
      };
    }
  }
}
