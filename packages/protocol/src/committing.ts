// `commit_to_repository`, field for field with `crates/ipc/src/committing.rs`. `docs/concepts/kit.md`, *Commit to
// repository*.

/** Which of the person's own pieces is copied, by the id its concept already uses. */
export type PieceToCommit =
  /** Tiles of the person's own for this repository, by `id` in each list, into `.armada/flight-controls.toml`. */
  | { kind: "flight_controls"; job?: string[]; deck?: string[] }
  /** A workflow from Kit's `~/.armada/workflows/`, into `.armada/workflows/`. */
  | { kind: "workflow"; workflow_id: string }
  /** A review tool from Kit's `~/.armada/review-tools/`, by its dashed id, into `.armada/review-tools/`. */
  | { kind: "review_tool"; id: string }
  /** The person's own `armada.yml` for the repository, from where their Kit keeps it, to the workspace root. */
  | { kind: "manifest" }
  /**
   * Requests' collections of the person's own for this repository by `id`, and environments by `name`, into
   * `.armada/requests.toml`. A secret goes as its name and `secret = true`, never its value.
   */
  | { kind: "requests"; collections?: string[]; environments?: string[] };

/**
 * `commit_to_repository`'s body (`POST /repositories/commit`): start a Job of its own that copies one piece onto its
 * own branch in one commit and opens a pull request for that alone. No Job need be open. The person's own copy
 * stays. A piece the person does not keep, a 422 `fleet.commit_not_yours`; an id main already holds (for `manifest`,
 * an `armada.yml`) without `overwrite`, or the piece as written, `fleet.commit_exists`; a file on main that will not
 * read, tiles that break a rule, a Kit `armada.yml` that is not a whole Manifest or no slot for the Job,
 * `fleet.commit_refused`. A 500 `fleet.commit_unwritable` where a file cannot be written or git refuses the commit.
 */
export type CommitToRepository = {
  /** A root `list_repositories` names. */
  repository: string;
  piece: PieceToCommit;
  /** Replace what main already holds under the same id. False where absent. */
  overwrite?: boolean;
};

/** `commit_to_repository`, answered 200. */
export type CommittedToRepository = {
  repository: string;
  /** The Job made for this commit alone. */
  session_id: string;
  /** The Job's own branch, which holds the one commit. */
  branch: string;
  /** Each file written, relative to the repository's root. */
  paths: string[];
  commit: string;
  /** The pull request's address, where one was opened. */
  pull_request?: string;
  /** Why no pull request was opened, where none was. The commit stays on the Job's branch. */
  pull_request_not_opened?: string;
};
