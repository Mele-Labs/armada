// Start Session's wire shapes (`docs/concepts/retro.md`, *Where Bridge draws it*).
//
// **LOCAL STAND-INS.** `start_lesson_session` (`POST /lessons/:lesson_id/session`, Protocol 23.76)
// is in `crates/ipc` and not yet in `@armada/protocol`. Each type below is the Rust one spelled
// out, so that when `LessonSessionStarted` and `Lesson.session_proposed` land the swap is these
// declarations replaced by a re-export from `@armada/protocol`, as `review-wire.ts` was.

import type { Lesson, Outcome } from "@armada/protocol";

/** A `Lesson` or retro item as 23.76 serves it: `session_proposed` beside `job_proposed`. */
export type WithSession<T> = T & {
  /** The Session `start_lesson_session` started for it, a Session id. At most one of this and `job_proposed`. */
  session_proposed?: string;
};

/** What `start_lesson_session` answers: the item as it stands, the Session, and the brief Fleet sent it. */
export type LessonSessionStarted = {
  lesson: WithSession<Lesson>;
  /** The Session started. It holds no process until a message is sent. */
  session_id: string;
  /** The text Fleet sent as the Session's first message. The Session starts work at once. */
  brief: string;
  /**
   * **A guess at the wire, to be checked against the Rust type.** Fleet's words where the Session
   * started and sending the brief failed; the Session is still opened, and the card says it.
   */
  send_failed?: string;
};

/** `LessonAnswer`'s shape for starting a Session. */
export type LessonSessionRead = { ok: true; started: LessonSessionStarted } | { ok: false; outcome: Outcome };

/** The Session an item or Lesson started, where it started one. */
export function sessionProposedOf(item: object): string | undefined {
  const one = (item as { session_proposed?: unknown }).session_proposed;
  return typeof one === "string" ? one : undefined;
}
