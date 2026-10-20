// The Cockpit's flight controls a person arranged in Kit, and a shell action run from one.
// `crates/ipc/src/flight_controls.rs`, `docs/practices/protocol.md`, *Flight controls*. Since 18 Oct 2026.
//
// The header rules in `protocol.ts` hold here. The file `~/.armada/flight-controls.toml` is
// `FlightControls` and a save sends the same type whole. Fleet does not know the built-in ids, so
// what a built-in tile may carry (a `kind` only on an own action, `on_text` and `off_text` on the
// land tile) is Bridge's to check before it saves.

import type { RunPlace } from "./run-memory";

/** What a person's own action does when pressed. Absent on a built-in tile. */
export type FlightControlKind = "words" | "command" | "shell" | "page";

/** One tile. Every field but `id` is the person's change to what Bridge draws by default. */
export type FlightControl = {
  /** A built-in id, or the person's own. */
  id: string;
  name?: string;
  /** A lucide name. */
  glyph?: string;
  /** One of `1` to `9`, `0`, or a letter. Once per list, a letter in either case. */
  key?: string;
  /** Absent is shown. */
  hidden?: boolean;
  /** The words or command a built-in sends, or an own action's words, slash command, shell line or address. */
  text?: string;
  /** The land tile only. */
  on_text?: string;
  /** The land tile only. */
  off_text?: string;
  /** A person's own action only. */
  kind?: FlightControlKind;
};

/**
 * The whole file, and the body of `save_flight_controls` (`POST /kit/flight_controls`). `version` is 1.
 * `job` is the tiles while a Job is open and `deck` those with none, each in order; a built-in a list
 * does not name is appended by Bridge in Armada's order. A save that breaks a rule is a 422
 * `fleet.flight_controls_refused` naming the field (`job[2].key`); a write that fails is a 500
 * `fleet.flight_controls_unwritable`.
 */
export type FlightControls = {
  version: number;
  job: FlightControl[];
  deck: FlightControl[];
};

/**
 * `get_flight_controls` (`GET /kit/flight_controls`), and `save_flight_controls`' answer once Fleet has
 * read the file again. An absent file is `read` with both lists empty. A file that will not read is
 * `not_read` with why, and is never drawn as the defaults.
 */
export type FlightControlsRead =
  | { what: "read"; controls: FlightControls }
  | { what: "not_read"; why: string };

/**
 * `get_repository_flight_controls` (`GET /repositories/flight_controls?repository=&session_id=`), and
 * `save_repository_flight_controls`' answer once Fleet has read the file again. `place` is where it was read: the
 * place chosen for the repository's remembered runs, except that `cloud` with Armada's store, and `repository`, keep
 * the file on this Mac and say `this_mac`. Every tile in `read.controls` carries a `kind`; a repository
 * holds a person's own actions only. A 404 `fleet.run_memory_repository_not_served` for a repository Fleet does not
 * serve.
 */
export type RepositoryFlightControls = {
  /** A root `list_repositories` names. */
  repository: string;
  place: RunPlace;
  read: FlightControlsRead;
  /**
   * The repository's own `.armada/flight-controls.toml`, as committed: read from the tree of the Job `session_id`
   * names, or from the checkout with none. Written only by `commit_to_repository`.
   */
  in_repository: FlightControlsRead;
  /** Each shell tile of `in_repository` and whether this Mac has approved its current command. Left out where none. */
  asking?: TileAsk[];
};

/** A shell tile the repository committed, and whether its current command may run on this Mac. */
export type TileAsk = {
  id: string;
  /** What `approve_repository_tile` names, so an approval cannot cover a later command. */
  command_hash: string;
  /** The command the ask shows. */
  command: string;
  approved: boolean;
};

/** A press that came from a tile the repository committed. Absent for a person's own tile. */
export type RepositoryTile = {
  /** A root `list_repositories` names. */
  repository: string;
  /** The tile's `id` in the repository's `.armada/flight-controls.toml`. */
  id: string;
};

/**
 * `approve_repository_tile`'s body (`POST /repositories/flight_controls/approve`), answered 200 with
 * `RepositoryFlightControls` read again. A 422 `fleet.tile_command_changed` where `command_hash` is not the tile's
 * current command, `fleet.tile_not_in_repository` where the repository holds no shell tile of that id.
 */
export type ApproveRepositoryTile = {
  repository: string;
  /** An open Job in the repository whose tree holds the tile. */
  session_id?: string;
  tile_id: string;
  /** The `TileAsk.command_hash` the person was shown. */
  command_hash: string;
};

/**
 * `save_repository_flight_controls`' body (`POST /repositories/flight_controls`): the repository's whole file. A
 * tile with no `kind` is a 422 `fleet.flight_controls_refused` naming the field (`job[0].kind`). A place that cannot
 * take it now is a 422 `fleet.flight_controls_unplaced`, and a bucket that cannot be used one of the
 * `fleet.run_cloud_*` codes `set_run_memory_place` answers.
 */
export type SaveRepositoryFlightControls = {
  repository: string;
  controls: FlightControls;
};

/**
 * `run_session_shell`: the person pressed a shell action. `POST /sessions/shell`. Answered 200 with
 * `SessionShellRan` once the command has ended. A 422 `fleet.session_shell_refused` for a Session that
 * holds no worktree slot, has ended, or is unknown, and for a blank command. With `repository_tile` set and its
 * command not approved on this Mac, a 422 `fleet.tile_not_approved` whose `fields` carry `repository`, `tile_id`,
 * `command` and `command_hash`.
 */
export type RunSessionShell = { session_id: string; command: string; repository_tile?: RepositoryTile };

/** `run_session_shell`, answered 200. */
export type SessionShellRan = {
  /** Absent where a signal ended it, which a timeout does. */
  exit?: number;
  /** The command ran past 120 seconds and was stopped. */
  timed_out: boolean;
  /** Standard output and error as they arrived: the last 65536 bytes. */
  output: string;
  /** The start of the output was dropped to fit. */
  truncated: boolean;
};
