// What the Now panel beside the Overview canvas draws, as data. **The draft fills all of it**;
// a real Fleet fills what it serves (`now-real.ts`) and has no plan interview, issues or sketches.
//
// The panel's `onOpen` and `onAnswer` are the host's. `target` names what a row
// opens (a Drone id, a Check's name, a Judge's step); a press with no host to
// open it is a stub.

import type { JobCheckLog } from "../check-log-sheet";

export type NowKindView = "drone" | "check" | "judge";

/** A diagram the asking Drone drew: the scene it drew, validated where it is drawn. */
export type NowSketchDraft = { scene: unknown };

/** What is asking, drawn on the left while the ask has no sketch: its live output, what it changed, and for a Judge the product and checks. */
export type NowAskerDraft = {
  name: string;
  of: "drone" | "judge";
  step?: string;
  state: "running" | "waiting";
  actions: readonly string[];
  tail: readonly string[];
  changed?: readonly { path: string; change: "added" | "changed" | "removed"; asking?: boolean; diff?: readonly string[] }[];
  product?: { title: string; lines: readonly string[] };
  checks?: readonly { name: string; state: "passed" | "failed" | "running"; tail?: readonly string[] }[];
};

export type NowAskView =
  | { key: string; kind: "plan"; decisions: readonly { id: string; question: string; options: readonly { id: string; label: string; sketch?: NowSketchDraft }[]; sketch?: NowSketchDraft }[]; asker?: NowAskerDraft }
  | { key: string; kind: "judge" | "drone"; name: string; text: string; target?: string; sketch?: NowSketchDraft; asker?: NowAskerDraft };

/** A quick act on a row. The host's handler is a stub on the mock. */
export type NowActView = { key: string; glyph: "retry" | "skip" | "skip_all" | "redirect" | "retry_step"; said: string };

export type NowWaitingView = {
  key: string;
  kind: "resource" | "job" | "transition" | "step";
  text: string;
  step?: { id: string; name: string };
  /** The Job a `job` row opens. */
  target?: string;
};

export type NowIssueView = {
  key: string;
  of: NowKindView;
  name: string;
  text: string;
  said: string;
  step?: { id: string; name: string };
  acts?: readonly NowActView[];
  target?: string;
};

export type NowRunningView = {
  key: string;
  of: NowKindView;
  name: string;
  line?: string;
  /** The canvas step it belongs to. */
  step?: { id: string; name: string };
  /** The last few lines of its output, oldest first. */
  tail?: readonly string[];
  state: "running" | "passed" | "failed";
  acts?: readonly NowActView[];
  /** A Drone's id, or a Check's file. */
  target?: string;
  /** Where a real Check keeps its log. Wins over `target`, which a mock Check names by file. */
  log?: JobCheckLog;
};

export type NowView = {
  asks?: readonly NowAskView[];
  issues?: readonly NowIssueView[];
  running?: readonly NowRunningView[];
  waiting?: readonly NowWaitingView[];
};

type Placed = readonly { step?: { id: string } | undefined }[] | undefined;

/**
 * Every step the panel's rows belong to. **The canvas keeps these lit and stands the rest back**
 * (owner, 8 Oct 2026); none named, nothing stands back.
 */
export function litSteps(view: { running?: Placed; issues?: Placed; waiting?: Placed } | undefined): ReadonlySet<string> {
  return new Set([...(view?.running ?? []), ...(view?.issues ?? []), ...(view?.waiting ?? [])].flatMap((one) => (one.step === undefined ? [] : [one.step.id])));
}
