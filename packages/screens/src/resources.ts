// What the machine panel says when it has no reading, and the two failures
// where it offers nothing to press.
//
// **Its own file because it is a decision, and a decision is unit-tested.** A
// `play` that computed rather than read would be a unit test paying a browser's
// price — `docs/practices/react.md` is explicit — and this is the decision that
// either offers a person a next move or admits there is none.
//
// # Nothing to ask
//
// A person opened a Job that stopped because Fleet itself did not answer, and
// the one control on the screen asked Fleet a question. The panel read "Nobody
// has asked whether this job is working. Looking costs no model call." over an
// act then labelled `Look now`, because a read that failed and a read nobody
// had made drew the same reading. #462.
//
// The act is withdrawn on the failure and never on the fact of failure. A
// refusal carries a code and a wait that ran out may already have been served,
// so both leave the control where it is — a panel that withdrew its act on a
// timeout would strand somebody whose Fleet is fine.
//
// Two of them withdraw it, for opposite reasons, and the panel says which. One
// is a Fleet that is not there. The other is a Fleet that is demonstrably up
// and answering something Bridge cannot read, which is structural rather than
// transient: the same request down the same route meets the same disagreement,
// and a restart brings back the build that caused it. #344 classifies that
// same condition as a fault on the command seam, for the same reason.

import type {
  Figure,
  HoldsFigures,
  HoldsLine,
  NothingToAsk,
  PulseReading,
  PulseWorktreeRow,
} from "@armada/components";
import { JOB_STATUS, nothingRunningIsAFault, sized } from "@armada/components";
import type {
  History,
  Holds,
  JobDetail as JobWhole,
  JobExamined,
  JobResources as Held,
  Noted,
  Recorded,
  StepDetail,
  Turn,
} from "@armada/protocol";
import type { PulseView } from "./draft/pulse";
import { ordered, spent } from "./facts";
import { cap } from "./RaiseCap";
import { checksOf, isRunning } from "./gates";
import type { LogRow } from "./story";
import { entriesOf, hideUnread } from "./story";
import { clock } from "./duration";
import { notesOf } from "./notes";
import { PULSE_INTERVAL_SAID } from "./pulse-poll";

/**
 * What a failed look says, and it says nothing about the Job.
 *
 * **Fleet did not answer, which is not a finding.** Drawing a failure as
 * `not_working` would report a Job as broken on the strength of a connection.
 */
export const LOOK_FAILED = "Fleet did not answer the look. Nothing here is a finding about the Job.";

/**
 * Why there is no machine reading, which is never the same sentence twice.
 *
 * **A read that has not answered and a Job that holds nothing are different
 * things**, and this is the half that says which — the panel's own arm says the
 * other. `undefined` where the reading is in hand.
 */
export function whyNoReading(resources: Holds): string | undefined {
  switch (resources.state) {
    case "none":
      return "Not reading";
    case "reading":
      return "Reading the machine.";
    case "failed":
      return "Fleet did not answer";
    case "read":
      return undefined;
  }
}

/**
 * Whether pressing `Refresh` could work, and where it could not, which of the
 * two reasons it is. `undefined` leaves the act on the panel.
 *
 * **The failure decides it, never the fact that something failed.** Every
 * reading on that panel is Fleet's and its one control asks Fleet for another,
 * so the question is whether another attempt could answer — not whether the
 * last one did.
 *
 * **`no_answer` is Fleet not being on the other end.** `not_connected` is
 * Bridge holding no port at all: no runtime file, a pid that did not verify, or
 * a socket that has not come up, so there is no address to send to.
 * `unreachable` is the fetch itself failing, which is the same thing one layer
 * down. Starting Fleet is what fixes either.
 *
 * **`unreadable` is Fleet being up and the answer being unreadable.** Fleet
 * returned a status and the body under it was not something Bridge could read,
 * which is the two sides disagreeing about this route rather than a Job going
 * wrong. Fleet being demonstrably alive is not a reason to keep the control:
 * the disagreement is in the builds, so the same request meets it again, and
 * a restart brings back the Fleet that caused it. Starting Fleet is the wrong
 * fix, which is why this cannot share `no_answer`'s sentence.
 *
 * **`refused` and `timed_out` keep the act.** A refusal carries a code and is
 * Fleet answering the route as designed; a wait that ran out may already have
 * been served, and Fleet has its own budget. Both are a Fleet that is there.
 *
 * A kept reading is not any of this: `keepsLastGood` holds the last good answer
 * for the open Job, so `failed` here is a Job whose machine reading never
 * arrived.
 */
export function nothingToAsk(resources: Holds): NothingToAsk | undefined {
  if (resources.state !== "failed") return undefined;
  const outcome = resources.outcome;
  if (outcome.ok) return undefined;
  if (outcome.why === "not_connected") return "no_answer";
  if (outcome.why !== "transport") return undefined;
  switch (outcome.fault.why) {
    case "unreachable":
      return "no_answer";
    case "unanswerable":
      return "unreadable";
    case "timed_out":
      return undefined;
  }
}

// # The summary under the run
//
// The full reading is a sheet now, and what sits under the run is five lines.
// Both are drawn from this one answer — a second reading of the same `Held`
// would let the block and the sheet it opens disagree about the same Job.

/**
 * The figures the summary draws, or `null` where no reading arrived.
 *
 * **`null` is not a Job holding nothing**, and the summary's `note` is what
 * says which — the same split the full reading keeps.
 *
 * **One worktree row, not two, since #1484.** It drew `Worktree on disk` over
 * `Size on disk 1.2 GiB`, and the first of those was a constant: nothing in
 * Armada looks at a worktree unless a person presses `Refresh` in the sheet,
 * so every Job anyone opened said `on disk` and said it forever. Two rows, one
 * of which never varied, for a fact the other one carried with a number on it.
 */
export function summarised(
  reading: Held | null,
  examined: JobExamined | null,
): HoldsFigures | null {
  if (reading === null) return null;
  const worktree = standingOf(reading, examined);
  return {
    processes: reading.processes.length,
    nothingRunningIsWrong: nothingRunningIsAFault(reading.held, examined),
    worktree: worktree.said,
    worktreeIsWrong: worktree.wrong,
  };
}

// # What the Job is spending, which used to be read above the run
//
// Spend and Turns were two of the header's facts until #1484. They are worth
// knowing and they are not what a person opens a Job to read, and Pulse is
// already the region that answers *is this working and what is it taking* —
// so they sit with the processes and the worktree rather than above them.
//
// **They do not wait on the machine reading and are not qualified by it.**
// `Read 4s ago` is about `summarised`'s figures, which a look produces; these
// two come off `GET /jobs/:job_id` and are there whether or not anyone has
// looked, which is why they are their own props and are drawn above it.

/**
 * What the Job has cost so far, or nothing where the Fleet does not count.
 *
 * **Hedged, always.** The design contract spells an estimated value `~$2.40`
 * and never `$2.40`, and the figure is notional besides — it is what the run
 * would have cost at list price, which is not what a subscription account is
 * billed. A Fleet with no figure draws nothing rather than a zero: a Job that
 * cost nothing and a Fleet that does not price are two different facts.
 */
export function spentOn(whole: JobWhole | null): string | undefined {
  const spend = whole?.spend;
  return spend === undefined ? undefined : spent(spend.cost_micros, spend.unpriced);
}

/**
 * How many turns the Job has taken, against how many it may take.
 *
 * **Never hedged, unlike the spend beside it.** P4 hedges by source: a cost is
 * derived from list prices and wears a tilde, and a turn is counted. Writing
 * the two alike would lend the estimate the authority of the count.
 *
 * **The cap is drawn with it and not on a line of its own.** It is the second
 * of the two ceilings `over_budget` folds, and it stops a Job that has passed
 * every Check — so the number a person is deciding a raise against has to be
 * beside the number they are deciding about.
 */
export function turnsTaken(whole: JobWhole | null): string | undefined {
  const spend = whole?.spend;
  return spend === undefined ? undefined : `${spend.turns} of ${spend.turn_cap}`;
}

/**
 * The worktree in one value: what it takes on disk, or what is wrong with it.
 *
 * **The size is the ordinary answer, and it is the evidence.** A figure in
 * gibibytes is a directory that was walked, so it says the checkout is there
 * without claiming anything a look has not found — which is what `on disk`
 * was for, and `on disk` did not carry the number.
 *
 * **`healthy` is gone with it.** It was the word for a look that found nothing
 * wrong, and a look is a press almost nobody makes; the size stands in its
 * place, and a person who wants the verdict presses `Refresh` and reads it on
 * the sheet, which is where every other look's answer already is.
 *
 * **What a look finds wrong replaces the size rather than joining it.** `gone`
 * and `1.2 GiB` in one row would be a size for a directory that is not there.
 * `cannot_tell` is neither a fault nor a pass, the same as the full reading's
 * own arm for it.
 *
 * **`sized` is the panel's own formatter**, imported rather than retyped:
 * binary units, because `du` and `df` answer in them.
 */
function standingOf(reading: Held, examined: JobExamined | null): { said: string; wrong?: boolean } {
  const worktree = reading.worktree;
  if (worktree === undefined) return { said: NONE_ON_DISK };
  const look = examined?.looks.find((one) => one.asked === "worktree");
  if (look?.found === "not_working") return { said: "gone", wrong: true };
  if (look?.found === "cannot_tell") return { said: "could not be read" };
  return { said: worktree.bytes === undefined ? NOT_MEASURED : sized(worktree.bytes) };
}

/** A walk that ran past its bound. Its own answer, and never a zero. */
const NOT_MEASURED = "not measured";

/** No checkout. A Job at its gate or one already reclaimed — a real answer. */
const NONE_ON_DISK = "none on disk";

/**
 * The last thing anyone did on this Job, from whichever voice did it: the
 * Drone's turns, Fleet's and Armada's notes, and a person's moves on the Job's
 * history.
 *
 * **Compared on the wire's own timestamps.** A row's clock drops the date, and
 * a Job can run past midnight. A row this build cannot read is skipped, the way
 * the log hides it, and Fleet's and the Drone's own moves on the history are
 * left out because their streams already say them in their own words.
 */
export function latestOf(
  turns: readonly Turn[],
  notes: readonly Noted[],
  moves: readonly Recorded[],
): HoldsLine | undefined {
  const heard: { when: number; line: HoldsLine }[] = [];
  for (let at = turns.length - 1; at >= 0; at -= 1) {
    const turn = turns[at]!;
    const [row] = hideUnread(entriesOf([turn], undefined)).rows;
    if (row === undefined) continue;
    heard.push({ when: Date.parse(turn.ts), line: lineOf(row) });
    break;
  }
  const note = notes[notes.length - 1];
  const [noted] = note === undefined ? [] : notesOf([note]);
  if (note !== undefined && noted !== undefined) heard.push({ when: Date.parse(note.at), line: lineOf(noted) });
  for (let at = moves.length - 1; at >= 0; at -= 1) {
    const move = moves[at]!;
    if (move.actor !== "human") continue;
    heard.push({ when: Date.parse(move.at), line: { at: clock(move.at), actor: "You", said: movedBy(move) } });
    break;
  }
  heard.sort((one, other) => other.when - one.when);
  return heard[0]?.line;
}

/** The moves a Job's history read carries, where it is this Job's and has answered. */
export function movesOf(history: History | undefined, jobId: string): Recorded[] {
  return history?.state === "read" && history.jobId === jobId ? history.moves : [];
}

/** What to say where nothing has happened on the Job yet. */
export const NOTHING_HAPPENED_YET = "Nothing yet";

function lineOf(row: LogRow): HoldsLine {
  return {
    at: row.at,
    actor: SAYS[row.actor],
    said: row.message,
    wrong: row.payload.some((line) => line.named === "failed") || undefined,
  };
}

/** A person's move, as they would say they did it. */
function movedBy(move: Recorded): string {
  const moved = move.moved;
  if (moved.kind === "status") {
    if (move.status === "awaiting_approval") return "Approved dispatch";
    if (moved.to === "killed") return "Killed the job";
    if (moved.to === "rejected") return "Rejected the work";
    if (move.status === "awaiting_review" && moved.to === "completed_success") return "Approved the work";
    return `Moved the job to ${JOB_STATUS[moved.to]?.verb ?? moved.to}`;
  }
  if (moved.kind === "step") return `Moved ${moved.step_id} to ${moved.to}`;
  return `Drone ${moved.presence}`;
}

/**
 * Who wrote a line, as a person reads it.
 *
 * **Capitalised here and nowhere else.** The wire spells these lowercase
 * because most of its readings are mid-sentence; this is the first word of a
 * column, and the log's own rows draw it their own way.
 */
const SAYS: Record<LogRow["actor"], string> = {
  armada: "Armada",
  drone: "Drone",
  fleet: "Fleet",
};

// # The board Pulse draws
//
// `PulseView` is the draft shape (`draft/pulse.ts`) and this turns it into the
// rows the panel takes. **The seam is deliberate**: the draft is derived from
// today's wire, and the panel knows nothing about either.

/**
 * The reading, as the board draws it.
 *
 * **The look is applied to one worktree and never to several.** Fleet's
 * `worktree` look asks about the Job's own checkout; drawn against a list it
 * would be a finding about members nothing looked at.
 */
export function pulseReadingOf(view: PulseView, examined: JobExamined | null): PulseReading {
  return {
    held: view.held,
    readAt: view.read_at,
    processes: view.processes.map((one) => ({
      pid: one.pid,
      command: one.command,
      owner: one.owner,
      cpuPercent: one.cpu_percent,
      memoryBytes: one.memory_bytes,
      runningFor: one.running_for,
      recorded: one.recorded,
    })),
    worktrees: view.worktrees.map((one) => ({
      branch: one.branch,
      path: one.path,
      ...(one.bytes === undefined ? {} : { bytes: one.bytes }),
      ...(view.worktrees.length === 1 ? standing(examined) : { state: ON_DISK }),
    })),
    logs: view.logs.map((one) => ({
      kind: one.kind,
      owner: one.owner,
      ...(one.bytes === undefined ? {} : { bytes: one.bytes }),
      writing: one.writing,
    })),
  };
}

/** A checkout nobody found anything wrong with. The ordinary answer. */
const ON_DISK = "on disk";

/** What the one look that asks about a checkout found, as the row's state. */
function standing(examined: JobExamined | null): Pick<PulseWorktreeRow, "state" | "wrong"> {
  const look = examined?.looks.find((one) => one.asked === "worktree");
  if (look?.found === "not_working") return { state: "gone", wrong: true };
  if (look?.found === "cannot_tell") return { state: "could not be read" };
  return { state: ON_DISK };
}

/**
 * What this Job is running and what it is taking, over the board.
 *
 * **Drones come off the reading and the rest do not.** A Drone running is a
 * process on this machine, which only a look at the machine answers; the rest
 * are on `GET /jobs/:job_id`, there whether or not anybody has looked.
 *
 * **The verb is in the label and the value is a figure**, the design board's
 * own strip. `Judges` over `none out` is a phrase the eye lands on; `Judges
 * running` over `0` is a reading, and the bare `0` is no gap because the label
 * already says what is counted.
 */
export function pulseFiguresOf(view: PulseView | null, whole: JobWhole | null): Figure[] {
  const steps = ordered(whole);
  const working: Figure[] = [];
  if (view !== null) working.push(dronesFigure(view.held));
  working.push({ label: "Checks running", value: `${checksRunning(steps)}` });
  working.push({ label: "Judges running", value: `${judgesRunning(steps)}` });
  const taking: Figure[] = [];
  const spend = spendFigure(whole);
  if (spend !== undefined) taking.push(spend);
  const turns = turnsFigure(whole);
  if (turns !== undefined) taking.push(turns);
  if (view !== null) taking.push({ label: "Processes", value: `${view.processes.length}` });
  // The rule lands on the first cost there is, whichever survives: a Fleet that
  // does not price has no spend, and it still has to fall between the groups.
  const [first, ...rest] = taking;
  return first === undefined ? working : [...working, { ...first, apart: true }, ...rest];
}

/**
 * How many Drones are up, and whether that is a fault.
 *
 * **One Drone per Job today.** `held` is Fleet's reading of the one process it
 * recorded, so the count is nought or one; `gone` and `replaced` are Fleet
 * believing something is running that is not, which is loud at any status —
 * and both count nought, so the detail is the whole of what tells them apart.
 */
function dronesFigure(held: string): Figure {
  if (held === "gone") return { label: DRONES, value: "0", detail: NOTHING_AT_THAT_PID, wrong: true };
  if (held === "replaced") return { label: DRONES, value: "0", detail: SOMEBODY_ELSE, wrong: true };
  // Words, and marked as such: `could not be read` in the figure's own mono is
  // a sentence dressed as a number.
  if (held === "unreadable") return { label: DRONES, value: "could not be read", words: true };
  return { label: DRONES, value: held === "running" ? "1" : "0" };
}

/** The band's word for the Drone count, spelled once for the four arms above. */
const DRONES = "Drones running";

/** Fleet recorded a pid and nothing holds it. */
const NOTHING_AT_THAT_PID = "fleet recorded one";

/** The pid came round and something else has it. */
const SOMEBODY_ELSE = "that pid is another process";

/** Checks the gate has started and not finished, across every step. */
export function checksRunning(steps: readonly StepDetail[]): number {
  return steps.reduce((count, step) => count + checksOf(step).filter(isRunning).length, 0);
}

/** Judge calls out right now, across every step. `StepDetail.judging`. */
export function judgesRunning(steps: readonly StepDetail[]): number {
  return steps.filter((step) => step.judging !== undefined).length;
}

/**
 * What the Job has spent against what it may spend.
 *
 * **The cap is under the figure and no longer beside it.** It is still there
 * for the reason it always was — the number a person decides a raise against
 * has to be with the number they are deciding about — but on the band a cap on
 * the same line doubled the figure's length and stopped it reading as one.
 *
 * **The spend is hedged and the cap is not.** A cost is derived from list
 * prices and wears the tilde `spent` gives it; a ceiling is a setting somebody
 * chose, and `~$3.00` would make the limit read as an estimate too.
 */
function spendFigure(whole: JobWhole | null): Figure | undefined {
  const spend = whole?.spend;
  if (spend === undefined) return undefined;
  return {
    label: "Spend",
    value: spent(spend.cost_micros, spend.unpriced),
    detail: `of ${cap(spend.cost_cap_micros)}`,
  };
}

/**
 * Turns taken, with the turn cap under them. `turnsTaken` is the panel's one
 * line and this is the band's two — the same two numbers, split where the band
 * wants the figure alone.
 */
function turnsFigure(whole: JobWhole | null): Figure | undefined {
  const spend = whole?.spend;
  return spend === undefined
    ? undefined
    : { label: "Turns", value: `${spend.turns}`, detail: `of ${spend.turn_cap}` };
}

/**
 * What keeps the reading current, said beside its age.
 *
 * **Built from the interval rather than spelled beside it.** `screen.ts`
 * re-takes `/jobs/:job_id/resources` on every event naming the open Job, and
 * since `#1571` `resources-poll.ts` takes it again on `PULSE_INTERVAL_MS` for
 * as long as this board is drawn — so a Job sitting at a gate still moves.
 * Moving the interval moves the sentence, which is what stops one promising
 * what the other does not do.
 */
export const PULSE_REFRESHES = `Taken again every ${PULSE_INTERVAL_SAID} while open.`;
