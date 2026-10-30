// `commit_to_repository`, field for field with `crates/ipc/src/committing.rs`. `docs/concepts/kit.md`, *Commit to
// repository*.

import type { PieceScope } from "./lifecycles";

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
  | { kind: "requests"; collections?: string[]; environments?: string[] }
  /** A lifecycle from Kit's `~/.armada/lifecycles/`, found by the id and scope its file declares, into `.armada/lifecycles/`. `scope` is everywhere where absent. */
  | { kind: "lifecycle"; id: string; scope?: PieceScope }
  /** A lifecycle rule from Kit's `~/.armada/rules/`, into `.armada/rules/`. */
  | { kind: "lifecycle_rule"; id: string; scope?: PieceScope }
  /** A Workspace from Kit's `~/.armada/workspaces/`, into `.armada/workspaces/`. Its id is its identity, so it has no scope. */
  | { kind: "workspace"; id: string }
  /** A layer of autonomy defaults from Kit's `~/.armada/autonomy/`, into `.armada/autonomy/`. */
  | { kind: "autonomy"; id: string; scope?: PieceScope };

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
  /**
   * Take the piece out of the repository instead of copying it in: the file on the base branch that declares it is
   * deleted by the same kind of Job, with a pull request. Only the `lifecycle`, `lifecycle_rule`, `workspace` and
   * `autonomy` kinds can be removed. **Leave it out unless true**: a Fleet older than the field refuses a body that
   * carries it.
   */
  remove?: boolean;
};

/** `commit_to_repository`, answered 200 once the Job and its branch exist. The commit and the pull request follow; a failure arrives as `act.failed` for that Job. */
export type CommittedToRepository = {
  repository: string;
  /** The Job made for this commit alone. */
  session_id: string;
  /** The Job's own branch, which will hold the one commit. */
  branch: string;
  /** Each file written, relative to the repository's root. */
  paths: string[];
};
