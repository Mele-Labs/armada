// LOCAL STAND-INS for the guided review's wire shapes, `POST /lessons/review`
// and `POST /lessons/:lesson_id/ask`.
//
// **Delete this file when `@armada/protocol` exports `LessonReview`,
// `ReviewEntry`, `SetAside`, `LessonAsk`, `AskTurn` and `AskLessonAnswer`**, and import
// them from there: the Rust side owns the shapes, and these copy what it was
// told to serve. Everything below is named the way the protocol will name it.

import type { Outcome } from "@armada/protocol";

/** One item the model put in the queue, best first. */
export type ReviewEntry = {
  lesson_id: string;
  /** The ids of duplicates this one stands for. They are not in the queue themselves. */
  merged_ids: string[];
  /** One line on why this one is worth a person's time. */
  reason: string;
};

/** One item the model left out of the queue, and why. */
export type SetAside = {
  lesson_id: string;
  why: string;
};

/** The model's reading of the open items, ordered best first. */
export type LessonReview = {
  model: string;
  entries: ReviewEntry[];
  set_aside: SetAside[];
};

/** Who said one turn of a question thread. */
export type AskRole = "person" | "fleet";

/** One earlier turn of the thread about an item. */
export type AskTurn = {
  role: AskRole;
  text: string;
};

/** The body of `POST /lessons/:lesson_id/ask`: the question and the turns before it, oldest first. */
export type LessonAsk = {
  question: string;
  history: AskTurn[];
};

/** What `POST /lessons/:lesson_id/ask` answers. */
export type AskLessonAnswer = { answer: string };

/** `LessonsRead`'s shape for the review. */
export type LessonReviewRead = { ok: true; review: LessonReview } | { ok: false; outcome: Outcome };

/** `LessonsRead`'s shape for an answer to a question. */
export type AskLessonRead = { ok: true; answer: AskLessonAnswer } | { ok: false; outcome: Outcome };
