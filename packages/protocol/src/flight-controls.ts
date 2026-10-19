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
 * `get_repository_flight_controls` (`GET /repositories/flight_controls?repository=`), and
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
 * holds no worktree slot, has ended, or is unknown, and for a blank command.
 */
export type RunSessionShell = { session_id: string; command: string };

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
