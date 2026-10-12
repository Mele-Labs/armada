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

/** What `start_lesson_session` answers: the item as it stands, the Session, and the unsent first message. */
export type LessonSessionStarted = {
  lesson: WithSession<Lesson>;
  /** The Session started. It holds no process until a message is sent. */
  session_id: string;
  /** The first message, for the person to read in the Session's message box and send. */
  draft: string;
};

/** `LessonAnswer`'s shape for starting a Session. */
export type LessonSessionRead = { ok: true; started: LessonSessionStarted } | { ok: false; outcome: Outcome };

/** The Session an item or Lesson started, where it started one. */
export function sessionProposedOf(item: object): string | undefined {
  const one = (item as { session_proposed?: unknown }).session_proposed;
  return typeof one === "string" ? one : undefined;
}
