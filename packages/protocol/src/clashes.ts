// Where two agents' changes in one repository would conflict. `crates/ipc/src/clashes.rs`,
// `docs/concepts/fleet.md`, *Write clashes*. Since protocol 23.80.
//
// The header rules in `protocol.ts` hold here.

/** What a clash is between. `main` is the repository's base branch as it stands now. */
export type ClashSide = "job" | "session" | "main";

/** One side of a clash. */
export type ClashHolder = {
  kind: ClashSide;
  /** The Job's or Session's id; the base branch's name for `main`. */
  id: string;
  /** A Job's handle, a Session's title, the base branch's name. */
  label: string;
};

/** Two holders whose changes conflict. */
export type WriteClash = {
  one: ClashHolder;
  other: ClashHolder;
  /** Repository-relative, sorted. */
  paths: string[];
  /** When these two first conflicted. A change in which files keeps it. */
  since: string;
};

/** `list_write_clashes`: `GET /clashes?manifest_id=&holder=`. `holder` is a Job's or a Session's id. */
export type WriteClashes = { clashes: WriteClash[] };
