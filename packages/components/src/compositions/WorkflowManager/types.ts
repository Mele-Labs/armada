// What the Workflow manager draws: workflows by where they come from, each a run of steps whose words
// are what the agent is told. `.claude/decisions/2026-10-27-a-feature-job-walks-before-it-tests.md`.

import {
  ClipboardList,
  FilePenLine,
  Footprints,
  GitPullRequest,
  ListChecks,
  MessageCircleQuestion,
  MessageSquarePlus,
  Cable,
  Shapes,
  Stamp,
  Microscope,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

/** Where a workflow comes from. A later one replaces an earlier one of the same id: Armada's, then the repository's, then yours. */
export type WorkflowSource = "armada" | "repository" | "yours";

/** The glyphs a step may carry, by name; the data holds the name and this holds the drawing. */
export const STEP_GLYPHS = {
  propose: MessageSquarePlus,
  discuss: MessageCircleQuestion,
  approve: Stamp,
  walk: Footprints,
  fix: Cable,
  deliver: GitPullRequest,
  cause: Microscope,
  tests: ListChecks,
  draft: FilePenLine,
  sketch: Shapes,
  plan: ClipboardList,
} as const satisfies Record<string, LucideIcon>;

export type StepGlyph = keyof typeof STEP_GLYPHS;

export type ManagedStep = {
  id: string;
  label: string;
  glyph: StepGlyph;
  /** What ends the step, finishing "Ends when": `you reply`. */
  ends: string;
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
  steps: readonly ManagedStep[];
};

/** One workflow copy: the source and the id name it. */
export type WorkflowRef = Pick<ManagedWorkflow, "id" | "source">;

export const keyOf = (one: WorkflowRef) => `${one.source}:${one.id}`;

/** A Job Fleet nudged at a step. */
export type DriftedJob = { id: string; job: string; what: string; when: string };

/** The recent nudges, by workflow id and then step id. */
export type Drift = Readonly<Record<string, Readonly<Record<string, readonly DriftedJob[]>>>>;

/** An agent asked to change a workflow's shape: nothing, working, or a change waiting on Approve. */
export type AgentState =
  | { phase: "idle" }
  | { phase: "working" }
  | { phase: "unchanged" }
  | { phase: "proposed"; before: readonly ManagedStep[]; after: readonly ManagedStep[] };

/** New words for a step, and the nudged Jobs they were drawn from. */
export type Suggestion = { words: string; from: readonly string[] };

/** The suggested changes waiting on the owner, by workflow id and then step id. */
export type Suggestions = Readonly<Record<string, Readonly<Record<string, Suggestion>>>>;
