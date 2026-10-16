// What the review surface holds of a pull request, and the one store both of its homes read. **A mock:
// nothing here is served by Fleet yet**, so the shapes are the ones a Fleet read would have to fill, and
// they borrow the pull request panel's page (`PullRequestPage`) rather than a second set of words for the
// conversation, the checks and the commits.
//
// A pull request is read as what it makes happen, not as the files it touches: an explanation, then themes,
// and each theme's changes in the order they were made, with why. A hunk is the evidence for a change, and a
// path is only a caption on it. A line is named by its place in `Hunk.lines`, since the diff draws no numbers.

import type { DiffLine } from "@armada/components";
import type { PullRequestPage } from "@armada/protocol";

/**
 * The lists a pull request can be on. **He is asked to review in two ways**: directly (`requested`), or through a team
 * he is on (`team`, and the row names the team). `assigned` is the forge's own assignee, and `mine` is his agents'.
 * A pull request can be on several, and on none, where only a find turns it up.
 */
export type PrList = "requested" | "team" | "assigned" | "mine";

/** The list the stage's left column shows: a `PrList`, or the id of a find he saved. */
export type ReviewTab = string;

/** A pull request a find turned up, and the one line that says why it matched. */
export type FindHit = { key: string; reason: string };

/** What he asked a Drone to look for, in his own words, saved as a list of its own. */
export type Find = { id: string; words: string; run: AgentRun; hits: readonly FindHit[] };

/** What a find would come to. The first whose pattern fits the words answers; one that fits none finds nothing. */
export type FindSeed = { when: string; hits: readonly FindHit[] };

/** One hunk of one file: the evidence for a change. */
export type Hunk = { path: string; lines: DiffLine[] };

/** One thing done, in the order it was done, and why. */
export type Change = { id: string; title: string; why: string; hunks: Hunk[] };

/** A set of changes that make one thing happen. */
export type Theme = { id: string; title: string; purpose: string; changes: Change[] };

export type Explanation = { summary: string; themes: Theme[] };

/** Where a comment sits: a change, and a line of one of its hunks. No line is a comment on the change itself. */
export type Anchor = { change: string; hunk: number; line?: number };

export type ThreadComment = { id: string; author: string; at: string; body: string };

/** A thread the forge holds. Resolving it is the forge's own act, so it is not part of the pending review. */
export type Thread = Anchor & { id: string; resolved: boolean; comments: ThreadComment[] };

/**
 * One of his words not yet posted. A reply carries the thread it answers. **Drafts leave only with Submit**,
 * all together, which is the forge's own pending review.
 */
export type Draft = Anchor & { id: string; body: string; thread?: string };

/** Where a review tool's definition came from, as a workflow's does: Armada's, his Kit's, or the repository's. */
export type ToolSource = "armada" | "kit" | "repository";

/** What a tool is drawn with. Armada's four have a mark each; one anybody adds takes the clipboard. */
export type ToolMark = "risks" | "bugs" | "breaks" | "comments" | "custom";

/** What a definition is for: a tool that finds things, or an explanation that tells him one thing about the pull request. */
export type DefKind = "tool" | "explanation";

/**
 * A review tool: a name and what it checks, as a prompt. **Shared like a skill**: a file in `~/.armada/review-tools/`
 * is his, in every repository on the machine, and one in `.armada/review-tools/` is the repository's, for everyone
 * on it. A later place replaces an earlier one of the same name.
 */
export type ToolDef = {
  id: string;
  /** **An explanation comes from the same three places a tool does** and is merged the same way. */
  kind: DefKind;
  name: string;
  source: ToolSource;
  mark: ToolMark;
  /** What it checks, or for an explanation what it explains. */
  checks: string;
  /** Runs an agent when pressed. Comments does not: it is the forge's conversation. */
  agent: boolean;
  /** The source a tool of the same name was replaced from, where it replaced one. */
  replaces?: ToolSource;
};

/**
 * What an explanation says, in the shape that kind of explanation has. How it works is the themes view, built from
 * `ReviewPr.explanation`. The rest are these. A kind a repository adds that has no shape of its own is `text`.
 */
export type Account =
  | { shape: "change"; rows: { what: string; before: string; after: string }[] }
  | { shape: "steps"; steps: { do: string; look: string; code?: string }[] }
  | { shape: "reason"; problem: string; cost: string; links: { label: string; address: string }[] }
  | { shape: "text"; body: string };

/** A question he asked about one pull request, in his own words. It is kept under them, for the commit it read. */
export type Asked = { id: string; words: string; run: AgentRun };

/** What a question would come to. The first whose pattern fits the words answers; one that fits none says nothing. */
export type AnswerSeed = { when: string; body: string };

/** Armada's own note on a change, from a tool he pressed. **Never posted to the forge.** */
export type Finding = {
  id: string;
  tool: string;
  change: string;
  title: string;
  body: string;
  dismissed: boolean;
  /** Sent to the Session that holds the pull request. */
  sent: boolean;
};

export type Verdict = "comment" | "approve" | "request_changes";

/** `told` is the Session a Request changes went to as well, by title. */
export type Submission = { verdict: Verdict; body: string; comments: number; at: string; told?: string };

/** A run of an agent: asked for, underway, or read. */
/** **A finished run is kept** for the commit it ran on: pressing again shows it with no run, and only Run again spends one. */
export type AgentRun = { state: "idle" } | { state: "running"; since: number } | { state: "done"; at: number; head: string };

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
  /** The commit the branch is at, as the forge last said. A run's result is kept for the commit it ran on. */
  head: string;
  /** Which list it is on: someone asked for his review, or an agent of his opened it. */
  lists: readonly PrList[];
  /** The team that was asked, as the forge names it, where he is asked through one. */
  team?: string;
  /** The Session that holds it, where one of his does. */
  holder?: { id: string; title: string };
  /** The description, as Markdown. */
  body: string;
  /** The forge's page: conversation, checks, commits, the review, and who opened it. */
  page: PullRequestPage;
  /**
   * How it works, as themes. **Where an explanation comes from**: a Session of his that wrote the pull request knows
   * how it works and why it was needed, so those two are there at open (`session`). Every other explanation, and every
   * explanation of anybody else's pull request, is read by an agent when he first chooses it (`agent`).
   */
  explanation: Explanation;
  explainedBy: "session" | "agent";
  /** What the other explanations would say, by the id of the kind. A kind with none says nothing. */
  accounts: Readonly<Record<string, Account>>;
  /** What a question he asks about it would come to. */
  answers: readonly AnswerSeed[];
  /** Questions already asked of it, and kept. */
  asked: readonly string[];
  threads: Thread[];
};

/** What a tool finds, by change. Placed when the tool is run. */
export type FindingSeed = { change: string; title: string; body: string };

export type ToolDefs = {
  armada: readonly ToolDef[];
  /** `~/.armada/review-tools/`, every repository on this machine. */
  kit: readonly ToolDef[];
  /** `.armada/review-tools/`, by repository. */
  repository: Readonly<Record<string, readonly ToolDef[]>>;
};

export type ReviewSeed = {
  /** Who he is on the forge. */
  me: string;
  prs: readonly ReviewPr[];
  tools: ToolDefs;
  /** What each tool comes to, by pull request. A tool with none for a pull request reads clean. */
  results: Readonly<Record<string, Readonly<Record<string, readonly FindingSeed[]>>>>;
  finds: readonly FindSeed[];
  /** A commit pushed to a pull request after he first read the list: the next fetch brings it, and results kept for the old head fall out of date. */
  pushes: Readonly<Record<string, { sha: string; headline: string; author: string }>>;
};

/** What the stage is showing: the explanation, a page of the forge's, or a tool's results. */
export type View = "explanation" | "checks" | "commits" | (string & {});

/** What the store holds about one pull request beyond what the forge served. */
export type PrWork = {
  /** The explanations run for this pull request, by kind, each kept for the commit it read. */
  explains: Readonly<Record<string, AgentRun>>;
  /** The explanation shown under the description, if he has chosen one. */
  kind: string | undefined;
  /** The questions he asked of this pull request, each kept under his words. */
  asks: readonly Asked[];
  view: View;
  runs: Readonly<Record<string, AgentRun>>;
  /** Changes whose hunks are open. */
  opened: readonly string[];
  drafts: readonly Draft[];
  threads: readonly Thread[];
  findings: readonly Finding[];
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
  /** The finds he asked a Drone for, each a list in the dropdown. */
  finds: readonly Find[];
  /** `key` of the pull request open in the reader. */
  open: string | undefined;
  /** The Cockpit has swapped into review mode. */
  mode: boolean;
  /** The tools, by place. Kit and the repositories' grow when he saves one. */
  armada: readonly ToolDef[];
  kit: readonly ToolDef[];
  repository: Readonly<Record<string, readonly ToolDef[]>>;
  work: Readonly<Record<string, PrWork>>;
};
