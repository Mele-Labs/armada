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
  then?: DispatchThen[];
};

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

/** What `dispatch_job` answers. Exactly one is present: the Job started, or the dispatch's id while the Operator holds it. */
export type JobDispatched = {
  session?: SessionRecord;
  /** The dispatch's id: `operator.dispatched` says what became of it. */
  operator?: string;
};
