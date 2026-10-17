// A command a person always allowed for a repository, and taking it back.
// `crates/ipc/src/commanding.rs`.

/**
 * One command a person allowed — a row of `RepositoryAllowedCommands`.
 * Since protocol 11.0. `crates/ipc/src/commanding.rs`.
 */
export type AllowedCommandRow = {
  /** The command, whole, as the person allowed it. What `remove_repository_allowed_command` names. */
  run: string;
  /** When it was allowed, by fleet's clock. */
  allowed_at: string;
  /** Who allowed it, in the envelope's spelling. Left as `string` like `actor`. */
  by: string;
};

/**
 * `get_repository_allowed_commands`'s answer: every rule a person
 * always-allowed for this manifest's repository, oldest first. Since
 * protocol 13.5.
 */
export type RepositoryAllowedCommands = {
  commands: AllowedCommandRow[];
};

/**
 * The body of `remove_repository_allowed_command`. Since protocol 13.5.
 * Answered with `RepositoryAllowedCommands`.
 *
 * Not scoped to anything but the repository: it names a rule kept for the
 * manifest. A rule this manifest holds no allow for is a 409.
 */
export type RemoveRepositoryAllowedCommand = {
  run: string;
};
