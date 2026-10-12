// Starting a Session on a retro item, mirrored by hand from
// `crates/ipc/src/retro_session.rs`. Since 23.76.
// `docs/concepts/retro.md`, *Agree and disagree*.

import type { Lesson } from "./retro";

/**
 * What `start_lesson_session` (`POST /lessons/:lesson_id/session`) answers. The
 * Session is running: Fleet sent `brief` as its first message at once. `lesson`
 * is `agreed` with `session_proposed` set to `session_id`.
 */
export type LessonSessionStarted = {
  lesson: Lesson;
  session_id: string;
  /** The first message, as it was sent. */
  brief: string;
};
