// Start Session's read result (`docs/concepts/retro.md`, *Where Bridge draws it*), over
// `LessonSessionStarted` from `@armada/protocol`.

import type { LessonSessionStarted, Outcome } from "@armada/protocol";

/** `LessonAnswer`'s shape for starting a Session. */
export type LessonSessionRead = { ok: true; started: LessonSessionStarted } | { ok: false; outcome: Outcome };

/** The Session an item or Lesson started, where it started one. */
export function sessionProposedOf(item: { session_proposed?: string }): string | undefined {
  return item.session_proposed;
}
