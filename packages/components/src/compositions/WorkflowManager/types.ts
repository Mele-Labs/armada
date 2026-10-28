// What the Workflow manager draws: workflows by where they come from, each a run of steps whose words
// are what the agent is told. The decision of 27 Oct, a feature Job walks before it tests.

import {
  Cable,
  ClipboardList,
  Footprints,
  GitPullRequest,
  MessageCircleQuestion,
  MessageSquareText,
  Microscope,
  NotebookPen,
  PenTool,
  Stamp,
  TestTubeDiagonal,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

/** Where a workflow comes from. A later one replaces an earlier one of the same id: Armada's, then the repository's, then yours. */
export type WorkflowSource = "armada" | "repository" | "yours";

/** The glyphs a step may carry, by name; the data holds the name and this holds the drawing. */
export const STEP_GLYPHS = {
  propose: MessageSquareText,
  discuss: MessageCircleQuestion,
  approve: Stamp,
  walk: Footprints,
  fix: Cable,
  deliver: GitPullRequest,
  cause: Microscope,
  tests: TestTubeDiagonal,
  draft: NotebookPen,
  sketch: PenTool,
  plan: ClipboardList,
} as const satisfies Record<string, LucideIcon>;

export type StepGlyph = keyof typeof STEP_GLYPHS;

/** The glyph of the steps the carried workflows name; a step of any other id draws none. */
const GLYPH_OF_STEP: Readonly<Record<string, StepGlyph>> = {
  propose: "propose",
  discuss: "discuss",
  approve_plan: "approve",
  walk: "walk",
  short_walk: "walk",
  show: "walk",
  fix: "fix",
  build: "fix",
  refine: "fix",
  wire: "fix",
  deliver: "deliver",
  find_the_cause: "cause",
  tests: "tests",
  draft: "draft",
  sketch: "sketch",
  record_plan: "plan",
};

export const glyphOf = (stepId: string): StepGlyph | undefined => GLYPH_OF_STEP[stepId];

export type ManagedStep = {
  id: string;
  label: string;
  /** Absent on a step whose id has no glyph of its own. */
  glyph?: StepGlyph;
  /** What ends the step, finishing "Ends when": `you reply`. Empty where nothing Fleet observes ends it. */
  ends: string;
  /** Each thing that ends the step where any one of several does, in order: drawn as a list under "Ends when" in place of `ends`. */
  endsAny?: readonly string[];
  /** When the step is passed over, finishing "Passed over when": `nothing would be seen`. */
  passedOver?: string;
  /** The instructions the agent is told. */
  words: string;
};

export type ManagedWorkflow = {
  id: string;
  source: WorkflowSource;
  /** The requests it is for, as the workflow's `for_requests` line says them. */
  forRequests: string;
  /** The Manifest of the repository this copy was read under. Armada's and Yours are the same under every one. */
  manifestId?: string;
  /** That repository's name, for the group a repository's copies sit in. Absent draws the `repository` prop. */
  repository?: string;
  steps: readonly ManagedStep[];
  /**
   * Set on a file Fleet left out: its sentence for why, and the file that names it. Such a copy has no steps
   * to draw, and is found by its file and not by its id.
   */
  leftOut?: { said: string; file: string };
};

/** One workflow copy: the source and the id name it, the Manifest it is read and acted on under, and the file where it was left out. */
export type WorkflowRef = Pick<ManagedWorkflow, "id" | "source" | "manifestId"> & { file?: string };

/**
 * What names a copy: a row, which carries the file it was left out at under `leftOut`, or a ref, which carries it itself.
 * **A repository's copy is also named by its repository**, since two repositories hold a copy of one id; Armada's
 * and Yours are one copy whichever repository read them.
 */
export const keyOf = (one: WorkflowRef & { leftOut?: { file: string } }) => {
  const file = one.leftOut?.file ?? one.file;
  const repository = one.source === "repository" && one.manifestId !== undefined ? `@${one.manifestId}` : "";
  return `${one.source}${repository}:${one.id}${file === undefined ? "" : `:${file}`}`;
};

/** A Job Fleet nudged at a step. `jobId` is the Job that Open Job opens. */
export type DriftedJob = { id: string; jobId: string; job: string; what: string; when: string };

/** The recent nudges, by workflow id and then step id. */
export type Drift = Readonly<Record<string, Readonly<Record<string, readonly DriftedJob[]>>>>;

/**
 * An agent asked to change a workflow's shape: nothing, working, a draft that did not fit, or a change waiting
 * on Approve. `asked` is what he typed, where a reload has lost it; `refused` is Fleet's sentence for an Approve
 * it would not take.
 */
export type AgentState =
  | { phase: "idle" }
  | { phase: "working"; asked?: string }
  | { phase: "failed"; reason: string; asked?: string }
  | { phase: "proposed"; before: readonly ManagedStep[]; after: readonly ManagedStep[]; asked?: string; refused?: string; /** A workflow nothing held before: approved or dismissed, never sent back. */ fresh?: true };

/** New words for a step, and the nudged Jobs they were drawn from. A draft that did not fit has no words and says why. */
export type Suggestion = { words: string; from: readonly string[]; failed?: string };

/** What an act answers with: nothing where it went through, or Fleet's sentence for why it did not. */
export type WorkflowActAnswer = string | undefined;

/** The suggested changes waiting on the owner, by workflow id and then step id. */
export type Suggestions = Readonly<Record<string, Readonly<Record<string, Suggestion>>>>;
