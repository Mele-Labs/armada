// What an agent says about one pull request, and the review tools it is asked with.
// `crates/ipc/src/review_runs.rs`, `docs/concepts/pull-review.md`, *Agent runs*.
//
// The header rules in `protocol.ts` hold here.

import type { DiffSide } from "./pull-reviews";
import type { ReviewMatch, ReviewRequest, WalkPlace } from "./review-walks";
import type { WalkStep } from "./sessions";

/** Where a review tool was read from. Kit's replaces the repository's of the same id, and either replaces Armada's. */
export type ReviewToolSource = "armada" | "kit" | "repository";

/** Where a save writes. Armada's own are carried in the binary. */
export type ReviewToolScope = "kit" | "repository";

/** What a definition is for: a tool finds things, an explanation tells one thing about the pull request. */
export type ReviewToolKind = "tool" | "explanation";

/** One review tool or explanation, resolved: the most specific place that defines its id. */
export type ReviewTool = {
  /** The name lower cased with its words joined by a dash, which is also the file's stem. */
  id: string;
  kind: ReviewToolKind;
  name: string;
  /** What it checks, or for an explanation what it explains. The shape of the answer is Fleet's. */
  prompt: string;
  source: ReviewToolSource;
  /** The place a definition of the same id was replaced from, where this one replaced one. */
  replaces?: ReviewToolSource;
  /** `repository` where this is Kit's and hides a repository definition of the same id that says something else. Absent where the two are the same. */
  hides?: ReviewToolSource;
};

/** A file Fleet read and left out, and why. Never fatal: the others stand. */
export type ReviewToolLeftOut = {
  /** Absolute. */
  file: string;
  source: ReviewToolSource;
  said: string;
};

/** `list_review_tools` (`GET /review_tools/:repository`), and what `save_review_tool` answers. Comments is not here: Bridge draws it. */
export type ReviewTools = {
  tools: ReviewTool[];
  left_out?: ReviewToolLeftOut[];
};

/** `save_review_tool`'s body (`POST /review_tools/:repository`). Helm only; Bridge's Add sheet is the person's own press. */
export type SaveReviewTool = {
  scope: ReviewToolScope;
  kind: ReviewToolKind;
  name: string;
  prompt: string;
  /** Replace the file already holding this id in `scope`. Without it that is a 422. */
  overwrite?: boolean;
};

/** What a run is of. */
export type ReviewRunKind = "tool" | "explanation" | "ask" | "search";

/** How far a run has got. `failed` is never a result. */
export type ReviewRunState = "queued" | "running" | "done" | "failed";

/** `start_review_run`'s body (`POST /review_runs/:repository`). */
export type StartReviewRun = {
  number: number;
  kind: ReviewRunKind;
  /** The id of the tool or explanation, or for `ask` the question's words. */
  name: string;
  /** Run it again, whatever is kept. Without it a kept result is answered as it stands. */
  again?: boolean;
  /** Where the change runs. Required for the tool `walk` and ignored by every other name. */
  place?: WalkPlace;
};

/** A place in the change: a range of lines of a file, on one side of the diff. */
export type ReviewPlace = {
  /** A file the pull request changes. */
  path: string;
  /** `right` is the head's numbering, `left` the base's, as the forge names a line. */
  side: DiffSide;
  start: number;
  /** At least `start`. */
  end: number;
};

/** One thing done, in the order it was done, and why. */
export type ReviewChange = {
  title: string;
  why: string;
  /** The hunk that is the evidence for it. */
  at: ReviewPlace;
};

/** A set of changes that make one thing happen. */
export type ReviewTheme = {
  title: string;
  purpose: string;
  changes: ReviewChange[];
};

/** One thing a person sees or can do differently. */
export type ReviewImpact = {
  what: string;
  before: string;
  after: string;
};

/** Where a step of How to Test came from: the author's directions come first. */
export type ReviewStepSource = { from: "description" } | { from: "doc"; path: string };

/** One step of How to Test. */
export type ReviewStep = {
  /** What to do. */
  action: string;
  /** What to look for. */
  look: string;
  code?: string;
  source: ReviewStepSource;
};

/** An issue or page the pull request names. */
export type ReviewLink = {
  label: string;
  /** An http or https address. */
  address: string;
};

export type ReviewSeverity = "low" | "medium" | "high";

/** One thing a review tool found, pinned to a place in the change. Armada's own note: never posted. */
export type ReviewFinding = {
  title: string;
  /** Markdown. */
  body: string;
  severity: ReviewSeverity;
  at: ReviewPlace;
};

/** What a run came to, in the shape its kind has. Validated by Fleet before it is kept. */
export type ReviewResult =
  | { shape: "themes"; summary: string; themes: ReviewTheme[] }
  | { shape: "impact"; rows: ReviewImpact[] }
  | { shape: "steps"; steps: ReviewStep[] }
  | { shape: "reason"; problem: string; cost: string; links?: ReviewLink[] }
  | { shape: "text"; body: string }
  | { shape: "findings"; findings: ReviewFinding[] }
  /** Natural Search: the open pull requests that matched, each with why. */
  | { shape: "matches"; matches: ReviewMatch[] }
  /** The tool `walk` for a web change: `show_window` steps over the page at `place`. */
  | { shape: "web_walk"; place: WalkPlace; steps: WalkStep[] }
  /** The tool `walk` for an API: one request a step. */
  | { shape: "api_walk"; place: WalkPlace; requests: ReviewRequest[] };

/** One run about one pull request at one commit, going or kept. Carried as `run` by `review_run.changed`. */
export type ReviewRun = {
  manifest_id: string;
  number: number;
  kind: ReviewRunKind;
  /** The tool's or explanation's id, or the question's words. */
  name: string;
  /** The commit it ran on, whole. A reader that holds a newer head marks the run out of date. */
  head: string;
  state: ReviewRunState;
  /** RFC 3339. */
  started_at: string;
  finished_at?: string;
  /** Why a failed run failed, in Fleet's words. */
  failed?: string;
  result?: ReviewResult;
  /** What the vendor billed, in millionths of a dollar. Absent is not zero. */
  cost_micros?: number;
};

/** `get_review_runs` (`GET /review_runs/:repository/:number`): every run kept or going for one pull request, newest first. */
export type ReviewRuns = {
  runs: ReviewRun[];
};

/** The Session of the person's that holds a pull request. */
export type PullReviewHolder = {
  id: string;
  title: string;
};
