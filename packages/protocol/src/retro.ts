// A Job's retro and the Lessons listing, mirrored by hand from
// `crates/ipc/src/retro.rs`. Since 23.12. `docs/concepts/retro.md`.

import type { Outcome } from "./reads";

/** Whom one retro item got in the way of. */
export type Whose = "drone" | "owner" | "fleet";

/**
 * Where the fix for one retro item lands: Armada itself, the Kit a person
 * brings, or the repository the Job worked on. Since 23.15.
 */
export type LandsIn = "armada" | "kit" | "manifest";

/** Where a Job's retro stands. */
export type RetroState = "pending" | "written" | "failed" | "skipped";

/**
 * Where a retro item stands with the person. Every item starts `open`.
 * `agreed` has a Job proposed for it, `accepted` is a Kit item kept as it is,
 * and `discarded` was disagreed with and stays in the store. Since 23.23.
 */
export type LessonState = "open" | "agreed" | "accepted" | "discarded";

/**
 * One thing that got in the way. `evidence` names rows of the record by `cite`.
 * `lands_in` is on every item written since 23.15, and absent on one kept
 * before, which is shown under All only.
 *
 * Since 23.23 an item has an `id`, and `title` (about eight words), `what` (one
 * or two short sentences) and `fix` (one sentence). All three are absent on an
 * item kept before, which has `statement` alone. On an item written since,
 * `statement` repeats `what`.
 *
 * Since 23.24 an item carries where it stands with the person, as a `Lesson`
 * does: `state`, and `job_proposed`, the Job `agree_lesson` proposed for it, a
 * Job id. `state` is absent only on an item whose row has no answer record.
 */
export type RetroItem = {
  id: string;
  who: Whose;
  title?: string;
  what?: string;
  fix?: string;
  statement: string;
  evidence: string[];
  lands_in?: LandsIn;
  state?: LessonState;
  job_proposed?: string;
};

/** A tool call the Drone was refused, with what it tried. */
export type RecordRefusal = {
  cite: string;
  at: string;
  step?: string;
  tool: string;
  tried?: string;
  because?: string;
};

/**
 * A file a failed Check names, set against what the Drone did. A fact read off
 * the transcript and never a verdict: `false` says no tool call of the Drone's
 * names the file, and `true` says one does, which may be a read. Since 23.23.
 */
export type RecordPath = {
  path: string;
  named_in_drone_calls: boolean;
};

/** A Check that did not pass: the gate's run, or the Drone's own. */
export type RecordCheck = {
  cite: string;
  at?: string;
  step?: string;
  attempt?: number;
  name: string;
  run: "gate" | "drone";
  expected?: string;
  produced?: string;
  /** Each file a gate failure names. Absent where it names none. Since 23.23. */
  paths?: RecordPath[];
};

/** A Judge criterion that was not met. */
export type RecordNotMet = {
  cite: string;
  step: string;
  attempt: number;
  criterion: string;
  expected?: string;
  produced?: string;
};

/** Something a Drone said, under the step it said it on. */
export type RecordSaid = {
  cite: string;
  at?: string;
  step?: string;
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
  step?: string;
  about: string;
  answered_at?: string;
  answer?: string;
  waited_ms?: number;
};

/** A stretch at an `awaiting_*` status. */
export type RecordWaited = {
  cite: string;
  status: string;
  from: string;
  until?: string;
  ms?: number;
};

/** Everything on a Job's record a retro is read from. An empty list is left out. */
export type RetroRecord = {
  refusals?: RecordRefusal[];
  failed_checks?: RecordCheck[];
  not_met?: RecordNotMet[];
  not_done?: RecordSaid[];
  said_after?: RecordSaid[];
  restarts?: RecordAct[];
  asked?: RecordAsked[];
  waited?: RecordWaited[];
  acts?: RecordAct[];
  notes?: RecordSaid[];
};

/** A note the owner left in Bridge while the Job's detail was open. */
export type LinkedAnnotation = {
  id: string;
  at: string;
  text: string;
  screen?: string;
  selector?: string;
};

/** `get_job_retro`: `GET /jobs/:job_id/retro`. */
export type JobRetro = {
  job_id: string;
  state: RetroState;
  at?: string;
  model?: string;
  why?: string;
  items?: RetroItem[];
  record: RetroRecord;
  annotations?: LinkedAnnotation[];
};

/**
 * One retro item, with the Job it came from. `id`, `title`, `what`, `fix` and
 * `lands_in` are as on `RetroItem`. `state` is every item's, `open` at the
 * start, and `job_proposed` is the Job `agree_lesson` proposed for it, a Job id.
 * Since 23.23.
 */
export type Lesson = {
  id: string;
  job_id: string;
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
  job_proposed?: string;
};

/**
 * `list_lessons`: `GET /lessons?manifest_id=&lands_in=&state=&most=`, newest
 * retro first. `lands_in` absent is all three, an item kept before 23.15
 * included. `state` absent is `open`; a person's saved Kit items are
 * `?state=accepted`.
 *
 * The two acts answer one `Lesson`, as it now stands: `POST
 * /lessons/:lesson_id/agree` and `POST /lessons/:lesson_id/disagree`, no body.
 * Agreeing an item whose fix lands in `armada` or `manifest` proposes a Job at
 * the approval gate and sets `job_proposed`, and one landing in `kit` becomes
 * `accepted`. An item that is not `open` answers with the state it stands in.
 */
export type Lessons = {
  lessons: Lesson[];
};

/**
 * What one Job's retro came back as. `BriefRead`'s shape: answered to the
 * caller rather than published, because nothing on `/events` says a retro
 * moved — a surface reads it when it opens and again when the window regains
 * focus.
 */
export type RetroRead = { ok: true; retro: JobRetro } | { ok: false; outcome: Outcome };

/** What the Lessons listing came back as. `RetroRead`'s shape and reasons. */
export type LessonsRead = { ok: true; lessons: Lesson[] } | { ok: false; outcome: Outcome };

/**
 * What `POST /lessons/:id/agree` and `POST /lessons/:id/disagree` came back as:
 * the item as it now stands, or the refusal. Agreeing an Armada or Manifest
 * item proposes a Job at the approval gate (`agreed`, `job_proposed` set);
 * agreeing a Kit item saves it (`accepted`); disagreeing discards it.
 */
export type LessonAnswer = { ok: true; lesson: Lesson } | { ok: false; outcome: Outcome };
