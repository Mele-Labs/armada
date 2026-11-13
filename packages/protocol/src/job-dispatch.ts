// `dispatch_job`: the person's work for a repository, handed to Fleet before any Job exists.
// `crates/ipc/src/job_dispatch.rs`.

import type { SentSketch, SessionTag, SessionUpload, StartSession } from "./hosted-sessions";
import type { SessionNote } from "./session-notes";
import type { SessionRecord } from "./sessions";

/**
 * `POST /sessions/dispatch`: a `StartSession`, flat, and the sends Bridge makes into the Job once it exists, in order.
 * Answered with `JobDispatched`. A body with `held`, `fork`, `root` or `after`, or no `request`, is `POST /sessions/start`
 * with its `then` sent.
 */
export type DispatchJob = StartSession & {
  /** What he marks it as: a hint the Operator is told, which picks no way and no workflow. Left out, the Operator reads it from the words and pictures. */
  kind?: DispatchKind;
  then?: DispatchThen[];
};

/** What the person marks the work he dispatches as: a hint to the Operator and nothing more. */
export type DispatchKind = "idea" | "feature" | "bug";

/** One send into the Job a dispatch became: the notes of a walk (`send_session_notes`), or a message (`send_session_message`). */
export type DispatchThen =
  | {
      notes: SessionNote[];
    }
  | {
      message: DispatchMessage;
    };

/** A `SendSessionMessage` whose Job is not known yet. */
export type DispatchMessage = {
  text: string;
  attachments?: SessionUpload[];
  mentions?: SessionTag[];
  sketches?: SentSketch[];
};

/** What `dispatch_job` answers. Exactly one is present: the Job started, or the dispatch's id while the Operator holds it. `parked` is no longer answered: an idea is kept only by the Operator's ruling, which `operator.dispatched` names. */
export type JobDispatched = {
  session?: SessionRecord;
  /** The dispatch's id: `operator.dispatched` says what became of it. */
  operator?: string;
  /** The idea's id, where it was kept for later. */
  parked?: string;
};

/** One idea kept for later: the dispatch whole, so starting it is `dispatch_job` with `dispatch`. */
export type ParkedIdea = {
  id: string;
  dispatch: DispatchJob;
  at: string;
};

/** `list_parked_ideas`' answer, and `remove_parked_idea`'s: oldest first. */
export type ParkedIdeas = {
  ideas: ParkedIdea[];
};

/** `remove_parked_idea`: one press. */
export type RemoveParkedIdea = {
  id: string;
};
