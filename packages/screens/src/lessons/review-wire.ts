// The guided review's wire shapes, from `@armada/protocol` (`docs/concepts/retro.md`,
// *Reviewing*), with the two read results Bridge's main process answers with.

import type { AskLessonAnswer, LessonReview, Outcome } from "@armada/protocol";

export type {
  AskLessonAnswer,
  AskRole,
  AskTurn,
  LessonAsk,
  LessonReview,
  ReviewEntry,
  SetAside,
} from "@armada/protocol";

/** `LessonsRead`'s shape for the review. */
export type LessonReviewRead = { ok: true; review: LessonReview } | { ok: false; outcome: Outcome };

/** `LessonsRead`'s shape for an answer to a question. */
export type AskLessonRead = { ok: true; answer: AskLessonAnswer } | { ok: false; outcome: Outcome };
