// `commit_to_repository`, field for field with `crates/ipc/src/committing.rs`. `docs/concepts/kit.md`, *Commit to
// repository*.

/** Which of the person's own pieces is copied, by the id its concept already uses. */
export type PieceToCommit =
  /** Tiles of the person's own for this repository, by `id` in each list, into `.armada/flight-controls.toml`. */
  | { kind: "flight_controls"; job?: string[]; deck?: string[] }
  /** A workflow from Kit's `~/.armada/workflows/`, into `.armada/workflows/`. */
  | { kind: "workflow"; workflow_id: string }
  /** A review tool from Kit's `~/.armada/review-tools/`, by its dashed id, into `.armada/review-tools/`. */
  | { kind: "review_tool"; id: string };

/**
 * `commit_to_repository`'s body (`POST /repositories/commit`): copy one piece onto the branch of the Job `session_id`
 * names, and commit it there. The person's own copy stays. With no Job named that holds a branch in the repository,
 * a 422 `fleet.commit_no_job`; a piece the person does not keep, `fleet.commit_not_yours`; an id the branch already
 * holds without `overwrite`, `fleet.commit_exists`; a branch file that will not read or tiles that break a rule,
 * `fleet.commit_refused`. A 500 `fleet.commit_unwritable` where a file cannot be written or git refuses the commit.
 */
export type CommitToRepository = {
  /** A root `list_repositories` names. */
  repository: string;
  session_id?: string;
  piece: PieceToCommit;
  /** Replace what the Job's branch already holds under the same id. False where absent. */
  overwrite?: boolean;
};

/** `commit_to_repository`, answered 200. */
export type CommittedToRepository = {
  repository: string;
  /** The Job's branch the commit is on. */
  branch: string;
  /** Each file written, relative to the repository's root. */
  paths: string[];
  /** The commit made. Absent where the branch already held the files as written. */
  commit?: string;
};
