// A Session's retro and the Lessons listing, mirrored by hand from
// `crates/ipc/src/retro.rs`. `docs/concepts/retro.md`.

import type { Outcome } from "./reads";

/** Whom one retro item got in the way of. */
export type Whose = "drone" | "agent" | "owner" | "fleet";

/**
 * Where the fix for one retro item lands: Armada itself, the Kit a person
 * brings, or the repository the Session worked on. Since 23.15.
 */
export type LandsIn = "armada" | "kit" | "manifest";

/** Where a Session's retro stands. */
export type RetroState = "pending" | "written" | "failed" | "skipped";

/**
 * Where a retro item stands with the person. Every item starts `open`.
 * `agreed` has a Session started for it, `accepted` is a Kit item kept as it is,
 * and `discarded` was disagreed with and stays in the store. Since 23.26.
 */
export type LessonState = "open" | "agreed" | "accepted" | "discarded";

/**
 * One thing that got in the way. `evidence` names rows of the record by `cite`.
 * `lands_in` is on every item written since 23.15, and absent on one kept
 * before, which is shown under All only.
 *
 * Since 23.26 an item has an `id`, and `title` (about eight words), `what` (one
 * or two short sentences) and `fix` (one sentence). All three are absent on an
 * item kept before, which has `statement` alone. On an item written since,
 * `statement` repeats `what`.
 *
 * Since 23.27 an item carries where it stands with the person, as a `Lesson`
 * does: `state`, which is absent only on an item whose row has no answer record.
 */
export type RetroItem = {
  id: string;
  /** Where the item stands, as on `Lesson`. Absent on an item kept before it was written. */
  state?: LessonState;
  /** The Session `start_lesson_session` started for it. Since 23.77. */
  session_proposed?: string;
  who: Whose;
  title?: string;
  what?: string;
  fix?: string;
  statement: string;
  evidence: string[];
  lands_in?: LandsIn;
  /**
   * What Accept would change in Kit. Only on a `kit` item whose command Fleet
   * copied off a refusal in the record; absent on every other item and on one
   * kept before 23.35. Since 23.35.
   */
  change?: RetroChange;
  /** What Accept applied, on every later read of the item. Absent where nothing was applied. Since 23.35. */
  applied?: RetroChange;
};

/**
 * A change to Kit a retro item carries, which `agree_lesson` applies. Since
 * 23.35. Tagged on `kind`; `allow_command` is the only kind.
 */
export type RetroChange = { kind: "allow_command"; command: string };

/** A tool call the agent was refused, or one that errored, with what it tried. */
export type RecordRefusal = {
  cite: string;
  at: string;
  tool: string;
  tried?: string;
  because?: string;
};

/** Something an agent or a person said. */
export type RecordSaid = {
  cite: string;
  at?: string;
  said: string;
};

/** A move a person or an agent made, with the door it came through. */
export type RecordAct = {
  cite: string;
  at: string;
  actor: string;
  via?: string;
  moved: string;
  said?: string;
};

/** A question put to a person, and how long it waited. */
export type RecordAsked = {
  cite: string;
  asked_at: string;
  about: string;
  answered_at?: string;
  answer?: string;
  waited_ms?: number;
};

/**
 * Everything on a Session's record a retro is read from. An empty list is left out.
 * Its gate denials are `refusals` and its resumes and reattaches are `restarts`.
 */
export type RetroRecord = {
  refusals?: RecordRefusal[];
  restarts?: RecordAct[];
  notes?: RecordSaid[];
  /** The agent's asks. */
  asks?: RecordAsked[];
  /** The owner's messages that redirect or reject. */
  corrections?: RecordSaid[];
  /** Tool results that errored: `tried` is the call, `because` the error. */
  failed_tools?: RecordRefusal[];
  /** Each subagent's own trouble. */
  subagents?: RecordSaid[];
};

/** The Session a retro or an item was written for. */
export type RetroSession = { id: string; title?: string };

/** Whose retro a read asks for: a Session's, on `/sessions/:id/retro?n=`, where `n` names an older retro and absent is the newest. */
export type RetroSubject = { kind: "session"; id: string; n?: number };

/** `get_session_retro`: `GET /sessions/:session_id/retro?n=`. */
export type Retro = {
  session: RetroSession;
  state: RetroState;
  at?: string;
  model?: string;
  why?: string;
  items?: RetroItem[];
  record: RetroRecord;
};

/**
 * One retro item, with the Session it came from. `id`, `title`, `what`, `fix` and
 * `lands_in` are as on `RetroItem`. `state` is every item's, `open` at the
 * start. Since 23.26.
 */
export type Lesson = {
  id: string;
  handle: string;
  at: string;
  who: Whose;
  title?: string;
  what?: string;
  fix?: string;
  statement: string;
  evidence: string[];
  lands_in?: LandsIn;
  state: LessonState;
  /** As on `RetroItem`. Since 23.77. */
  session_proposed?: string;
  session: RetroSession;
  /** As on `RetroItem`. Since 23.35. */
  change?: RetroChange;
  /** As on `RetroItem`. Since 23.35. */
  applied?: RetroChange;
};

/**
 * `list_lessons`: `GET /lessons?manifest_id=&lands_in=&state=&most=`, newest
 * retro first. `lands_in` absent is all three, an item kept before 23.15
 * included. `state` absent is `open`; a person's saved Kit items are
 * `?state=accepted`.
 *
 * The two acts answer one `Lesson`, as it now stands: `POST
 * /lessons/:lesson_id/agree` and `POST /lessons/:lesson_id/disagree`, no body.
 * Agreeing an item whose fix lands in `armada` or `manifest` starts a Session
 * and sets `session_proposed`, and one landing in `kit` becomes
 * `accepted`. An item that is not `open` answers with the state it stands in.
 */
export type Lessons = {
  lessons: Lesson[];
};

/**
 * What one Session's retro came back as. Answered to the caller rather than
 * published, because nothing on `/events` says a retro moved — a surface reads
 * it when it opens and again when the window regains focus.
 */
export type RetroRead = { ok: true; retro: Retro } | { ok: false; outcome: Outcome };

/** What the Lessons listing came back as. `RetroRead`'s shape and reasons. */
export type LessonsRead = { ok: true; lessons: Lesson[] } | { ok: false; outcome: Outcome };

/**
 * What `POST /lessons/:id/agree` and `POST /lessons/:id/disagree` came back as:
 * the item as it now stands, or the refusal. Agreeing an Armada or Manifest
 * item starts a Session (`agreed`, `session_proposed` set);
 * agreeing a Kit item saves it (`accepted`); disagreeing discards it.
 */
export type LessonAnswer = { ok: true; lesson: Lesson } | { ok: false; outcome: Outcome };
