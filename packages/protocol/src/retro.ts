// A Job's retro and the Lessons listing, mirrored by hand from
// `crates/ipc/src/retro.rs`. Since 23.12. `docs/concepts/retro.md`.

/** Whom one retro item got in the way of. */
export type Whose = "drone" | "owner" | "fleet";

/** Where a Job's retro stands. */
export type RetroState = "pending" | "written" | "failed" | "skipped";

/** One thing that got in the way. `evidence` names rows of the record by `cite`. */
export type RetroItem = {
  who: Whose;
  statement: string;
  evidence: string[];
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

/** One retro item, with the Job it came from. */
export type Lesson = {
  job_id: string;
  handle: string;
  at: string;
  who: Whose;
  statement: string;
  evidence: string[];
};

/** `list_lessons`: `GET /lessons?manifest_id=&most=`, newest retro first. */
export type Lessons = {
  lessons: Lesson[];
};
