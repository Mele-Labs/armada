// What the review surface holds of a pull request, and the one store both of its homes read. **A mock:
// nothing here is served by Fleet yet**, so the shapes are the ones a Fleet read would have to fill, and
// they borrow the pull request panel's page (`PullRequestPage`) rather than a second set of words for the
// conversation, the checks and the commits.
//
// A line is named by its place in `ReviewFile.lines`. The diff draws no numbers, so none is derived here.

import type { DiffLine } from "@armada/components";
import type { PullRequestFile, PullRequestPage } from "@armada/protocol";

/** The two tabs that narrow the list. */
export type ReviewTab = "requested" | "mine";

export type ReviewFile = {
  path: string;
  status: PullRequestFile["status"];
  additions: number;
  deletions: number;
  /** The patch, split into the lines git wrote. Empty for a file the forge sent no patch for. */
  lines: DiffLine[];
  /** Fleet's sentence where there is no patch. */
  said?: string;
};

/** Where a comment sits: a file, and a line of it by place. */
export type Anchor = { path: string; line: number };

export type ThreadComment = { id: string; author: string; at: string; body: string };

/** A thread the forge holds, on a line. Resolving it is the forge's own act, so it is not part of the pending review. */
export type Thread = Anchor & { id: string; resolved: boolean; comments: ThreadComment[] };

/**
 * One of his words not yet posted. A reply carries the thread it answers. **Drafts leave only with Submit**,
 * all together, which is the forge's own pending review.
 */
export type Draft = Anchor & { id: string; body: string; thread?: string };

export type FindingKind = "bug" | "risk" | "question";

/** Armada's own note on a line, from an agent review he asked for. **Never posted to the forge.** */
export type Finding = Anchor & {
  id: string;
  kind: FindingKind;
  title: string;
  body: string;
  dismissed: boolean;
  /** Sent to the Session that holds the pull request. */
  sent: boolean;
};

export type Verdict = "comment" | "approve" | "request_changes";

export type Submission = { verdict: Verdict; body: string; comments: number; at: string };

/** An agent review of one pull request: asked for, underway, or read. */
export type AgentRun = { state: "idle" } | { state: "running"; since: number } | { state: "done"; at: number };

export type ReviewPr = {
  /** `owner/name#number`. */
  key: string;
  /** `owner/name`. */
  repository: string;
  manifestId: string;
  number: number;
  title: string;
  branch: string;
  address: string;
  draft: boolean;
  checks: "passed" | "failed" | "running";
  /** Which list it is on: someone asked for his review, or an agent of his opened it. */
  tab: ReviewTab;
  /** The Session that holds it, where one of his does. */
  holder?: { id: string; title: string };
  /** The description, as Markdown. */
  body: string;
  /** The forge's page: conversation, checks, commits, the review, and who opened it. */
  page: PullRequestPage;
  files: ReviewFile[];
  threads: Thread[];
};

/** What an agent review would find, by place. Placed on a line when the review is asked for. */
export type FindingSeed = { path: string; line: number; kind: FindingKind; title: string; body: string };

export type ReviewSeed = {
  /** Who he is on the forge. */
  me: string;
  prs: readonly ReviewPr[];
  /** What a review by an agent comes to, by pull request. A pull request with none reads clean. */
  agent: Readonly<Record<string, readonly FindingSeed[]>>;
};

/** What the store holds about one pull request beyond what the forge served. */
export type PrWork = {
  viewed: readonly string[];
  drafts: readonly Draft[];
  threads: readonly Thread[];
  findings: readonly Finding[];
  run: AgentRun;
  submissions: readonly Submission[];
  /** The page's review state, which a submitted review moves. */
  review: PullRequestPage["review"];
};

export type ReviewSnapshot = {
  /** The pull requests as last fetched. Empty until the view is first opened. */
  prs: readonly ReviewPr[];
  fetchedAt: number | undefined;
  fetching: boolean;
  tab: ReviewTab;
  /** `key` of the pull request open in the reader. */
  open: string | undefined;
  /** The Cockpit has swapped into review mode. */
  mode: boolean;
  work: Readonly<Record<string, PrWork>>;
};
