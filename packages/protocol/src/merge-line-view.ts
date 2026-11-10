// The merge line as a screen draws it, worked out by Fleet: `get_merge_line_view` and `merge_line_view.changed`.
// Hand-mirrored from `crates/ipc/src/merge_line_view.rs`. `docs/practices/protocol/the-merge-line-fleet-reads.md`.
//
// **Fleet reads, every client draws.** `get_merge_lines` serves what Fleet read of each forge; this is which pull
// requests to list and in what order, which of them call for nobody, and whether main reads green, red or nothing.
// Nothing here is a sentence. The header rules in `protocol.ts` hold: every closed set is left as `string`.

import type {
  FixingElsewhere,
  HeldCheckout,
  HubMerged,
  HubPullRequest,
  HubSession,
  MainChecking,
  MainMerge,
} from "./merge-lines";

/** `get_merge_line_view` (`GET /merge_lines/view`), and what `merge_line_view.changed` carries whole. */
export type MergeLineViews = {
  /** One a served repository whose forge Fleet has read, in `get_merge_lines`' order. */
  lines: MergeLineView[];
};

export type MergeLineView = {
  root: string;
  /**
   * What the repository is called among the ones Fleet serves: its Manifest's name, widened to `parent/folder` and then
   * the whole root only where another repository reads alike.
   */
  name: string;
  /**
   * Main, where it is worth a mark. Absent for a main not read yet, one running with nothing failed, one the forge ran
   * nothing on, and a red that names no failed job.
   */
  main?: MainView;
  /**
   * The pull requests the line draws, in its order: the forge's merge queue by position, then those waiting for ci to
   * join it, then the rest as the forge lists them. Someone else's pull request is listed only while it is in the
   * queue, and quiet (`mine` false): its number, title, address, branch, author and place, with no ci, review,
   * mergeability or hand, and its queue state `in_queue`, so nothing downstream raises a card for it. Absent is none.
   */
  pulls?: PullView[];
  /** The newest pull requests merged, newest first, with the run each started on main. Absent is none. */
  landed?: HubMerged[];
  checkout?: HeldCheckout;
  /** When the forge last answered a whole visit, RFC 3339. */
  read_at?: string;
  /** When the forge first failed to answer, since the last whole visit, RFC 3339. */
  quiet_since?: string;
};

/** The tone of a pull request's chip: `bad` for one whose ci failed, `ask` for one the person can merge, `calm` for the rest. A state, never a colour. */
export type PullTone = "calm" | "ask" | "bad";

/** One pull request on the line: the hub's row whole, and two facts Fleet works out beside it. */
export type PullView = HubPullRequest & {
  /**
   * The Job that holds it: the one whose ledger names the pull request by its number, else its branch, in any state,
   * an ended Job included. Its `title` is always set: the ledger's, else the Job's id. Absent where no Job holds it.
   */
  held_by?: HubSession;
  /** How its chip reads, decided by Fleet. Another author's is always `calm`. */
  tone: PullTone;
};

/** Main as the line marks it. `state` is `green` or `red`. */
export type MainView =
  | {
      state: "green";
      /** Newer commits whose ci still runs, newest first. */
      checking?: MainChecking[];
    }
  | {
      state: "red";
      /** The first job that failed, in the forge's order. */
      failed: MainRedJob;
      /** The others, in order. */
      also?: MainRedJob[];
      /** When main first read red, RFC 3339. */
      since?: string;
      /** The pull request that turned it red. Its Job's `title` is always set: the ledger's, else the branch, else the Job's id. */
      merge?: MainMerge;
      /** The Job working on it, with its `title` always set as `merge`'s is. */
      taken_by?: HubSession;
      /** Another machine's claim on it. */
      elsewhere?: FixingElsewhere;
      /** Newer commits whose ci still runs. Non-empty is a held red: a newer run may have fixed it already, so nothing is handed a Job. */
      checking?: MainChecking[];
    };

/** One ci job that failed on main. */
export type MainRedJob = {
  /** The Manifest Check it maps to, else the job's own name. */
  check: string;
  /** The job maps to no Manifest Check. Absent is mapped. */
  unmapped?: boolean;
  /** The first test its log names. Absent where none could be read. */
  test?: string;
  log_url?: string;
};
