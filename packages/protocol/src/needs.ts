// `armada need` on the wire: what a checkout says it needs on a path, and what
// Fleet answers from the session ledger. `crates/ipc/src/needs.rs`,
// `docs/capabilities/needs.md`. Since protocol 23.46.
//
// The header rules in `protocol.ts` hold here.

import type { Holder } from "./sessions";

/** Which form of `armada need` was asked. */
export type NeedAct = "declare" | "took" | "release";

/** What `armada need` sends. `POST /needs`. */
export type NeedCall = {
  act: NeedAct;
  /** The repository the checkout is of. Absent is the first Fleet serves. */
  manifest_id?: string;
  /** The branch the caller stands on: a need belongs to what holds the branch. */
  branch: string;
  /** Repository-relative. */
  path: string;
  /** What is needed there, in the caller's words. `declare` only. */
  what?: string;
  /** What it took. `took` only. */
  value?: string;
};

/** One need, as a person reads it. */
export type NeedLine = {
  holder: Holder;
  /** Who holds it: the branch, or the session's title where there is none. */
  held_by: string;
  path: string;
  what: string;
  /** Absent until the holder says what it took. */
  took?: string;
  since: string;
};

/** What an act came to. */
export type NeedAnswer = {
  /** The holder's need after a `declare` or a `took`. */
  mine?: NeedLine;
  /** A `declare` of a need the holder already had: nothing was recorded. */
  already: boolean;
  /** The standing needs on the same path that were declared first, in order. */
  ahead: NeedLine[];
  /** A `release` that gave one back. */
  gave_back: boolean;
};

/** `list_needs`: every standing need of one repository, by path and in order. */
export type NeedList = { needs: NeedLine[] };
