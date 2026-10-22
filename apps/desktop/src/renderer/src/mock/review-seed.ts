// What the mock forge for the Reviews tab is seeded with. **These are fixtures, not what the store holds**: `review-forge.ts` serves
// them as the wire does (a list, a page, runs, tools), and the store meets only the wire.

import type { RememberedPlace, ReviewRequest, ReviewSeverity, WalkStep } from "@armada/protocol";

import type { Account, Explanation, ReviewPr, ThreadComment, ToolDef } from "../pr-review/model";
import type { MockResponse } from "./review-walk-runner";

/** What a Natural Search would come to. The first whose pattern fits the words answers; one that fits none finds nothing. */
export type FindSeed = { when: string; hits: readonly { key: string; reason: string }[] };

/** The walk the agent would write for a pull request: steps over a page, or requests with the response each gets back. */
export type WalkSeed =
  | { kind: "web"; steps: readonly WalkStep[]; slot: number }
  | { kind: "api"; requests: readonly ReviewRequest[]; responses: readonly MockResponse[]; slot: number };

/** Where a comment sits: a change, and a line of one of its hunks. No line is a comment on the change itself. */
export type Anchor = { change: string; hunk: number; line?: number };

/** A thread of the mock forge, named by the change and the line of its hunk the fixture put it on. The mock Fleet serves it as a `Thread`. */
export type SeedThread = Anchor & { id: string; resolved: boolean; comments: ThreadComment[] };

/** What a question would come to. The first whose pattern fits the words answers; one that fits none says nothing. */
export type AnswerSeed = { when: string; body: string };

/** What a tool finds, by change. Placed at the change's first hunk when the tool is run. */
export type FindingSeed = { change: string; title: string; body: string; severity?: ReviewSeverity };

/**
 * A pull request of the mock forge: what Fleet would serve of it, the threads it holds by the hunk line they were written on, and what
 * an agent would say of it. `explainedBy: "session"` is a pull request a Session of his wrote, whose how it works and why are kept already.
 */
export type MockPr = Omit<ReviewPr, "explanation"> & {
  explanation: Explanation;
  explainedBy: "session" | "agent";
  /** What the other explanations would say, by the id of the kind. A kind with none says nothing. */
  accounts: Readonly<Record<string, Account>>;
  /** What a question he asks about it would come to. */
  answers: readonly AnswerSeed[];
  /** Questions already asked of it, and kept. */
  asked: readonly string[];
  threads: SeedThread[];
};

export type MockTools = {
  armada: readonly ToolDef[];
  /** `~/.armada/review-tools/`, every repository on this machine. */
  kit: readonly ToolDef[];
  /** `.armada/review-tools/`, by repository. */
  repository: Readonly<Record<string, readonly ToolDef[]>>;
  /** Files Fleet could not read, by repository. */
  leftOut?: Readonly<Record<string, readonly { file: string; source: ToolDef["source"]; said: string }[]>>;
};

/** The mock's world: the forge it serves (`mock/slices/reviews.ts`) and what its agent would say. */
export type ReviewSeed = {
  /** Who he is on the forge. */
  me: string;
  prs: readonly MockPr[];
  tools: MockTools;
  /** What each tool comes to, by pull request. A tool with none for a pull request reads clean. */
  results: Readonly<Record<string, Readonly<Record<string, readonly FindingSeed[]>>>>;
  finds: readonly FindSeed[];
  /** The walk written for a pull request, by its key. A pull request with none has nothing to walk. */
  walks: Readonly<Record<string, WalkSeed>>;
  /** The places Armada remembers a change runs, before anything is answered. */
  places: readonly RememberedPlace[];
  /** A commit pushed to a pull request after he first read the list: the next fetch brings it, and results kept for the old head fall out of date. */
  pushes: Readonly<Record<string, { sha: string; headline: string; author: string }>>;
  /** The window starts as a relaunch does: a reading of the lists kept `minutesAgo` minutes ago, without the pull request `without`, and Fleet answers only after `answersAfterMs`. */
  kept?: { minutesAgo: number; without: string; answersAfterMs: number };
};

/** The set Armada carries, as its own place serves it: Comments is not here, since Bridge draws it. */
export const ARMADA_DEFS: readonly ToolDef[] = [
  { id: "risks", kind: "tool", name: "Risks", source: "armada", mark: "risks", agent: true, checks: "What could go wrong that the change does not handle: shared state, ordering, a caller that was not updated." },
  { id: "possible-bugs", kind: "tool", name: "Possible bugs", source: "armada", mark: "bugs", agent: true, checks: "Behaviour that differs from what the change says it does: an off by one, a case the code skips, a state it never resets." },
  { id: "api-breaking-changes", kind: "tool", name: "API breaking changes", source: "armada", mark: "breaks", agent: true, checks: "Public shapes others depend on: wire fields, exported types and signatures, files a caller reads." },
  { id: "walk", kind: "tool", name: "Walk", source: "armada", mark: "walk", agent: true, checks: "Plays the change where it runs, as a walk of a page or a run of requests, from the description, How to Test and the themes." },
  { id: "how-it-works", kind: "explanation", name: "How it works", source: "armada", mark: "custom", agent: true, checks: "How the code does what it does: its changes in the order they were made, each with why." },
  { id: "user-impact", kind: "explanation", name: "User Impact", source: "armada", mark: "custom", agent: true, checks: "How this changes things for someone who uses it: what they see or can do differently, before and after." },
  { id: "how-to-test", kind: "explanation", name: "How to Test", source: "armada", mark: "custom", agent: true, checks: "The steps to try this change, each with what to look for. Follow the author's own testing directions in the description first. Where there are none, or they do not cover a changed area, use the nearest docs up the tree from each changed path: READMEs and AGENTS.md, and the docs they point to. Say where each step came from." },
  { id: "why", kind: "explanation", name: "Why", source: "armada", mark: "custom", agent: true, checks: "The problem, and what it cost, with the issue or incident the pull request names." },
];
