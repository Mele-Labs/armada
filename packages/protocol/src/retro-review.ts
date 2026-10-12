// The guided review of the open retro items and a question about one,
// mirrored by hand from `crates/ipc/src/retro_review.rs`. Since 23.74.
// `docs/concepts/retro.md`, *Reviewing*.

/** The body of `review_lessons` (`POST /lessons/review`). It may be left out. */
export type ReviewLessons = {
  /** Narrows the review to one repository's Jobs. Absent is every repository served. */
  manifest_id?: string;
};

/**
 * One item worth the owner's time, with the duplicates it stands for.
 * `reason` reads `not ranked` on an item the model left out of its answer.
 */
export type ReviewEntry = {
  /** `Lesson.id` of the best representative. */
  lesson_id: string;
  /** `Lesson.id` of open items that say the same thing. */
  merged_ids: string[];
  reason: string;
};

/** An item the review leaves out of the queue, and why. */
export type SetAside = {
  lesson_id: string;
  why: string;
};

/**
 * What `review_lessons` answers. `entries` are best first and carry no rank.
 * With no open item the lists are empty and `model` is empty.
 */
export type LessonReview = {
  model: string;
  entries: ReviewEntry[];
  set_aside: SetAside[];
};

/** Who said one turn of a thread about an item. */
export type AskRole = "person" | "fleet";

/** One earlier turn of the thread about an item. */
export type AskTurn = {
  role: AskRole;
  text: string;
};

/**
 * The body of `ask_lesson` (`POST /lessons/:lesson_id/ask`). Fleet keeps no
 * thread, so the turns before the question travel with it, oldest first. The
 * question is at most 2000 characters, the history 20 turns of at most 4000.
 */
export type LessonAsk = {
  question: string;
  history?: AskTurn[];
};

/** What `ask_lesson` answers. */
export type AskLessonAnswer = { answer: string };
