// What a step's gates found, read once for both surfaces that draw it.
//
// **Several surfaces read `check_runs` and `judged`** — the lead, the verdict
// slot and the grounds a refusal is read on. Those are the same rows read for
// different purposes, and reading them twice is the defect `#321` already found
// once in this file's neighbours — two surfaces stating one ordering
// separately, agreeing until one of them changed.
//
// **Nothing here is JSX, and that is the seam.** What a strip row looks like
// and what a verdict grid looks like are each surface's own answer. What they
// must not disagree about is which attempt's rows count, which member of a
// panel answered what, and whether a criterion was refused — so that is what
// lives here and nothing else does.
//
// The three rules this file holds, each of them from somewhere else:
//
// | Rule | Where it comes from |
// |---|---|
// | Only the current attempt's rows are the live gate | `check_runs` and `judged` hold every attempt since protocol 7.0 |
// | A panel is one row per member and reads as one row per criterion | `Judged.member`, since protocol 7.7 |
// | Any single refusal refuses the criterion | `docs/concepts/judge.md`, unanimity |

import { CHECK_ADVANCES, CRITERION_VERDICT_CHECK } from "@armada/components";
import type {
  CheckRun,
  CheckUnderway,
  Criterion,
  DeclaredCheck,
  Judged,
  StepDetail,
} from "@armada/protocol";

import { isSweepMarker, nameOf } from "./declared";
import { onlyCurrentAttempt } from "@armada/screens/src/facts";

/**
 * One declared Check, and what this attempt's run of it came to.
 *
 * **Declared first, run second.** The list is the step's declaration, so a
 * Check the gate has not reached keeps its row — the shape of what is still
 * coming is part of reading a running Job, and a list built from the runs
 * would make a Job look like it has fewer gates than it has.
 */
export type CheckRead = {
  /** The Check's name, or the built-in's kind. What a citation resolves against. */
  name: string;
  /** The declaration, for the command and the paths it covers. */
  check: DeclaredCheck;
  /**
   * Absent where this attempt's gate has not reached it. **While the gate is
   * running, a finished Check's live row** — the one the ruling will write —
   * so every surface below reads a result the moment it lands, not when the
   * Judge answers.
   */
  run: CheckRun | undefined;
  /**
   * The gate's own entry for this Check while it runs them, and absent once
   * its ruling is written down. Whether the Check is waiting or running is read
   * off this by [`isWaiting`] and [`isRunning`], never guessed from `run`.
   */
  live: CheckUnderway | undefined;
};

/**
 * The step's Checks, each beside what it came to on the current attempt.
 *
 * **Empty is a step that gates on nothing, and so is a step whose workflow
 * Fleet does not hold.** Those are two different sentences and the phase
 * strip's note is where they are told apart — nothing that draws a list of
 * Checks can say either of them, so this answers with no rows and lets the
 * caller draw nothing rather than an empty region.
 *
 * **The live set wins while it is there.** `StepDetail.checking` is what the
 * gate is doing now; a re-gate of the same attempt would otherwise draw the
 * last ruling's rows beside Checks that are running again.
 */
export function checksOf(step: StepDetail): CheckRead[] {
  // The sweep marker never counts as a Check and never draws as one — it
  // declares that the step gates on everything the repository declares, and
  // nothing ever runs it. `run.ts`'s tier count reads this same filter, on
  // this same list, for the reason atop this file.
  const declared = (step.checks ?? []).filter((check) => !isSweepMarker(check));
  const runs = onlyCurrentAttempt(step.check_runs);
  const underway = step.checking?.checks ?? [];
  return declared.map((check) => {
    const name = nameOf(check);
    const live = underway.find((one) => one.name === name);
    const run = live === undefined ? runs.find((ran) => ran.name === name) : ranOf(live);
    return { name, check, run, live };
  });
}

/** Stopped when another Check failed first, so it says nothing either way. #1062. */
export function isStopped(read: CheckRead): boolean {
  return read.live?.stopped_by !== undefined;
}

/**
 * A finished live Check as the row it will be recorded as, with its live log
 * where the recorded one will go. `undefined` while it waits or runs.
 */
function ranOf(live: CheckUnderway): CheckRun | undefined {
  if (live.ran === undefined) return undefined;
  return live.output_path === undefined ? live.ran : { ...live.ran, output_path: live.output_path };
}

/**
 * A stopped step whose Checks a person sent to run again, while they run.
 *
 * **Read off the gate's live set and nothing else.** `checking` stands from
 * the moment a gate starts until its ruling is written down, and it is Fleet's
 * memory, never the record — so on a step that stopped it is that run and
 * cannot be a stale one. Fleet withholds `stuck.recourse` for the same span
 * (`checks_rerunning`), which is not on the wire. The owner's Job 3, 4 Oct 2026.
 */
export function checksAgain(step: StepDetail): boolean {
  return step.state === "stopped" && step.checking !== undefined;
}

/** Started, and not finished: the gate is running it right now. */
export function isRunning(read: CheckRead): boolean {
  return read.live?.started_at !== undefined && read.live.ran === undefined;
}

/**
 * On the gate's list and not started — waiting for one of its slots, or for
 * the Commands it requires. **Not the same as not reached**: the gate has
 * reached it and is working towards it.
 */
export function isWaiting(read: CheckRead): boolean {
  return read.live !== undefined && read.live.started_at === undefined && read.live.ran === undefined;
}

/**
 * How many Checks from other work hold the machine's places while this one
 * waits for room. `undefined` where it is not waiting, or waits only on its own run. #1063.
 */
export function waitingBehind(read: CheckRead): number | undefined {
  return isWaiting(read) ? read.live?.waiting_behind : undefined;
}

/** How many places this Check takes. `undefined` at one, or not waiting. #1102. */
export function placesOf(read: CheckRead): number | undefined {
  return isWaiting(read) ? read.live?.places : undefined;
}

/**
 * Runs on the current attempt that no declared Check accounts for.
 *
 * **What `checksOf` cannot surface.** It joins from `step.checks`, the
 * declaration, so a mechanical run with no declared counterpart — the
 * `artifact_exists` check a gate-only workflow keeps as its whole tier — never
 * appears there. The verdict sheet's "what proves it" reads this alongside
 * `checksOf` so that a real run is never dropped for want of a declaration to
 * join it to.
 */
export function mechanicalRunsOf(step: StepDetail): CheckRun[] {
  const named = new Set(checksOf(step).map((read) => read.name));
  return onlyCurrentAttempt(step.check_runs).filter((run) => !named.has(run.name));
}

/** A Check that did not pass, read off the registry's own `advances`. */
export function didNotPass(run: CheckRun): boolean {
  return CHECK_ADVANCES[run.outcome] === false;
}

/**
 * A Check that ran and held — `CheckOutcome::passed`, and `check-outcomes.toml`
 * is where *did it pass* and *may the step advance* are two questions.
 *
 * **Not `!didNotPass`.** `CHECK_ADVANCES` cannot tell a pass from a skip, so
 * counting passes off `advances` reads a step that skipped every Check as one
 * that verified itself — the vacuous pass `verification::mechanical` exists to
 * make unreachable. An outcome this build has never seen answers `false` to
 * both, rather than defaulting into either.
 */
export function didPass(run: CheckRun): boolean {
  return run.outcome === "passed";
}

/**
 * What a tier the run has not got to stands at.
 *
 * **The registry's word, and `criterion_verdict_check` owns it**: the five
 * `check_outcome` carries are what a Check that *ran* did, and `icons.toml`
 * already calls "not reached" a Check state. Once here for every surface that
 * says it, rather than typed at each.
 */
export const NOT_REACHED = CRITERION_VERDICT_CHECK.not_reached?.verb ?? "not_reached";

/** The Checks that ran on this attempt, and the ones among them that failed. */
export function howTheChecksWent(reads: readonly CheckRead[]): {
  ran: CheckRead[];
  failed: CheckRead[];
} {
  // A stopped Check is neither: it did not finish, so it neither ran nor failed.
  const ran = reads.filter((read) => read.run !== undefined && !isStopped(read));
  return { ran, failed: ran.filter((read) => read.run !== undefined && didNotPass(read.run)) };
}

/**
 * What the Checks tier stands at, in words — `3 of 3 passed`.
 *
 * **One sentence, because two surfaces say it.** The phase strip's tier and the
 * Checks chapter's header sit four inches apart, and a count written twice is a
 * count that disagrees with itself the day `skipped` stops advancing a step.
 */
export function checksStand(reads: readonly CheckRead[]): string {
  const { ran, failed } = howTheChecksWent(reads);
  const stopped = reads.filter(isStopped).length;
  const counted =
    (failed.length > 0
      ? `${failed.length} of ${reads.length} did not pass`
      : `${ran.length} of ${reads.length} passed`) + (stopped === 0 ? "" : ` · ${stopped} stopped`);
  // **While the gate works through them, what is running leads**, because "is
  // anything happening" is the question a person opened the step with. The
  // count of what landed rides behind it once there is one.
  const running = reads.filter(isRunning).length;
  const waiting = reads.filter(isWaiting).length;
  if (running > 0 || waiting > 0) {
    // Waiting behind other work reads as queued, not as a gate that is stuck. #1063.
    const forRoom = reads.some((read) => waitingBehind(read) !== undefined);
    const moving =
      running > 0 ? `${running} running` : forRoom ? `${waiting} waiting for room` : `${waiting} waiting`;
    return ran.length === 0 ? moving : `${moving} · ${counted}`;
  }
  if (ran.length === 0) return NOT_REACHED;
  return counted;
}

/**
 * Which Check's output an `o` press opens on this step, or none.
 *
 * **The failed one first.** A person reaching for an output on a step that
 * stopped wants the Check that says why; on a step where nothing failed there
 * is still a reading worth opening, so the first output there is answers
 * instead of nothing.
 *
 * **Narrowed to the current attempt, like everything else here.** Read across
 * every attempt this surfaced a stale run's output the moment a step had been
 * worked more than once — and the key and the Checks chapter's own act both
 * come through here, so the two cannot open different files.
 */
export function outputOf(step: StepDetail | undefined): string | undefined {
  return outputRunOf(step)?.output_path;
}

/**
 * Which Check's output a reader is offered on this step, whole.
 *
 * **One reading behind three surfaces.** The header act opens the file, `o`
 * opens the file, and the chapter now reads it into the panel — three ways of
 * asking one question, and three answers to it would put a person reading one
 * Check's output under a button that opens another's. `outputOf` is this,
 * narrowed to the path, and exists because two callers only ever wanted that.
 *
 * The row rather than the path, because a reader has to say whose output it is
 * showing: a console pane with a file name and no Check name is a transcript
 * nobody can attribute.
 */
export function outputRunOf(step: StepDetail | undefined): CheckRun | undefined {
  if (step === undefined) return undefined;
  const runs = onlyCurrentAttempt(step.check_runs).filter((run) => run.output_path !== undefined);
  return runs.find(didNotPass) ?? runs[0];
}

/**
 * One criterion, and what the panel that answered it made of it.
 *
 * **A panel sends one row per member and a person reads one row per
 * criterion.** At `panel_size: 3` three rows arrive carrying the same
 * `criterion_id` and differing only in `member`; drawn straight they are the
 * same sentence three times.
 */
export type Panel = {
  criterionId: string;
  /**
   * The Job's own criterion, where the id joins. **Absent is a criterion this
   * Job does not carry** — a verdict on something the frozen list has no row
   * for, which is a fact to draw rather than one to hide.
   */
  criterion: Criterion | undefined;
  /**
   * The criterion's frozen 1-based position in `acceptance_criteria`. What a
   * citation to `02` means, and never the row's place on screen. Absent where
   * the id does not join, because a position cannot be invented for it.
   */
  ordinal: number | undefined;
  /**
   * Every member that answered, in panel order. **One entry at `panel_size:
   * 1`**, whose `member` is absent — the convention `Judged.member` keeps.
   */
  members: Judged[];
  /** The members that refused. Empty is a criterion nothing objected to. */
  refused: Judged[];
  /**
   * Unanimity, from `docs/concepts/judge.md`: one veto refuses the criterion.
   * **`asking` is a live `JudgeQuestion`, or `step.judging`'s own call** —
   * neither met nor refused yet. #1153.
   */
  verdict: "met" | "not_met" | "asking";
};

/**
 * A step's verdicts as one entry per criterion, in the order they were asked.
 *
 * **First-appearance order, never sorted.** The order is the order the criteria
 * were asked, which is the order they were written, and a criterion may be
 * appended but never reordered — so position is stable and is what a citation
 * to `02` means.
 *
 * **Narrowed to the current attempt.** `judged` holds every attempt's rows
 * since protocol 7.0, and both surfaces draw the live gate.
 *
 * **Falls back to `step.judging`'s own criterion.** #1153.
 */
export function panelsOf(
  step: StepDetail,
  criteria: readonly Criterion[],
  /**
   * The criterion a live `JudgeQuestion` is holding open on this step, where
   * one is. Overrides `judged` for it — Fleet asks before it commits a
   * verdict, so an absent or stale row must not read as met.
   */
  asking?: string,
): Panel[] {
  return panelsFrom(onlyCurrentAttempt(step.judged), criteria, asking ?? step.judging?.criterion_id);
}

/**
 * `panelsOf`'s own grouping, over whichever `Judged` rows a caller already
 * narrowed. **One reading for both shapes of narrowing** — the live gate's
 * current attempt, and the run tree's single historical one — so a step's
 * Judge count cannot read one way in the strip and another in the rail. #689.
 */
export function panelsFrom(
  judged: readonly Judged[],
  criteria: readonly Criterion[],
  asking?: string,
): Panel[] {
  const held = new Map<string, Judged[]>();
  for (const one of judged) {
    const already = held.get(one.criterion_id);
    if (already === undefined) held.set(one.criterion_id, [one]);
    else already.push(one);
  }
  if (asking !== undefined && !held.has(asking)) held.set(asking, []);
  return [...held].map(([criterionId, answered]) => {
    // In panel order rather than in arrival order. `member` is a position, and
    // a grid whose columns changed order between two criteria would put one
    // judge's mark under another judge's heading.
    const members = [...answered].sort((a, b) => (a.member ?? 1) - (b.member ?? 1));
    const at = criteria.findIndex((held) => held.criterion_id === criterionId);
    const refused = members.filter((one) => one.verdict !== "met");
    return {
      criterionId,
      criterion: at === -1 ? undefined : criteria[at],
      ordinal: at === -1 ? undefined : at + 1,
      members,
      refused,
      verdict: criterionId === asking ? "asking" : refused.length === 0 ? "met" : "not_met",
    };
  });
}

/**
 * Whether the run this step is being read as has ended.
 *
 * **Not the same question as `state`.** A run handed back reads `retrying`,
 * which is a step still working; what separates them is whether the attempt
 * itself closed. A gate that never reached its Judge says so in the past tense
 * once it has, and in the present tense only while there is one.
 */
export function runEnded(step: StepDetail): boolean {
  return step.attempts.at(-1)?.ended_at !== undefined;
}

/** How many criteria the step's declaration says the panel will answer. */
export function askedOf(step: StepDetail): number {
  return (step.judge_checks ?? []).reduce((sum, judge) => sum + judge.criteria, 0);
}

/**
 * Whether this attempt's panel was asked and never answered, rather than
 * never asked at all. Both draw `judged` empty for the current attempt, and
 * they are not the same sentence: one is a call still ahead of the step, the
 * other is one Fleet already made and could not read back.
 *
 * **`last_verdict.trigger` and not a live `judging` read**, because a step
 * open after the Job stopped carries no in-flight call — `judging` is
 * "right now", and there is no "now" left to name once the Job is over.
 */
export function stoppedUndecided(step: StepDetail): boolean {
  return step.last_verdict?.trigger === "gate_undecided";
}

/**
 * Fleet's own sentence, corrected to this screen's case. Fleet writes what it
 * logged, lower-case and unpunctuated like every other log line; everywhere
 * this crosses onto a sentence of its own the two surfaces that draw it would
 * otherwise punctuate it two different ways.
 */
export function sentenceOf(said: string): string {
  const capped = said.charAt(0).toUpperCase() + said.slice(1);
  return /[.!?]$/.test(capped) ? capped : `${capped}.`;
}
