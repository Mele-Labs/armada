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

import { checksAgain, checksOf, didNotPass, didPass, isRunning, panelsOf } from "./gates";

import { elapsedSince, span } from "./duration";
import { onlyCurrentAttempt } from "./facts";
import { flagSaid, flagsOf, heldByAFlag } from "./gaming";

import type { DetailTab } from "./detail-tabs";
import { fixesMainOf } from "./main-red";
import type { CheckAt } from "./tab-record";

/**
 * Where an act sends a person: a destination, or one Check's row on the
 * Record. **The Record's own route** for the second — the same
 * `RecordTabProps.opensCheck` the Plan's group boundary presses through.
 *
 * **Absent where the thing the act names is already under the lead.** A
 * Drone's question and a command it was not given are answered in the lead's
 * own region, not at a destination.
 */
export type LeadOpens = { tab: DetailTab } | { check: CheckAt } | { mainLog: { check: string; branch: string } };

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
  /**
   * A Drone's own words the lead quotes, kept out of `because` so they draw as
   * the markdown they were written in while Armada's sentence stays plain.
   */
  asked?: string;
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
  /**
   * The act is approving the dispatch, and its control is the header's own:
   * the same split button, the same handler, the same menu. **One act reached
   * two ways** — the owner, 1 Oct 2026: *"It should be in both places."*
   */
  approves?: true;
  /**
   * The lead found nothing to name and fell through to its quiet line. **With
   * the read still out that line is a guess**, so the screen stands in for it;
   * every other branch reachable with no read is proven by the Board's row.
   */
  quiet?: true;
  /**
   * The Job already fixing the test this Job's failed Check failed on, which
   * `because` follows. **Its own field so the title can be a press** — the
   * owner, 2 Oct 2026, #1673: the clause links to that Job. `rest` is the
   * clause after the title.
   */
  fix?: { job: string; title: string; rest: string };
  /**
   * The Jobs parked on this one's fix, each by its id and, where the wire
   * read it, its title. **The Jobs themselves and not a count** — the owner,
   * 2 Oct 2026, on the count this replaced: *"This should show the jobs that
   * are waiting on this job."* Drawn under the lead's line, never in it.
   */
  parkedOnIt?: { job: string; title?: string }[];
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
 * What the Drone did, on a Job the gaming check holds: the commands it was
 * refused on the held step, and what each standing flag caught.
 *
 * **What the Drone did, not where the Job stopped** (owner, 2 Oct 2026).
 * Until then this reached the escalated branch and said *This Job stopped at
 * Regression check*, which was true and said nothing about a weakened test.
 * The refused count is `stuck.refusals`, which counts the calls `refused` left
 * out, and never less than the rows that arrived.
 */
function gamingHold(whole: JobWhole | null) {
  const step = whole?.steps.find((one) => one.step_id === whole.stuck?.step_id);
  if (whole === null || step === undefined || !heldByAFlag(whole, step)) return undefined;
  const caught = [...new Set(flagsOf(step).held.map(flagSaid))].join(" · ");
  const count = Math.max(whole.stuck?.refusals ?? 0, whole.stuck?.refused.length ?? 0);
  const refused =
    count === 0
      ? undefined
      : `${count} ${count === 1 ? "command was" : "commands were"} refused during ${step.label}`;
  return { step, caught, refused };
}

/**
 * The Job already fixing the test `failed` failed on, where it is another Job.
 *
 * **Matched by the Check's name**, which is what `ClaimedBreakage.check`
 * carries. The claim says nothing about which attempt, so any red run of that
 * Check is the test the claim names.
 *
 * **Fleet keeping this Job off the test's files is said only where the wire
 * says it**: `held_off` present and holding a file. Since protocol 23.3.
 */
function fixedElsewhere(job: JobSummary, whole: JobWhole | null, failed: CheckRun) {
  const claim = whole?.breakages?.find((one) => one.check === failed.name && one.fix !== job.id);
  if (claim === undefined) return undefined;
  const held = (claim.held_off ?? []).length > 0;
  return {
    job: claim.fix,
    title: claim.fix_title,
    rest: ` is already fixing this${held ? ", and this Job is kept off the test's files" : ""}`,
  };
}

/**
 * The Jobs parked on this one's fix, or nothing where none are. A Job parked
 * on two of this Job's claims is one Job. #1673.
 */
function parkedOn(job: JobSummary, whole: JobWhole | null): { parkedOnIt?: { job: string; title?: string }[] } {
  const parked = new Map<string, string | undefined>();
  for (const claim of whole?.breakages ?? []) {
    if (claim.fix !== job.id) continue;
    for (const one of claim.waiting ?? []) parked.set(one.job_id, parked.get(one.job_id) ?? one.title);
  }
  if (parked.size === 0) return {};
  return {
    parkedOnIt: [...parked].map(([id, title]) => (title === undefined ? { job: id } : { job: id, title })),
  };
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
 * The headline for a stop. **A verb that opens on an article is a clause and
 * not a predicate** — "This Job the judge did not answer" is not English, so
 * it stands as its own sentence.
 */
function stoppedSaid(verb: string): string {
  return /^(the|a|an) /i.test(verb) ? `${verb[0]?.toUpperCase()}${verb.slice(1)}` : `This Job ${verb}`;
}

/** What `gate_undecided` leaves true: nothing was judged, and the person may go on or ask again. */
const UNJUDGED = "The work was not judged. Ask again, or accept the step yourself.";

/**
 * The status a dispatched request stands at until the proposer answers.
 *
 * **A wire value and not a token**, `render.ts`'s `awaiting_repair` rule: the
 * registry key is what Fleet sends, and a token is a rendering choice somebody
 * thinking about colour could rename. `proposing` shares `running`'s hue.
 */
const BEING_PROPOSED = "proposing";

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

  // A dispatched request the proposer has not answered. **First, and keyed on
  // the status**: this Job has no workflow, no step, no plan and no Drone, so
  // every rule below reads a thing it does not have and the quiet line at the
  // bottom said `Nothing needs you` over a model call that is spending money.
  if (job.status === BEING_PROPOSED) {
    return {
      said: "A model is reading the request",
      // **Empty, and the wait is under the lead instead.** `ProposerWait` draws
      // how far the call has got, what is left of Fleet's budget and how much
      // the model has thought; a clause here restating any of it is the
      // duplication the owner took out of this region on 30 Sep 2026.
      because: "",
      act: "Stop the proposer",
    };
  }

  // **A Job that is over answers nothing**, so nothing below that would answer
  // it is read on one. Fleet went on serving the question it was holding on
  // the owner's Job 1 after he killed it, 1 Oct 2026, and the lead offered
  // `Answer it` under a Killed badge. Waiting-on-you outranks stopped only
  // while the Job runs — answering may be what clears a Job still in flight,
  // which is the owner's reason for that order, and it clears nothing here.
  const lifecycle = JOB_LIFECYCLE[job.status];
  const over = lifecycle?.terminal === true;
  if (over && job.status !== "completed_success") {
    return {
      said: "This Job stopped",
      because: tasksSaid(whole),
      tone: "completed-failed",
      act: "Read what stopped it",
      opens: { tab: "record" },
    };
  }

  // Waiting on you, **named by the thing itself and not by the step it is on**
  // — the owner's call of 29 Sep 2026, after three different situations drew
  // one sentence between them.
  //
  // The Drone's own question first: it is the one thing on the wire saying a
  // Drone has stopped and is holding its turn open.
  if (!over && whole?.asking !== undefined) {
    return {
      said: "A Drone asked you something",
      asked: whole.asking.question,
      because: holdsUp(whole, step),
      tone: "awaiting-review",
      elapsed: span(whole.asking.asked_at, now) ?? undefined,
      act: "Answer it",
    };
  }
  // The command, because the command is the decision. A tool with no argument
  // has an empty `detail`, so the tool's own name carries the line instead.
  const command = over ? undefined : whole?.command_waiting;
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
  // **A step the gaming check holds**, named by what the Drone did. Ahead of a
  // Judge's refusal, because the flag is what `stuck` says stopped it, and
  // answered under the lead by `gaming-held.tsx`'s block.
  const held = over ? undefined : gamingHold(whole);
  if (held !== undefined) {
    return {
      said: held.refused ?? held.caught,
      because: because(held.refused === undefined ? "" : held.caught, holdsUp(whole, held.step)),
      tone: "awaiting-review",
      act: "Answer it",
    };
  }
  // **A stopped step's Checks running again.** The status and the step still
  // say stopped and Fleet offers nothing until the run ends, so *Out of
  // retries* with no act read as a dead end over work in flight — the owner's
  // Job 3, 4 Oct 2026. The line beneath is the Check running now; when the run
  // ends this branch is not taken and the lead is whatever Fleet then says.
  if (!over && step !== undefined && checksAgain(step)) {
    return {
      said: "Running Checks again",
      because: because(...checksOf(step).filter(isRunning).map((one) => one.name)),
    };
  }
  const refused = over ? undefined : refusals(step, whole?.acceptance_criteria ?? []);
  // **A stopped step has no question open.** The owner agreed with a refusal on
  // his Job 3, 2 Oct 2026, and the lead went on offering `Answer it` over
  // *do not start until you answer*. The refusal is still why it stopped; what
  // can be done is Fleet's `stuck.recourse`, drawn as `StepActs` in the act's
  // place by `tab-overview.tsx`. The headline says it stopped (owner, 3 Oct).
  if (refused !== undefined && step?.state === "stopped") {
    return {
      said: "Stopped on a Judge refusal",
      because: refused.said,
      tone: "completed-failed",
    };
  }
  if (refused !== undefined) {
    return {
      said: `A Judge refused ${refused.count} of ${refused.of} ${refused.of === 1 ? "criterion" : "criteria"}`,
      because: because(refused.said, holdsUp(whole, step)),
      tone: "awaiting-review",
      act: "Answer it",
    };
  }
  if (lifecycle?.whoIsActing === "Person" && lifecycle.mode === "Waited on") {
    // **Escalated is not the same as waiting to be approved.** Fleet stopped
    // the one and named why; the other is work that passed and wants a press.
    const stopped = stoppedBecause(job);
    if (stopped !== undefined) {
      return {
        said: stoppedSaid(stopped),
        // A step nobody judged says what is safe and what is on offer; the
        // plan's progress is on the canvas below it.
        because: job.reason?.named === "gate_undecided" ? UNJUDGED : tasksSaid(whole),
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
    // waiting — `JobSummary.branch` is absent until a worktree exists. The act
    // is the approval itself. It was `Read the proposal`, which reached
    // `ProposalTab` — drawn only off a draft, and no real Fleet serves one
    // until #1545 — so on the owner's own Job the lead offered nothing at all.
    if (job.branch === undefined) {
      return {
        said: "Waiting for your approval",
        because: "",
        tone: "awaiting-review",
        act: "Approve dispatch",
        approves: true,
      };
    }
    // **Nothing is being approved here.** A step spent its gate-failure retry
    // budget and the work is unfinished — `job-statuses.toml`, `awaiting_repair`
    // — which read as a sign-off under a badge saying *Needs repair*. **No act
    // of the lead's own**: Fleet's `stuck.recourse` says what can be done to the
    // step, and `tab-overview.tsx` draws those as `StepActs` in this act's place.
    const spent = checkThatFailed(step);
    if (step?.state === "stopped" && spent !== undefined) {
      const fix = fixedElsewhere(job, whole, spent);
      return {
        said: "Out of retries",
        because: because(`${spent.name} failed`, fix === undefined ? (spent.produced ?? "") : ""),
        tone: "completed-failed",
        ...(fix === undefined ? {} : { fix }),
      };
    }
    return {
      said: "Waiting for your review",
      because: because(metSaid(whole), holdsUp(whole, step)),
      ...parkedOn(job, whole),
      tone: "awaiting-review",
      act: "Review it",
    };
  }

  // **A Job that took main's red leads with it**, ahead of a Check of its own: the Check is main's,
  // and its log is the one the merge line's head opens too.
  const fixes = fixesMainOf(job);
  if (!over && fixes?.state === "fixing") {
    return {
      said: "Fixing main",
      because: fixes.test === undefined ? `${fixes.check} · #${fixes.merge}` : `${fixes.test} · #${fixes.merge}`,
      tone: "completed-failed",
      act: "Read the log",
      opens: { mainLog: { check: fixes.check, branch: "main" } },
    };
  }

  // A Check that went red, wherever the Job's status has it. **This is the
  // complaint the reframe came from** — a failed group was legible only by
  // opening the step, so a Job at `running` said nothing was wrong.
  const failed = checkThatFailed(step);
  if (failed !== undefined) {
    // **Another Job already fixing the test is what changes what a person
    // does** (the owner, 2 Oct 2026, #1673), so it leads the second line and
    // the output follows it. `Running again` gives way: two facts at most.
    const fix = fixedElsewhere(job, whole, failed);
    return {
      said: `${failed.name} failed`,
      because: because(
        failed.produced ?? "",
        fix === undefined && (step?.state === "running" || step?.state === "retrying") ? "Running again" : "",
      ),
      ...(fix === undefined ? {} : { fix }),
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
    ...parkedOn(job, whole),
    ...(step === undefined ? { quiet: true as const } : {}),
  };
}
