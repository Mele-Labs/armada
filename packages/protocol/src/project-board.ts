// A project's dashboard board, kept by Fleet with the project's look, and the issues and pull requests ranked for it.
// Mirrors `crates/ipc/src/project_board.rs`.

import type { TrackedIssue } from "./issues";

/** One block of the board, in the grid's columns and rows. `kind` and `view` are Bridge's words, kept as written whether or not they are known. */
export type BoardBlock = {
  /** `needs`, `run`, `reviews`, `issues`, `features`, `checks`, and any kind a later Bridge adds. */
  kind: string;
  x: number;
  y: number;
  w: number;
  h: number;
  /** `list`, `table`, `cards`, `graph` or `picture`. */
  view: string;
};

/** A project's board. */
export type BoardLayout = {
  blocks: BoardBlock[];
};

/** `GET` and `POST /projects/project/:project/board`: `layout` is absent for a project that saved none, and Bridge draws its preset. */
export type ProjectBoard = {
  project: string;
  layout?: BoardLayout;
};

/** `POST /projects/project/:project/board`: the layout to keep, whole. Absent forgets it. */
export type SaveProjectBoard = {
  layout?: BoardLayout;
};

/** How a feature came to be matched: an issue by `pull_request_files`, `label` or `title`; a pull request by `files`. */
export type MatchedBy = "pull_request_files" | "files" | "label" | "title";

/** A feature a ranked row matched, and why. */
export type MatchedFeature = {
  slug: string;
  title: string;
  by: MatchedBy[];
  /** The owner's Jobs that filed a file under this feature in the window. */
  jobs: number;
};

/** One open, unassigned issue and the features it matched. `features` is empty for an issue that matched none, which is ranked last. */
export type RankedIssue = {
  /** The repository it was listed from, absent for an issue of the project's own tracker. */
  manifest_id?: string;
  issue: TrackedIssue;
  features: MatchedFeature[];
  /** The owner's Jobs across `features`, each counted once. */
  jobs: number;
  /** The librarian's short reason for the row's place, where it ranked the list. */
  why?: string;
};

/** One open pull request that is not his, and the features its files fall in. */
export type RankedPull = {
  manifest_id: string;
  repository: string;
  number: number;
  title: string;
  author: string;
  draft: boolean;
  features: MatchedFeature[];
  jobs: number;
  /** As on `RankedIssue`. */
  why?: string;
};

/** `GET /projects/project/:project/ranked_issues`. */
export type RankedIssues = {
  project: string;
  /** The window the Jobs were counted in, in days. */
  days: number;
  /** The oldest read of the issue lists. */
  fetched_at?: string;
  might_pick_up: RankedIssue[];
  might_be_interested: RankedPull[];
  /** When the librarian ranked both lists, which are then in its order with a `why` on each row. */
  by_librarian?: string;
};
