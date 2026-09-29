// What Overview leads with on an ordinary Job: one sentence, what it is
// holding up, and one act.
//
// **A thing you can act on outranks a thing you can only watch**, because
// acting may resolve the watching — waiting on you, then stopped, then
// running. The owner's reasoning and its costs are
// `.claude/decisions/2026-09-29-overview-leads-with-what-releases-the-most.md`.
//
// Every design board already leads this way, and only the train and the landed
// Job build it. `members.ts` is this shape for a Job whose members are Jobs,
// and `JobLead.tsx` draws both.

import { CHECK_ADVANCES, JOB_LIFECYCLE } from "@armada/components";
import type { CheckRun, JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";

import { elapsedSince } from "./duration";

/**
 * The line that leads Overview. `waiting` colours the edge, exactly as it does
 * on the train's lead, so the tone is read before a word of it is.
 */
export type JobLead = {
  said: string;
  because: string;
  /** Colours the edge. One of the state machine's own hues, never a fourth. */
  tone?: "awaiting-review" | "completed-failed";
  /** The one act, where the screen has somewhere to send a person. */
  act?: string;
};

/** The step a person is being asked about, or the one a Drone is on. */
function currentStep(whole: JobWhole | null): StepDetail | undefined {
  if (whole === null) return undefined;
  return (
    whole.steps.find((step) => step.state === "awaiting_human") ??
    whole.steps.find((step) => step.state === "running" || step.state === "retrying") ??
    whole.steps.find((step) => step.state === "stopped")
  );
}

/**
 * What a step holds up: the steps after it that have not started.
 *
 * **This is the whole of what Armada can say about consequence today.** A step
 * boundary is the one release Fleet records — a Drone's question is asked
 * mid-step and nothing writes down what is behind the answer, so two things
 * waiting take the same tail rather than an invented difference.
 */
function holdsUp(whole: JobWhole | null, step: StepDetail | undefined): string {
  if (whole === null || step === undefined) return "";
  const after = whole.steps.filter(
    (one) => one.ordinal > step.ordinal && one.state === "not_started",
  );
  const next = after[0];
  if (next === undefined) return "Nothing in this workflow is behind it.";
  return after.length === 1
    ? `${next.label} does not start until you answer.`
    : `${next.label} and ${after.length - 1} more do not start until you answer.`;
}

/** How far the plan is through, where there is one to be through. */
function tasksSaid(whole: JobWhole | null): string {
  const tasks = whole?.work_plan?.tasks.filter((task) => task.state !== "dropped") ?? [];
  if (tasks.length === 0) return "";
  const done = tasks.filter((task) => task.state === "done").length;
  return `${done} of ${tasks.length} ${tasks.length === 1 ? "task is" : "tasks are"} through.`;
}

/**
 * The step later in the workflow that will stop for a person.
 *
 * **Which step is known and when it arrives is not**, so the line counts steps
 * and never minutes — the workflow is frozen onto the Job, and nothing on it
 * estimates a time.
 *
 * **Nothing, where no step ahead declares a gate at all.** An older Fleet
 * serves no `advance_gate`, and *no step after this one stops for you* would
 * be a claim read off an absent field rather than off a declaration.
 */
function nextGateSaid(whole: JobWhole | null, step: StepDetail | undefined): string {
  if (whole === null || step === undefined) return "";
  const ahead = whole.steps.filter((one) => one.ordinal > step.ordinal);
  if (!ahead.some((one) => one.advance_gate !== undefined)) return "";
  const away = ahead.findIndex((one) => one.advance_gate === "human_always") + 1;
  const at = away === 0 ? undefined : ahead[away - 1];
  if (at === undefined) return "No step after this one stops for you.";
  return `${at.label} is the step that asks you, ${away} ${away === 1 ? "step" : "steps"} away.`;
}

/**
 * A Check on this step's latest run that did not advance it.
 *
 * **`CHECK_ADVANCES` and never a word typed here.** `skipped` measured
 * nothing and advances; `signalled` and `timed_out` are as red as `failed`.
 * The table is generated from `check-outcomes.toml`.
 */
function checkThatFailed(step: StepDetail | undefined): CheckRun | undefined {
  const runs = step?.check_runs ?? [];
  const latest = runs.reduce((at, run) => Math.max(at, run.attempt), 0);
  return runs.find((run) => run.attempt === latest && CHECK_ADVANCES[run.outcome] === false);
}

/**
 * Two clauses at most, each ended. **Fleet's own words end nothing** —
 * `CheckRun.produced` is a log line, so a clause taken from the wire runs
 * straight into the next one without this.
 */
function because(...parts: string[]): string {
  return parts
    .filter((part) => part !== "")
    .map((part) => (/[.!?]$/.test(part) ? part : `${part}.`))
    .join(" ");
}

/**
 * What leads Overview, for a Job that is neither a train nor landed.
 *
 * **Who is acting is read, never listed.** `JOB_LIFECYCLE` is generated from
 * the Rust registry and already says whether a person or a Drone moves a Job
 * next; a roster of statuses typed here would drift the first time Fleet adds
 * one, which is `frozen.ts`'s rule for terminality.
 *
 * **A screen never says a thing it does not know**, so the quiet lead reads
 * *nothing needs you* rather than inventing urgency out of a Job working.
 */
export function leadOf(job: JobSummary, whole: JobWhole | null, now: number): JobLead {
  const step = currentStep(whole);
  const label = step?.label ?? "this Job";
  const held = () => because(holdsUp(whole, step), tasksSaid(whole));

  // Waiting on you. The Drone's own question first: it is the one thing on the
  // wire saying a Drone has stopped and is holding its turn open.
  if (whole?.asking !== undefined) {
    return { said: "The Drone asked you something.", because: held(), tone: "awaiting-review", act: "Answer it" };
  }
  if (whole?.command_waiting !== undefined) {
    return {
      said: "The Drone wants to run a command it was not given.",
      because: held(),
      tone: "awaiting-review",
      act: "Decide it",
    };
  }
  const lifecycle = JOB_LIFECYCLE[job.status];
  if (lifecycle?.whoIsActing === "Person" && lifecycle.mode === "Waited on") {
    return { said: `${label} is waiting on you.`, because: held(), tone: "awaiting-review", act: "Open it" };
  }

  // Stopped, second — answering something that was waiting may already have
  // been what cleared it, which is the owner's reason for this order.
  if (lifecycle?.terminal === true && job.status !== "completed_success") {
    return {
      said: `This Job stopped at ${label}.`,
      because: because(tasksSaid(whole), "Nothing else on it is running."),
      tone: "completed-failed",
      act: "Read what stopped it",
    };
  }

  // A Check that went red, wherever the Job's status has it. **This is the
  // complaint the reframe came from** — a failed group was legible only by
  // opening the step, so a Job at `running` said nothing was wrong.
  const failed = checkThatFailed(step);
  if (failed !== undefined) {
    return {
      said: `${failed.name} failed on ${label}.`,
      because: because(
        failed.produced ?? "",
        step?.state === "running" || step?.state === "retrying"
          ? "The step is running again."
          : "The step has not advanced.",
      ),
      tone: "completed-failed",
      act: "Read what it produced",
    };
  }

  // Over, before running: a Job that finished has no running step to name and
  // would otherwise fall through to the quiet line.
  if (lifecycle?.terminal === true) {
    return { said: "Done. Nothing is waiting on you.", because: tasksSaid(whole) };
  }

  // Running — the board's own quiet line, `Narrow`: what is running leads, and
  // what will next need a person is the clause under it.
  const on = elapsedSince(step?.entered_at, now);
  return {
    said:
      step === undefined
        ? "Nothing needs you."
        : `On ${label}${on === undefined ? "" : `, ${on} in`}. Nothing needs you.`,
    because: because(tasksSaid(whole), nextGateSaid(whole, step)),
  };
}
