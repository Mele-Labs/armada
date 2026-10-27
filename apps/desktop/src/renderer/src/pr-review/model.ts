// What the review surface holds of a pull request, and the one store both of its homes read. **The lists, a pull
// request's page, its threads and the review he writes are Fleet's** (`@armada/protocol`, `pull-reviews.ts`), and they
// borrow the pull request panel's page (`PullRequestPage`) for the conversation, the checks and the commits. **What an
// agent says is Fleet's too**: the explanation, the accounts, the answers and what a tool finds are runs
// (`review-runs.ts`), mapped into these shapes in `runs.ts`. So is Natural Search, a run across every repository.
//
// A pull request is read as what it makes happen, not as the files it touches: an explanation, then themes,
// and each theme's changes in the order they were made, with why. A hunk is the evidence for a change, and a
// path is only a caption on it. A line is named by its place in `Hunk.lines`, since the diff draws no numbers.

import type { DiffLine } from "@armada/components";
import type { DiffSide, PullRequestFilePatches, PullRequestPage, PullReviewHolder, PullReviewThread, PullReviewThreadComment, RememberedPlace, ReviewPlace, ReviewRun, ReviewSeverity, ReviewWalk, ReviewWalkState, WalkPlaceFound } from "@armada/protocol";

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

/**
 * What he asked a Drone to look for, in his own words, saved as a list of its own. **Fleet keeps it** under the words, lower cased
 * and single spaced, and `id` is that name behind `search:` so no list can share it. `words` are his own while this window
 * has them, and the kept name after a restart.
 */
export type Find = { id: string; words: string; run: AgentRun; hits: readonly FindHit[] };

/** One hunk of one file: the evidence for a change. */
export type Hunk = { path: string; lines: DiffLine[] };

/** One thing done, in the order it was done, and why. `at` is the place the explanation named; its hunk is cut from the file's patch once that is read. */
export type Change = { id: string; title: string; why: string; hunks: Hunk[]; at?: ReviewPlace };

/** A set of changes that make one thing happen. */
export type Theme = { id: string; title: string; purpose: string; changes: Change[] };

export type Explanation = { summary: string; themes: Theme[] };

export type ThreadComment = PullReviewThreadComment;

/** A thread the forge holds, named by its place in a file. Resolving it is the forge's own act, so it is not part of the pending review. */
export type Thread = PullReviewThread;

/**
 * One of his words not yet posted, named by the place it is on: a line of a file, or the file alone. A reply carries the
 * thread it answers. **Drafts leave only with Submit**, all together, which is the forge's own pending review.
 */
export type Draft = { id: string; path: string; line?: number; side?: DiffSide; body: string; thread?: string };

/** Where a review tool's definition came from, as a workflow's does: Armada's, his Kit's, or the repository's. */
export type ToolSource = "armada" | "kit" | "repository";

/** What a tool is drawn with. Armada's five have a mark each; one anybody adds takes the clipboard. */
export type ToolMark = "risks" | "bugs" | "breaks" | "comments" | "walk" | "custom";

/** What a definition is for: a tool that finds things, or an explanation that tells him one thing about the pull request. */
export type DefKind = "tool" | "explanation";

/** A definition file Fleet read and left out, and why. */
export type LeftOut = { file: string; source: ToolSource; said: string };

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
  /** Kit's, hiding the repository's tool of the same id that says something else. */
  hides?: ToolSource;
};

/**
 * Where a step of How to Test came from. **The author's own directions come first**, in the description. Where there are none, or
 * they do not cover a changed area, the nearest docs to the changed code: READMEs and agent files (`AGENTS.md`) up
 * the tree from each changed path, and the docs they point to. A doc is named by its path and opens at its address.
 */
export type StepSource = { from: "description" } | { from: "doc"; path: string; address: string };

/**
 * What an explanation says, in the shape that kind of explanation has. How it works is the themes view, built from
 * `ReviewPr.explanation`. The rest are these (`accountOf` in `runs.ts`). A kind a repository adds that has no shape of its own is `text`.
 */
export type Account =
  | { shape: "change"; rows: { what: string; before: string; after: string }[] }
  | { shape: "steps"; steps: { do: string; look: string; code?: string; from?: StepSource }[] }
  | { shape: "reason"; problem: string; cost: string; links: { label: string; address: string }[] }
  | { shape: "text"; body: string };

/** A question he asked about one pull request, in his own words. It is kept under them, for the commit it read. `answer` is Markdown, once answered. */
export type Asked = { id: string; words: string; run: AgentRun; answer?: string };

/** Armada's own note on a place in the change, from a tool he pressed. **Never posted to the forge.** */
export type Finding = {
  /** The run's key and the finding's place in its answer, so the same finding keeps its id. */
  id: string;
  tool: string;
  /** The change it is about is the one that holds this place (`ownerOf`). */
  at: ReviewPlace;
  severity: ReviewSeverity;
  title: string;
  body: string;
  dismissed: boolean;
  /** Sent to the Session that holds the pull request. */
  sent: boolean;
};

export type Verdict = "comment" | "approve" | "request_changes";

/** `told` is the Session a Request changes went to as well, by title. */
export type Submission = { verdict: Verdict; body: string; comments: number; at: string; told?: string };

/**
 * A run of an agent: asked for, underway, read, or failed. **A finished run is kept** for the commit it ran on: pressing again shows
 * it with no run, and only Run again spends one. A failed run is never a result: `said` is Fleet's reason.
 */
export type AgentRun = { state: "idle" } | { state: "running"; since: number } | { state: "done"; at: number; head: string } | { state: "failed"; said: string; at: number; head: string };

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
  /** The commit the branch is at, as the forge last said (the whole of it once the page is read). A run's result is kept for the commit it ran on. */
  head: string;
  /** When the forge last changed it, RFC 3339: the lists are ordered by it. */
  updatedAt: string;
  /** Which lists it is on: someone asked for his review, it is assigned to him, or he or an agent of his opened it. */
  lists: readonly PrList[];
  /** The team that was asked, as the forge names it, where he is asked through one. */
  team?: string;
  /** The Session that holds it, where one of his does, as Fleet reads it from the pull request owners. */
  holder?: PullReviewHolder;
  /** The description, as Markdown. Empty until the page is read. */
  body: string;
  /** The forge's page: conversation, checks, commits, the review, and who opened it. **Only what the row says until it is read.** */
  page: PullRequestPage;
  /** How it works, as themes: the how-it-works run's answer, each change's hunk cut from its file once that is read. Empty until it has run. */
  explanation: Explanation;
};

/** What the stage is showing: the explanation, a page of the forge's, or a tool's results. */
export type View = "explanation" | "files" | "checks" | "commits" | (string & {});

/** What the store holds about one pull request beyond what the forge served. */
export type PatchRead = { state: "reading" } | { state: "read"; lines: DiffLine[] } | { state: "said"; said: string };

/** Every file's patch, read together for the Files tab, with the commit they were read on. Fleet's words where it would not. */
export type PatchesRead =
  | { state: "unread" }
  | { state: "reading" }
  | { state: "read"; head: string; files: PullRequestFilePatches["files"] }
  | { state: "said"; said: string };

/** Whether the page of a pull request has been read from the forge, and the words of Fleet's refusal where it would not be. */
export type PageRead = { state: "unread" } | { state: "reading" } | { state: "read" } | { state: "said"; said: string };

/** Where Fleet found the place a change runs: one call that reads the pull request and then what Armada remembers. */
export type WalkFound = { state: "idle" } | { state: "looking" } | { state: "found"; found: WalkPlaceFound } | { state: "said"; said: string };

/** A note taken in a review's walk window that no Session holds. It stays here and is never a comment on the forge. */
export type WalkNote = { at: string; text: string; location: string };

/**
 * What the Walk tool holds of one pull request beyond its kept run. **The walk itself is the run's result** (`web_walk` or
 * `api_walk`, with the place it was written against), so it survives a restart; this is the finding, the question, the place
 * held up for walking, and the notes.
 */
export type WalkWork = {
  find: WalkFound;
  /** Nothing found where it runs, so the question is up. */
  asking: boolean;
  /** The place held up for a walk, as Fleet last said it. */
  held: ReviewWalk | undefined;
  /** The last state it reached before it failed: which setup line broke. */
  reached: ReviewWalkState | undefined;
  /** The walk is being opened, and the window is not up yet. */
  opening: boolean;
  /** Why the walk did not open or the answer was not remembered, in Fleet's words. */
  said: string | undefined;
  notes: readonly WalkNote[];
};

export type PrWork = {
  page: PageRead;
  /** The patches opened, by path. Read from the forge when a file's change is first shown. */
  patches: Readonly<Record<string, PatchRead>>;
  /** The commit the patches were read on, so a page that answers a newer one drops them. */
  patchesHead?: string;
  /** Every file's patch, for the Files tab. Read the first time it is shown, and dropped when the commit moves. */
  every: PatchesRead;
  /** Fleet's refusal of the last review he submitted, in the forge's words. His drafts are still there. */
  refused: string | undefined;
  /** A review is on its way to the forge. */
  submitting: boolean;
  /** The explanations run for this pull request, by kind, each kept for the commit it read. */
  explains: Readonly<Record<string, AgentRun>>;
  /** What each of the other explanations says, by the id of the kind. Held while it runs again. */
  accounts: Readonly<Record<string, Account>>;
  /** The runs as Fleet last said them, by kind and name. The newest of each is the one held. */
  wire: Readonly<Record<string, ReviewRun>>;
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
  walk: WalkWork;
};

/** The sheet that is up for the Walk: asking where this pull request runs, or the places Armada remembers. */
export type WalkSheet = { kind: "ask"; key: string } | { kind: "places" } | undefined;

/** The places Armada remembers, across the repositories it serves, as Fleet last said them. */
export type PlacesRead = { state: "unread" | "reading" | "read"; list: readonly RememberedPlace[]; said: string | undefined };

export type ReviewSnapshot = {
  /** The pull requests as last fetched. Empty until the view is first opened. */
  prs: readonly ReviewPr[];
  fetchedAt: number | undefined;
  fetching: boolean;
  tab: ReviewTab;
  /** Who he is on the forge, as Fleet last said. Empty until the lists are read. */
  me: string;
  /** Fleet's words where it would not read the lists. The rows above are those last held. */
  failed: string | undefined;
  /** The finds he asked a Drone for, each a list in the dropdown. Fleet keeps them. */
  finds: readonly Find[];
  /** `key` of the pull request open in the reader. Kept across a reload, and changed only by opening another. */
  open: string | undefined;
  /** `key`s of the pull requests he opened by link, the latest first. Each is a row above whichever list is chosen, and is kept across a reload. */
  linked: readonly string[];
  /** The Natural Search box is up, in place of the list. */
  searching: boolean;
  /** Asking before anything that spends tokens runs. One setting for all of it, turned off by "Don't ask again". Machine-local. */
  askFirst: boolean;
  /** What waits on his Run, by name, where a confirmation is up. */
  spending: string | undefined;
  /** The tools and explanations Fleet resolved, by repository's manifest id: Kit's over the repository's, which is over Armada's. Comments is not here. */
  tools: Readonly<Record<string, readonly ToolDef[]>>;
  /** The definition files Fleet could not read, by repository's manifest id. */
  leftOut: Readonly<Record<string, readonly LeftOut[]>>;
  work: Readonly<Record<string, PrWork>>;
  sheet: WalkSheet;
  places: PlacesRead;
};
