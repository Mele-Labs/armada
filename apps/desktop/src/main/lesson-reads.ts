// A retro, the Lessons listing, and the guided review over its open items.
//
// Each answers the caller and publishes nothing. `port` arrives as a function so this file never
// learns how a port is found, only that one might not be.

import type { LessonsRead, RetroRead, RetroSubject } from "@armada/protocol";
import type { AskLessonRead, AskTurn, LessonReviewRead } from "@armada/screens/src/lessons/review-wire";
import type { Picked } from "./picked";
import { askOf, lessonsOf, retroOf, reviewOf } from "./request";

export class LessonReads {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** One retro, answered to the surface that asked and held nowhere. */
  async readRetro(subject: RetroSubject): Promise<RetroRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    return await retroOf(port, subject);
  }

  /** The Lessons listing, narrowed to the calling window's pick: each window keeps its own. */
  async readLessons(picked: Picked, state: "open" | "accepted"): Promise<LessonsRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    return await lessonsOf(port, picked, state);
  }

  /** The guided review over the open items, narrowed to the calling window's pick as `readLessons` is. */
  async reviewLessons(picked: Picked): Promise<LessonReviewRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    return await reviewOf(port, picked);
  }

  /** One question about one item. Answered to the card that asked and held nowhere. */
  async askLesson(lessonId: string, question: string, history: AskTurn[]): Promise<AskLessonRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    return await askOf(port, lessonId, question, history);
  }
}
