// What Overview leads with on an ordinary Job: one sentence, what it is
// holding up, and one act.
//
// **A thing you can act on outranks a thing you can only watch**, because
// acting may resolve the watching — waiting on you, then stopped, then
// running. The owner's reasoning and its costs are
// *Overview leads with what releases the most*, 29 Sep 2026, in the decisions
// register, has his reasoning and the costs he took.
//
// Every design board already leads this way, and only the train and the landed
// Job build it. `members.ts` is this shape for a Job whose members are Jobs,
// and `JobLead.tsx` draws both.

import { CHECK_ADVANCES, ESCALATION_REASON, JOB_LIFECYCLE } from "@armada/components";
import type { CheckRun, Criterion, JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";

import { didNotPass, didPass, panelsOf } from "./gates";

import { elapsedSince, span } from "./duration";
import { onlyCurrentAttempt } from "./facts";

import type { DetailTab } from "./detail-tabs";
import type { CheckAt } from "./tab-record";

/**
 * Where an act sends a person: a destination, one Check's row on the Record,
 * or the proposal. **The Record's own route** for the second — the same
 * `RecordTabProps.opensCheck` the Plan's group boundary presses through.
 *
 * **The proposal is its own arm because it is not a destination**:
 * `ProposalTab` is what `JobDetail` draws in Overview's place at the approval
 * gate, so there is no tab to select.
 *
 * **Absent where the thing the act names is already under the lead.** A
 * Drone's question and a command it was not given are answered in the lead's
 * own region, not at a destination.
 */
export type LeadOpens = { tab: DetailTab } | { check: CheckAt } | { proposal: true };

/**
 * The line that leads Overview. `waiting` colours the edge, exactly as it does
 * on the train's lead, so the tone is read before a word of it is.
 *
 * **The voice is the owner's, 30 Sep 2026** — the headline names the thing and
 * stops, and the second line carries facts or is empty. *The lead says the
 * thing and stops*, in the decisions register.
 */
export type JobLead = {
  said: string;
  because: string;
  /** Colours the edge. One of the state machine's own hues, never a fourth. */
  tone?: "awaiting-review" | "completed-failed";
  /**
   * How long the thing being asked about has waited, already rendered. Drawn
   * at the lead's top right, since the boxes inside it no longer have a head.
   */
  elapsed?: string;
  /** The one act, where the screen has somewhere to send a person. */
  act?: string;
  /** Where that act goes, where it goes somewhere rather than answering here. */
  opens?: LeadOpens;
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
  // **Nothing, where nothing is behind it.** *Nothing in this workflow is
  // behind it* asserted no fact and filled the line anyway, which is the note
  // the voice came from.
  if (next === undefined) return "";
  return after.length === 1
    ? `${next.label} does not start until you answer`
    : `${next.label} and ${after.length - 1} more do not start until you answer`;
}

/** `its criterion`, `both`, `all 6` — a bare `all 2` reads as a fragment. */
function met(n: number): string {
  return n === 1 ? "its criterion" : n === 2 ? "both criteria" : `all ${n} criteria`;
}

/**
 * What every gate on this Job found, on a Job waiting to be approved. **The
 * positive is what makes a sign-off a sign-off** — without it the line says a
 * step is waiting and nothing about whether the work is any good.
 *
 * **The whole Job's evidence, never the waiting step's alone**, because what is
 * being signed off is the branch and not the step: the delivering step verifies
 * nothing of its own, so the step-local reading left this empty on the one Job
 * a person is most likely to be approving. The owner, 30 Sep 2026, and what
 * honest counting costs is *the lead counts the whole Job's evidence* in the
 * decisions register.
 *
 * **Empty stays empty.** A Job whose steps ran no Check and answered no
 * criterion draws no second line rather than a sentence saying so.
 */
function metSaid(whole: JobWhole | null): string {
  if (whole === null) return "";
  const said = [checksSaid(whole.steps), criteriaSaid(whole)].filter((part) => part !== "");
  return said.join(" and ");
}

/**
 * Every Check this Job measured, and how many held.
 *
 * **One step's latest attempt each**, the reading `checksOf` and
 * `checkThatFailed` take: a Check that failed on attempt 1 and passed on
 * attempt 2 is one Check that passed, and the gate records a row per declared
 * Check per attempt, so the latest is a whole answer.
 *
 * **A skip is out of both figures**, and so is an outcome neither predicate
 * owns — `didPass` says why. A step that never ran needs no filter: it has no
 * runs, and a declared Check nothing ran is not evidence.
 */
function checksSaid(steps: readonly StepDetail[]): string {
  const measured = steps
    .flatMap((step) => onlyCurrentAttempt(step.check_runs))
    .filter((run) => didPass(run) || didNotPass(run));
  const passed = measured.filter(didPass).length;
  if (measured.length === 0) return "";
  // **Nothing Fleet serves today reaches the partial** — a human gate opens
  // only once every tier has held, and an override is refused where a Check
  // failed — and *All 11 Checks passed* over one that did not is the single
  // thing this clause must never be able to say.
  if (passed < measured.length) {
    return `${passed} of ${measured.length} ${measured.length === 1 ? "Check" : "Checks"} passed`;
  }
  return `${passed === 1 ? "Its Check" : passed === 2 ? "Both Checks" : `All ${passed} Checks`} passed`;
}

/**
 * Every criterion this Job's Judges answered, and how many they met.
 *
 * **Criteria, not panels**: `panelsOf` answers per step, and a criterion asked
 * on two steps would otherwise be counted twice.
 *
 * **One refusal anywhere refuses it**, which is the conservative direction and
 * also what keeps an overruled step honest without reading `overridden` — an
 * override leaves the Judge's `not_met` row exactly as it was, so the criterion
 * a person overruled can never count as one the Judge met.
 */
function criteriaSaid(whole: JobWhole): string {
  const answered = new Map<string, boolean>();
  for (const step of whole.steps) {
    for (const panel of panelsOf(step, whole.acceptance_criteria)) {
      // A criterion still being asked is not one the Judge met, either.
      const held = panel.verdict === "met";
      answered.set(panel.criterionId, (answered.get(panel.criterionId) ?? true) && held);
    }
  }
  if (answered.size === 0) return "";
  const held = [...answered.values()].filter((one) => one).length;
  if (held < answered.size) {
    return `the Judge met ${held} of ${answered.size} ${answered.size === 1 ? "criterion" : "criteria"}`;
  }
  return `the Judge met ${met(held)}`;
}

/**
 * How far the plan is through, where there is one to be through.
 *
 * **Three readings, because a plan nobody has started is not `0 of 8`** — the
 * owner, 30 Sep 2026: *"What does it mean to say 0 of 8 tasks are through?
 * Shouldnt it be more like 8 tasks are planned or 0 of 8 tasks started?"*
 */
function tasksSaid(whole: JobWhole | null): string {
  const tasks = whole?.work_plan?.tasks.filter((task) => task.state !== "dropped") ?? [];
  if (tasks.length === 0) return "";
  const done = tasks.filter((task) => task.state === "done").length;
  const working = tasks.filter((task) => task.state === "working").length;
  const word = tasks.length === 1 ? "task" : "tasks";
  if (done > 0) return `${done} of ${tasks.length} ${word} done`;
  return `${tasks.length} ${word} planned, ${working === 0 ? "none" : working} started`;
}

/**
 * The step later in the workflow that will stop for a person.
 *
 * **Which step is known and when it arrives is not**, so the line counts steps
 * and never minutes — the workflow is frozen onto the Job, and nothing on it
 * estimates a time.
 *
 * **Nothing, where none of them asks.** *No step after this one stops for you*
 * is `holdsUp`'s empty case wearing other words — the clause exists to name a
 * gate, and where there is none the line is empty.
 */
function nextGateSaid(whole: JobWhole | null, step: StepDetail | undefined): string {
  if (whole === null || step === undefined) return "";
  const ahead = whole.steps.filter((one) => one.ordinal > step.ordinal);
  const away = ahead.findIndex((one) => one.advance_gate === "human_always") + 1;
  const at = away === 0 ? undefined : ahead[away - 1];
  if (at === undefined) return "";
  return `${at.label} asks you, ${away} ${away === 1 ? "step" : "steps"} away`;
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
 * The criteria a Judge refused on this step, with the first one's words.
 *
 * **The refusal is the thing that needs answering, not the step it happened
 * on.** Until 29 Sep 2026 the lead said `<step> is waiting on you` here, which
 * is what a clean sign-off says too — a Judge disputing the evidence and a Job
 * ready to approve read identically, and only the header badge told them apart.
 */
function refusals(step: StepDetail | undefined, criteria: readonly Criterion[]) {
  if (step === undefined) return undefined;
  const panels = panelsOf(step, criteria);
  const refused = panels.filter((panel) => panel.verdict === "not_met");
  const first = refused[0];
  if (first === undefined) return undefined;
  return { count: refused.length, of: panels.length, said: first.criterion?.text ?? first.criterionId };
}

/**
 * Why Fleet stopped the Job, in the registry's own verb. **Generated from the
 * Rust registry**, so a trigger Fleet learns to raise reads correctly without
 * this file being touched.
 */
function stoppedBecause(job: JobSummary): string | undefined {
  const named = job.reason?.named;
  return named == null ? undefined : (ESCALATION_REASON[named]?.verb ?? undefined);
}


/**
 * Two facts at most, divided rather than punctuated. **A middot and no full
 * stops**: `CheckRun.produced` is a log line and a command is a command, so
 * ending each with a period was the surface writing prose over Fleet's words.
 */
function because(...parts: string[]): string {
  return parts.filter((part) => part !== "").join(" · ");
}

/**
 * The criterion this Job owes an attestation on, in the requester's own
 * words. **Read off `reason.criteria_owed`**, which is what the registry says
 * `awaiting_attestation` stores — never off the status.
 */
function owedSaid(job: JobSummary, whole: JobWhole | null): string | undefined {
  const owed = job.reason?.criteria_owed ?? [];
  if (owed.length === 0) return undefined;
  const criteria = whole?.acceptance_criteria ?? [];
  return owed
    .map((id) => criteria.find((one) => one.criterion_id === id)?.text ?? id)
    .join(" · ");
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

  // Waiting on you, **named by the thing itself and not by the step it is on**
  // — the owner's call of 29 Sep 2026, after three different situations drew
  // one sentence between them.
  //
  // The Drone's own question first: it is the one thing on the wire saying a
  // Drone has stopped and is holding its turn open.
  if (whole?.asking !== undefined) {
    return {
      said: "A Drone asked you something",
      because: because(whole.asking.question, holdsUp(whole, step)),
      tone: "awaiting-review",
      elapsed: span(whole.asking.asked_at, now) ?? undefined,
      act: "Answer it",
    };
  }
  // The command, because the command is the decision. A tool with no argument
  // has an empty `detail`, so the tool's own name carries the line instead.
  const command = whole?.command_waiting;
  if (command !== undefined) {
    return {
      said: "A Drone wants to run a command",
      because: because(
        command.detail === "" ? command.tool : `${command.detail}${command.truncated ? " …" : ""}`,
        holdsUp(whole, step),
      ),
      tone: "awaiting-review",
      elapsed: span(command.asked_at, now) ?? undefined,
      act: "Decide it",
    };
  }
  const refused = refusals(step, whole?.acceptance_criteria ?? []);
  if (refused !== undefined) {
    return {
      said: `A Judge refused ${refused.count} of ${refused.of} ${refused.of === 1 ? "criterion" : "criteria"}`,
      because: because(refused.said, holdsUp(whole, step)),
      tone: "awaiting-review",
      act: "Answer it",
    };
  }
  const lifecycle = JOB_LIFECYCLE[job.status];
  if (lifecycle?.whoIsActing === "Person" && lifecycle.mode === "Waited on") {
    // **Escalated is not the same as waiting to be approved.** Fleet stopped
    // the one and named why; the other is work that passed and wants a press.
    const stopped = stoppedBecause(job);
    if (stopped !== undefined) {
      return {
        said: `This Job ${stopped}`,
        because: tasksSaid(whole),
        tone: "completed-failed",
        act: "Read what stopped it",
        // The whole Record, because no one row is why it stopped — newest
        // first is the reading, and the row that ended it is the top of it.
        opens: { tab: "record" },
      };
    }
    // A criterion owed outside Armada, read off the reason the wire carries.
    // **No act**: Fleet serves no attestation, so a button would reach nothing.
    const owed = owedSaid(job, whole);
    if (owed !== undefined) {
      return { said: "A criterion needs your attestation", because: owed, tone: "awaiting-review" };
    }
    // **No branch is a Job that has never run**, which is the dispatch itself
    // waiting — `JobSummary.branch` is absent until a worktree exists. The
    // proposal is where it is answered, and `ProposalTab` is what draws it.
    if (job.branch === undefined) {
      return {
        said: "Waiting for your approval",
        because: "",
        tone: "awaiting-review",
        act: "Read the proposal",
        opens: { proposal: true },
      };
    }
    // **Nothing is being approved here.** A step spent its gate-failure retry
    // budget and the work is unfinished — `job-statuses.toml`, `awaiting_repair`
    // — which read as a sign-off under a badge saying *Needs repair*. No act:
    // Fleet does not serve the status, so there is nothing to build against.
    const spent = checkThatFailed(step);
    if (step?.state === "stopped" && spent !== undefined) {
      return {
        said: "Out of retries",
        because: because(`${spent.name} failed`, spent.produced ?? ""),
        tone: "completed-failed",
      };
    }
    return {
      said: "Waiting for your review",
      because: because(metSaid(whole), holdsUp(whole, step)),
      tone: "awaiting-review",
      act: "Review it",
    };
  }

  // Stopped, second — answering something that was waiting may already have
  // been what cleared it, which is the owner's reason for this order.
  if (lifecycle?.terminal === true && job.status !== "completed_success") {
    return {
      said: "This Job stopped",
      because: tasksSaid(whole),
      tone: "completed-failed",
      act: "Read what stopped it",
      opens: { tab: "record" },
    };
  }

  // A Check that went red, wherever the Job's status has it. **This is the
  // complaint the reframe came from** — a failed group was legible only by
  // opening the step, so a Job at `running` said nothing was wrong.
  const failed = checkThatFailed(step);
  if (failed !== undefined) {
    return {
      said: `${failed.name} failed`,
      because: because(
        failed.produced ?? "",
        step?.state === "running" || step?.state === "retrying" ? "Running again" : "",
      ),
      tone: "completed-failed",
      act: "Read what it produced",
      // That Check's own row, and the step it ran on with it: a Check at a
      // group's boundary is on the step that works the groups, and one that
      // failed mid-workflow is on whichever step `currentStep` found.
      opens: {
        check: {
          name: failed.name,
          stepAttempt: failed.attempt,
          ...(step === undefined ? {} : { step: step.step_id }),
        },
      },
    };
  }

  // Over, before running: a Job that finished has no running step to name and
  // would otherwise fall through to the quiet line.
  if (lifecycle?.terminal === true) {
    return { said: "Done", because: tasksSaid(whole) };
  }

  // Running — the board's own quiet line, `Narrow`: what is running leads, and
  // what will next need a person is the clause under it. The step's own label
  // and never a verb made out of it: a label is whatever the workflow's author
  // wrote, and `Landing` off `Land` is grammar this file would be inventing.
  const on = elapsedSince(step?.entered_at, now);
  return {
    said:
      step === undefined
        ? "Nothing needs you"
        : `${step.label}${on === undefined ? "" : ` · ${on} in`}`,
    because: because(tasksSaid(whole), nextGateSaid(whole, step)),
  };
}
