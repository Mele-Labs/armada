import type { ReactNode } from "react";
import { phaseSaid } from "./compositions/PhaseCard/PhaseCard";
import { Tooltip } from "./primitives/Tooltip/Tooltip";

/**
 * What a word naming an Armada concept means, in one sentence, written once.
 *
 * **Standing copy rather than values.** Every sentence here is true of every
 * Job on every workflow, which is exactly what makes it a thing a surface must
 * not retype: the moment two screens each explain what a Check is, they can
 * disagree, and the difference between a Check and a Judge is the distinction
 * the whole gate rests on.
 *
 * **Keyed by the word a reader sees, so no call site holds a string.** A fact
 * label, a chapter title, a region's eyebrow and a header field are all the
 * same question — *what is this thing* — asked of the same vocabulary. Looking
 * the answer up by the visible word is what keeps this one table instead of one
 * annotation per component, which is how a hover pass becomes two hundred
 * hand-written strings that drift.
 *
 * **The three gate tiers are not here.** `PhaseCard` already writes them, keyed
 * by kind *and* state, because a tier that can never hold a step cannot say
 * what a tier holding one says — see `phaseSaid`. Those entries below read it
 * rather than carrying a second sentence about the same thing.
 *
 * **What does not get an entry.** A criterion sentence, a Judge's grounds, a
 * log line and a step's own name already read as themselves; a hover that
 * restates the visible text is worse than none.
 */
const SAID: Readonly<Record<string, string>> = {
  // The gate tiers, read from the one place they are written. `current` is the
  // state a tier holding this step stands in, which is what a reader asking
  // "what is a Check" is looking at when they ask it.
  checks: phaseSaid("checks", "current") as string,
  judge: phaseSaid("judge", "current") as string,
  waiting: phaseSaid("human", "waiting") as string,

  produced:
    "The work product of this step — the files the Drone changed, and anything else it wrote. " +
    "It is what the Checks run against and what the Judge reads.",
  cleared:
    "The Checks this step passed. A step advances when everything its gate asks for has cleared.",
  attempt:
    "One run of this step by a Drone. A refused step is handed back and tried again up to the " +
    "workflow's limit, and every attempt keeps its own log.",
  stopped:
    "Every attempt this step allows, spent on the same failure. Nothing is lost and nothing " +
    "advances; the next move is yours.",

  "drone instructions":
    "What the Drone was given at the start of this step: the brief, this step's own task, and " +
    "the criteria it will be judged against. It never sees the Judge's answer.",
  "activity log":
    "Every turn Fleet recorded on this step — what the Drone read, wrote and ran, and what Fleet " +
    "did about it.",

  worktree:
    "A checkout of the repository cut for this Job alone. Nothing outside it is touched, and it " +
    "is reclaimed when the Job is cleaned up.",
  branch:
    "The branch this Job's worktree sits on. Every commit its Drone makes lands here and nowhere " +
    "else.",
  manifest:
    "The repository's own file, declaring its Checks and the commands behind them. Fleet reads " +
    "it; a Drone cannot change what it runs.",
  workflow:
    "The steps this Job runs, in order, and what it takes to advance past each one. It is frozen " +
    "at dispatch, so editing the definition does not change a Job already running.",
  // The two keys a step's gate can defer to, named where a row prints one. A
  // policy is read at the gate and never frozen onto the Job, which is the
  // half of each sentence a reader cannot get from the row.
  review_gate:
    "The repository's rule for a step that defers to it: whether a person answers the step, or " +
    "the Checks and the Judge do. It is read from the Manifest each time a step reaches its " +
    "gate, so editing it changes a Job that is already running.",
  auto_merge:
    "The repository's rule for landing without a person: whether Fleet may merge the pull " +
    "request, and whether the forge's own checks have to pass first. It is read from the " +
    "Manifest each time, so editing it changes a Job that is already running.",
  "job log":
    "Everything Fleet recorded about this Job, one line per event. The screen you are reading is " +
    "this file read back.",
  transcript: "Every turn between Fleet and the Drone, as it was sent. The activity log summarises it.",
  drone:
    "The coding agent working this Job. Bridge never talks to one — every instruction and every " +
    "answer goes through Fleet.",

  brief:
    "What was asked for, in the requester's own words, with the acceptance criteria written " +
    "beside it. Every step is read against it.",
  "run time":
    "Wall clock since this Job first started running, stopping where the Job stopped. Time it " +
    "spent waiting for approval, or queued for a slot before its first run, is not in it.",
  job:
    "What a person calls this Job — the number it came from and its title, which is also the " +
    "name on its branch and its worktree.",
  "spend, estimated":
    "What the Job has cost so far, from the tokens its Drone reported. Estimated rather than " +
    "measured, which is why it is marked approximate.",

  "the run": "The workflow's steps, in the order this Job ran them. The order is the workflow's and never changes.",
  "where things are":
    "What this Job holds on disk and what identifies it. The screen above exists so nobody needs " +
    "these; they are here for when you want one anyway.",
  pulse:
    "The last thing anyone did on this Job, whether Fleet, its Drone or you, and what it holds on " +
    "this machine right now: its processes, its worktree and the disk that takes. Details opens " +
    "the full reading.",
  "what it left behind":
    "What the Job recorded, kept after it ended — its moves, its Drone's turns, what it touched " +
    "and what it claimed.",

  criterion:
    "One thing the work has to be true of, written when the Job was dispatched and frozen from " +
    "then on. The Judge answers per criterion, never overall.",
  // Keys nothing on screen spells, so the call site names the concept rather
  // than the word. Written here all the same: a sentence about the vocabulary
  // has one home whether or not a label happens to carry its name.
  "#":
    "The criterion's frozen position in the brief. A citation names this number, never the row's " +
    "place on screen, so the order here is the brief's and is never sorted.",
  "the split":
    "How many of the panel refused. One veto is a refusal whatever its size — the count is what " +
    "tells a lone dissent from a unanimous one, which are different situations for you and the " +
    "same verdict for the Job.",

  // A Job's own settings. Every one of these is a label a person hovers on
  // Overview, and a label there answers for itself or it answers nowhere: a
  // `?` beside a region explains the region, and nine marks down a column of
  // nine settings would be nine cards for nine words.
  model: "Which model a task runs on, picked by how hard the planner judged that task to be.",
  difficult: "Tasks the planner judged hardest. Each one runs on the model named beside it.",
  medium: "Tasks the planner judged ordinary. Each one runs on the model named beside it.",
  easy: "Tasks the planner judged simplest. Each one runs on the model named beside it.",
  "drones at once":
    "How many Drones this Job may run side by side. The machine's own cap holds over it, so a " +
    "Job never runs more than the machine allows.",
  "cost cap":
    "What this Job may spend before Fleet stops it. Stopping is not failing — the work is kept " +
    "and you decide whether to raise the ceiling.",
  "turn cap":
    "How many turns this Job's Drones may take before Fleet stops it. A turn is one exchange " +
    "between Fleet and a Drone.",
  repository:
    "The git repository this Job works in. Its Manifest is what declares the Checks, and the " +
    "worktree is cut from it.",
  // `Base branch` since 28 Sep 2026, matching the composer's own picker
  // (#1627). It was keyed `from`, and job detail's label with it, which left
  // one field with two names on two screens a minute apart.
  "base branch":
    "The ref this Job's branch is cut from. It differs from where the work lands when you start " +
    "from a branch nothing has merged yet.",
  "lands in":
    "The branch this Job's pull request is opened against. The Manifest names it unless this " +
    "Job was given another.",
  landing: "Where the work goes when it is done, and what has to be true before Armada says so.",
  branches:
    "Whether the Job takes one branch for all of its work or one per group of tasks. Every " +
    "commit its Drones make lands on that branch and nowhere else.",
  "complete when":
    "What has to have happened before Armada calls the Job finished. Until it has, the Job " +
    "stays open whatever its last step did.",
  // **One word, two surfaces**: the header's fact names the pull request this
  // Job opened, and this setting names how it is offered. One sentence has to
  // be true of both, which is what keying by the word buys.
  "pull request":
    "The pull request this Job's work lands as. It is opened either way — offered for review, " +
    "or parked as a draft, which asks nobody to look at it yet.",
};

/**
 * What a word means, or nothing where it names no concept.
 *
 * **The word is normalised, not matched exactly**, so `Attempt 2` and
 * `Attempt 3` are the same question as `Attempt 1` — a run tree with three
 * attempts on one step would otherwise need three entries and get two of them
 * wrong the day a fourth appears.
 *
 * A `ReactNode` that is not a string answers nothing: a label built from
 * elements is a label whose text this cannot read, and guessing at it is how a
 * hover ends up describing the wrong thing.
 */
export function conceptSaid(word: ReactNode): string | undefined {
  if (typeof word !== "string") return undefined;
  const key = word.trim().toLowerCase().replace(/\s+/g, " ").replace(/ \d+$/, "");
  return SAID[key];
}

/**
 * A label, and what its word means where the table above has a sentence for
 * it. A word the table does not carry draws as itself, with no hover at all.
 *
 * **One of these, not one per surface.** The header, the run tree and the
 * proposal each grew a private label doing exactly this, which is three
 * places for one rule. It lives beside the sentences it looks up rather than
 * in a composition of its own: the table is the thing, and this is how it
 * reaches a screen.
 */
export function ConceptLabel({
  children,
  className,
}: {
  children: ReactNode;
  /** The caller's own class for the label. Absent draws a bare span. */
  className?: string;
}) {
  const says = conceptSaid(children);
  const label = <span className={className}>{children}</span>;
  return says === undefined ? (
    label
  ) : (
    <Tooltip asChild label={says}>
      {label}
    </Tooltip>
  );
}
