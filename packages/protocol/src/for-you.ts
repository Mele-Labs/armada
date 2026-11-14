// One Job's For you tab on the wire, mirrored by hand from `crates/ipc/src/for_you.rs`.

import type { SessionStanding } from "./sessions";

/** What a For you row is, for the glyph and the press. */
export type ForYouKind = "walk" | "document" | "plan" | "draft" | "sketch" | "question" | "run" | "approve";

/** Who put the row on the list. */
export type ForYouBy = "fleet" | "agent";

/** What a press on the row opens. */
export type ForYouOpens =
  | { kind: "artifact"; id: string }
  | { kind: "walk"; url: string }
  | { kind: "waiting"; id: string }
  | { kind: "plan" };

/** One thing the Job waits on him for. */
export type ForYouItem = {
  /** The same across reads: a Fleet item's own id, `waiting:<id>` for an agent's, `added:<n>` for one `add_for_you` added. */
  id: string;
  kind: ForYouKind;
  title: string;
  /** Why it is there, or what to decide. Absent where none was set. */
  line?: string;
  /** `order_for_you` placed it. */
  placed: boolean;
  /** When it began to wait. */
  since: string;
  added_by: ForYouBy;
  opens: ForYouOpens;
  /** Settled in place with Approve or Ask for changes. */
  approves: boolean;
};

/** One live Job this Job started. */
export type ForYouBuild = {
  job: string;
  title: string;
  /** What it is doing. Absent for a Job Fleet has read nothing of. */
  standing?: SessionStanding;
  /** The step of its workflow it is on. Absent for a Job on no workflow. */
  line?: string;
};

/** One of the Job's own pull requests still open. */
export type ForYouLanding = {
  pr: number;
  /** Empty where the forge has not been read for it yet. */
  title: string;
};

/** `GET /sessions/for_you?session_id=`: one Job's own items, never another Job's. */
export type ForYou = {
  job: string;
  /** Placed first in the order set, then the rest oldest first. */
  waiting: ForYouItem[];
  building: ForYouBuild[];
  landing: ForYouLanding[];
};

/** `POST /sessions/for_you/withdraw`. */
export type WithdrawForYou = { session_id?: string; id: string };

/** `POST /sessions/for_you/order`: replaces the order set before. */
export type OrderForYou = { session_id?: string; ids: string[] };

/** `POST /sessions/for_you/line`: a blank line clears it. */
export type LineForYou = { session_id?: string; id: string; line: string };

/** `POST /sessions/for_you/add`. */
export type AddForYou = { session_id?: string; title: string; kind: ForYouKind; opens: ForYouOpens; line?: string };

/** What `add_for_you` answers. */
export type ForYouAdded = { id: string };

/** `session.for_you`: the Job's For you moved. Read `get_for_you` again. */
export type ForYouChanged = { session_id: string };
