// Retros and Lessons: one retro, the Lessons listing, and the owner's answer to an item.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { LessonAnswer, LessonsRead, RetroRead, RetroSubject } from "@armada/protocol";
import type { AskLessonRead, AskTurn, LessonReviewRead } from "@armada/screens/src/lessons/review-wire";
import type { LessonSessionRead } from "@armada/screens/src/lessons/session-wire";

export type LessonsApi = {
  /**
   * One retro — `docs/concepts/retro.md`: answered once to the
   * surface that asked, and asked again when the window regains focus, because nothing on
   * `/events` says a retro was written.
   */
  readRetro: (subject: RetroSubject) => Promise<RetroRead>;
  /**
   * Every written retro's items across Sessions, newest first, narrowed to this window's pick: the
   * open ones, or the saved Kit items under `accepted`.
   */
  readLessons: (state: "open" | "accepted") => Promise<LessonsRead>;
  /**
   * Agree with one retro item. An Armada or Manifest item starts a Session;
   * a Kit item is saved under Accepted. Answers the item as it now stands.
   */
  agreeLesson: (lessonId: string) => Promise<LessonAnswer>;
  /** Disagree with one retro item: it is discarded. */
  disagreeLesson: (lessonId: string) => Promise<LessonAnswer>;
  /**
   * Agree with an Armada or Manifest item by starting a Session on its repository. **Fleet sends
   * the brief as the Session's first message**, so the Session begins work at once. Answers the
   * item as it now stands, the Session and the brief sent.
   */
  startLessonSession: (lessonId: string) => Promise<LessonSessionRead>;
  /**
   * The model's reading of the open items for the guided review: the order to read them in, which
   * stand for duplicates, and which it set aside. **Narrowed to this window's pick in main**, as
   * `readLessons` is, so the renderer names no manifest.
   */
  reviewLessons: () => Promise<LessonReviewRead>;
  /** Put one question about one item to Fleet, with the thread so far. Answered, and held nowhere. */
  askLesson: (lessonId: string, question: string, history: AskTurn[]) => Promise<AskLessonRead>;
};

export const LESSONS_CHANNELS = {
  readRetro: "bridge:read-retro",
  readLessons: "bridge:read-lessons",
  agreeLesson: "bridge:agree-lesson",
  disagreeLesson: "bridge:disagree-lesson",
  startLessonSession: "bridge:start-lesson-session",
  reviewLessons: "bridge:review-lessons",
  askLesson: "bridge:ask-lesson",
} as const;
