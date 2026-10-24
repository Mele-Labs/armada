// The merge line `armada land` keeps in each repository Fleet serves —
// `crates/ipc/src/merge_line.rs`. Since protocol 22.1; `landed` and `sent_back` since 23.2, and
// one Check's log on its own socket since 23.7.
//
// Read once per connection off `get_merge_lines`, and replaced whole by every
// `merge_lines.changed`. The header rules in `events.ts` hold: hand-written,
// and every closed set left as `string`.

import type { ProtocolId } from "./connection";
import type { Requester } from "./requester";

/** Every served repository that has a line. One nobody has run `armada land` in is not here. */
export type MergeLines = { lines: MergeLine[] };

/** One repository's line. */
export type MergeLine = {
  /** The repository's root, as `list_repositories` names it. */
  root: string;
  /** In place order. Empty is a line nobody is in. */
  line: MergeLineRow[];
  /** The newest few that left it, newest first. What a Bridge before 23.2 drew; this one reads the two below. */
  off: MergeLineRow[];
  /** The newest three that landed, newest first. Since 23.2. */
  landed: MergeLineRow[];
  /** Red, conflict or stopped and not back in line, in the last three days, newest first. Since 23.2. */
  sent_back: MergeLineRow[];
  /** Main's CI and the open pull requests, from the forge. Absent where Fleet has read neither. Since 23.41. */
  hub?: MergeLineHub;
};

/**
 * What Fleet read off the forge for one repository, beside the queue: where main's CI stands and
 * every open pull request. `crates/ipc/src/merge_hub.rs`. Since 23.41.
 */
export type MergeLineHub = {
  /** Absent where Fleet has not read main yet. */
  main?: MainStanding;
  /** Newest first, as the forge lists them. Absent is none open. */
  pull_requests?: HubPullRequest[];
  /** The newest pull requests merged into the base, newest first. Absent from a Fleet before 23.42. */
  merged?: HubMerged[];
  /** The Session working on main's red. */
  fixing_session?: HubSession;
  /**
   * Another machine's claim on main's red, from the central store. Absent while nobody else holds it, where
   * the repository's switch is off, and whenever the store could not be read.
   */
  fixing_elsewhere?: FixingElsewhere;
  /** The checkout Fleet serves, where main moved past it and Fleet would not fast-forward it. Absent while it has main's tip. */
  checkout?: HeldCheckout;
  /** When every forge call of Fleet's last visit to this repository was answered: how old the rest of the hub is. Absent until one visit has been. */
  forge_read_at?: string;
  /** When the forge first failed to answer a call of a visit, since the last visit answered whole. Absent while it answers. */
  quiet_since?: string;
};

/** What stops the served checkout catching up to main. Fleet never forces, stashes or switches it. */
export type HeldCheckout = {
  base: string;
  /** The branch checked out, where it is not `base`. `HEAD` when detached. */
  on?: string;
  /** Tracked files with uncommitted changes. Absent is none. */
  uncommitted?: string[];
  /** Those of `uncommitted` that main also changed, so an autostash could stop on a conflict. */
  clashing?: string[];
  behind: number;
  /** Above zero is diverged. */
  ahead: number;
  /** The pull request Fleet opened from the uncommitted files, on the person's press of "Pull request". Absent until then. */
  pull_request?: { number: number; auto_merge: boolean };
};

/** Who holds main's red on another machine: a Session by id, the machine, and since when. No title: only the holder can say. */
export type FixingElsewhere = {
  machine: string;
  holder: { kind: "session"; id: string };
  since: string;
};

/** One pull request recently merged. Since 23.42. */
export type HubMerged = {
  number: number;
  title: string;
  branch: string;
  url: string;
  author?: string;
  merged_at: string;
  /** The merge commit, whole. */
  commit?: string;
  /** The CI run on its merge commit, on main itself, apart from the pull request's own checks. Absent where nothing ran or none was read yet. Since 23.44. */
  main_run?: MainRun;
};

/**
 * The CI run on one commit of main. `state` is `passed`, `running` or `failed`. A press on a failed
 * job opens its log on the Check log socket under the branch `main@<commit>`. Since 23.44.
 */
export type MainRun = {
  state: string;
  /** Each job that failed, by the forge's name. */
  failed?: string[];
};

/** `catch_up_checkout`'s body: pull a held checkout level with its base. Names the repository and no command: Fleet reads the hold again and runs the pull itself. */
export type CatchUpCheckout = {
  manifest_id: string;
};

/** `checkout_to_pull_request`'s body: send a held checkout's uncommitted files up as a pull request. Names the repository and no path: Fleet reads the hold again and sends the files it lists. */
export type CheckoutToPullRequest = {
  manifest_id: string;
};

/** `checkout_to_pull_request`'s answer: the send was accepted, and nothing more. The pull request arrives on the hold (`HeldCheckout.pull_request`) when it has opened; a failure arrives as `act.failed`. */
export type CheckoutSentAsPullRequest = Record<string, never>;

/** `checkout_diff`'s query: which uncommitted file of a held checkout to read. */
export type CheckoutDiffAsk = {
  manifest_id: string;
  /** A path from the hold's `uncommitted` list. Fleet refuses any other. */
  path: string;
};

/** `checkout_diff`'s answer: the file's unified diff against HEAD in the served checkout. Empty where the file no longer differs. */
export type CheckoutDiff = {
  diff: string;
  /** Set where Fleet cut `diff` short of what git printed, at a line or byte bound. Absent otherwise. */
  cut?: boolean;
};

/** `catch_up_checkout`'s answer. A pull git refused or left conflicted is still an answer, with a non-zero `code`. */
export type CheckoutCaughtUp = {
  /** git's exit code. `1` where git exited clean and the stashed work would not go back on; the work is in git's stash. */
  code: number;
  /** What git printed. */
  output: string;
};

/**
 * `main_ci_state` is `green`, `red`, `running` or `nothing_ran`. A red a fix is still running for
 * stays `red`. `nothing_ran` is not green: nothing was proved.
 */
export type MainStanding = {
  state: string;
  commit: string;
  read_at: string;
  /** When main first read red, kept while it stays red. */
  red_since?: string;
  /** Each CI job that failed, in the forge's order. */
  failed?: MainFailedJob[];
  /** The pull request that turned main red, where the forge named one. */
  merge?: MainMerge;
  /** The commit the red was read at, whole. `red` only. Since 23.44. */
  red_commit?: string;
  /**
   * Newer commits whose CI is still running, newest first. Main's state is the newest commit whose
   * run has finished. On `red`, non-empty is a held red: a newer run may already have fixed it, so
   * Fleet refuses `fix_main` (`fleet.main_checks_running`) and the band is caution with no buttons.
   * On `green` it is only checks running. Absent from a Fleet before 23.44. Since 23.44.
   */
  checking?: MainChecking[];
};

/** A commit on main, newer than the red one, whose CI is still running. Since 23.44. */
export type MainChecking = {
  commit: string;
  /** The pull request that merged it, where the forge named one. */
  pull_request?: MainMerge;
};

export type MainFailedJob = {
  /** The job's name as the forge reports it. */
  name: string;
  /** The Manifest Check it maps to. Absent is ordinary. */
  check?: string;
  log_url?: string;
  /** The tests its log names, in the order printed. Absent where none could be read. */
  tests?: string[];
};

/** A Session, by its id and the title the ledger holds for it. Since 23.80. */
export type HubSession = { id: string; title?: string };

export type MainMerge = {
  number: number;
  url?: string;
  branch?: string;
  /** The Session that held the pull request or its branch. */
  session?: HubSession;
};

/** `ci` is `passed`, `running`, `failed` or `waiting_on_main`: failed only on the jobs main is failing. */
export type HubPullRequest = {
  number: number;
  title: string;
  branch: string;
  url: string;
  author?: string;
  /** Absent where nothing has run on it. */
  ci?: string;
  /** Where the merge queue holds it, and `waiting_for_ci` where auto-merge waits on its checks. Absent otherwise. Since 23.70. */
  queue?: HubQueue;
  /** When the forge says it was opened, RFC 3339. Absent from an older Fleet. */
  opened_at?: string;
  /** When anything last happened on it, RFC 3339. Absent as `opened_at` is. */
  updated_at?: string;
  /** Still a draft. Absent is ready. */
  draft?: boolean;
  /** `clean`, `conflicting` or `unknown`: the forge still computing, not a conflict. Absent where the listing named none. */
  mergeable?: string;
  /** The forge's review decision: `approved`, `changes_requested`, `review_required`, or `none` for a pull request no rule applies to. Absent where the listing carried none. */
  review?: string;
  /** Lines added and removed, as the forge counts them. */
  additions?: number;
  deletions?: number;
  /**
   * The Session or Job that claimed it, from the central store. Absent where nobody registered it, where the
   * repository's switch is off (rows another machine wrote included), and whenever the store could not be read.
   */
  owner?: PullRequestOwner;
  /**
   * The pull request of an `armada.yml` edit a form sent, which the forge would not turn auto-merge on for, so it waits
   * for a person (`Pending::merge_by_hand`). Absent is any other pull request, or one the forge is going to merge itself.
   */
  merge_by_hand?: boolean;
  /**
   * Opened by the person using this Fleet: its author is the login Armada is signed in as for the repository or the one
   * the forge's command line is signed in as, or neither is known. Absent is mine, and what an older Fleet sends. One
   * that is not is drawn only in the merge queue, quietly, and raises no call.
   */
  mine?: boolean;
};

/** Who registered a pull request: a kind, a short id and a machine's name, and nothing about the pull request. */
export type PullRequestOwner = {
  kind: "session";
  /** The Session's short id. */
  id: string;
  /** The host's name, or `machine.name` where two hosts share one. */
  machine: string;
  /** The row was written by this machine. */
  this_machine: boolean;
};

/** `state` is `waiting_for_ci`, `in_queue` (a state this build does not know), `queued`, `awaiting_checks`, `mergeable` or `unmergeable`. Since 23.70. */
export type HubQueue = {
  state: string;
  /** 1 is next to merge. Absent for `waiting_for_ci`. */
  position?: number;
};

/** One branch, in line or just off it. */
export type MergeLineRow = {
  branch: string;
  /** 1-based. Absent once off the line. */
  place?: number;
  /** `settled`, landed only: how it ended, `merged` or `closed_unmerged`. Since 23.2. */
  pull_request?: { number: number; url: string; settled?: string };
  /** `land_state` in the generated vocabulary. */
  state: string;
  /** The runner's own words, while gating or merging. */
  doing?: string;
  /** The batch, by its first member's branch. */
  batch?: string;
  /** Landed: the merge commit, whole. */
  merge_commit?: string;
  failed?: string[];
  conflicts?: string[];
  /** Gating, red and stopped: each Check the turn runs, as it stands. Since 23.2. */
  checks?: MergeLineCheck[];
};

/** One Check a turn runs. `state` is `waiting`, `running`, `passed`, `failed` or `timed_out`. */
export type MergeLineCheck = {
  name: string;
  state: string;
  /** The merge line, for the entry's branch. Since 23.40; **absent reads as `outside`**. */
  requester?: Requester;
  /**
   * When the runner began it, off the line's own state. **Absent while it waits**,
   * and on a line state written before the field. Since 23.40.
   */
  started_at?: string;
};

/**
 * One message on a merge line Check's log socket, `observe_land_check`. Since 23.7.
 *
 * **`OutputMessage`'s three with its own first one**: a turn's Check belongs to no Job, so the
 * opening names the line. A finished Check's log opens too, sends what it holds and closes
 * `finished`, so this one socket reads a Check that is running and one that has ended.
 */
export type LandOutputMessage =
  | ({ message: "opened" } & LandOutputOpened)
  | ({ message: "lines" } & { lines: string[] })
  | ({ message: "closed" } & { because: string });

/** Whose log this is, by the line's own names. No path: the runner's logs stay off the wire. */
export type LandOutputOpened = {
  protocol_id: ProtocolId;
  root: string;
  branch: string;
  name: string;
  /** Older lines the first read left out, because the window is bounded. */
  skipped: number;
};

/** Which merge line Check's log is asked for: the line's three names, and nothing else. */
export type LandCheckAt = { root: string; branch: string; check: string };

/**
 * The merge line Check's log a window is reading, as main holds it. `FollowedLog`'s shape, by the
 * line's names. **One at a time**: opening another replaces the first.
 */
export type FollowedLandLog =
  | { state: "none" }
  | ({ state: "opening" } & LandCheckAt)
  | ({
      state: "following";
      /** The file's own line number of `lines[0]`, counted from one. */
      fromLine: number;
      /** The newest lines, oldest first, bounded. */
      lines: string[];
      /** Why it ended, `finished`, `unreadable` or `broke`, or absent while it is still arriving. */
      ended?: string;
    } & LandCheckAt)
  | ({ state: "failed"; detail: string } & LandCheckAt);
