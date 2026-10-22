// What each served repository holds and where each agent has changed it, for the flight deck's map.
// `crates/ipc/src/territory.rs`, `docs/concepts/fleet.md`, *The territory*.
//
// The header rules in `protocol.ts` hold here.

/** `list_repository_areas`: `GET /repository_areas`. A repository whose base git could not read is absent. */
export type RepositoryAreas = { repositories: AreasOfRepository[] };

/** One repository's directories, read at the head of its base. */
export type AreasOfRepository = {
  manifest_id: string;
  /** The repository's root, as `list_repositories` names it. */
  root: string;
  /** The commit of the base branch the counts were read at. */
  head: string;
  /** The largest directories, a top directory before any inside it. At most 40 counting each. */
  areas: RepositoryArea[];
  /** How many directories past the 40 were left out. */
  omitted: number;
};

/** A top directory, its files and the directories inside it that made the list. */
export type RepositoryArea = {
  /** One name: `crates`. */
  dir: string;
  /** Every file at or under it. */
  files: number;
  /** Absent where none inside it made the list. */
  kids?: AreaKid[];
  /**
   * Its first files, as git lists them, named relative to `dir`: `src/lib.rs`. Present only where it has no kids, since
   * each kid names its own. At most 100; the files past them are `files` less these.
   */
  names?: string[];
};

/** A directory inside a top directory, by its own name: `fleet` for `crates/fleet`. */
export type AreaKid = {
  name: string;
  files: number;
  /** Its first files, as git lists them, named relative to the kid. At most 100; `files` stays the exact count. */
  names?: string[];
};

/** `list_slot_touches`: `GET /slot_touches`. A holder with no slot, or no change in it, is absent. */
export type SlotTouches = { repositories: TouchesInRepository[] };

export type TouchesInRepository = { manifest_id: string; holders: HolderTouches[] };

/** What one hosted Session has changed in its slot since the slot met the base branch. */
export type HolderTouches = {
  kind: "session";
  /** The Session's id. */
  id: string;
  slot: number;
  /** Repository-relative and sorted: committed, edited, deleted or not tracked yet. At most 200. */
  paths: string[];
  /** How many paths past the 200 were left out. */
  omitted: number;
};

/**
 * `get_session_file_change`: `GET /sessions/file_change?session_id=&path=&seen=`. One file a Job has been seen writing, as
 * `git diff` prints it from where the Job's branch met the base to the worktree as it stands, uncommitted edits included.
 * Asked on a `timeline.appended` `session.edited` entry naming the file the lens shows, never on a timer.
 */
export type SessionFileChange = {
  /** Opaque. Sent back as `seen`, an equal stamp is answered `unchanged` without the diff. */
  stamp: string;
  /** Empty where `unchanged` is set, or where the file is as the base has it. */
  diff: string;
  /** Set where Fleet cut `diff` at 2,000 lines or 256 KiB. Whole lines of the start of the change. Absent otherwise. */
  cut?: boolean;
  /** Set where `seen` was this stamp: the diff already held is the current one. Absent otherwise. */
  unchanged?: boolean;
};
