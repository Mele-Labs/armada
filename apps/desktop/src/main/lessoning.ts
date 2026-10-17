// Answering a retro item: agree, disagree, or start a Session on it.
//
// Beside `command.ts` rather than inside it: the subject is an item, and what comes back is the
// item. A second answer on one while the first is out is refused here rather than sent.
//
// It holds no connection. The `Board` it is handed is what an act needs.

import type { Lesson, LessonAnswer, LessonSessionStarted } from "@armada/protocol";
import type { LessonSessionRead } from "@armada/screens/src/lessons/session-wire";
import { ask } from "./request";
// Type-only, and therefore not a cycle at runtime: `Board` is declared where the acts are.
import type { Board } from "./command";

/** Retro items with an answer in flight, by item id. */
export class Lessoning {
  private readonly board: Board;
  private readonly answering = new Set<string>();

  constructor(board: Board) {
    this.board = board;
  }

  /**
   * Agree with one retro item by starting a Session on its repository (`start_lesson_session`).
   * **Fleet sends the brief as the Session's first message**, so the Session is running when this
   * answers: the item, the Session's id and the brief sent. Shares `answer`'s one-press guard, so an
   * item cannot be agreed with and started on at once.
   */
  async startSession(lessonId: string): Promise<LessonSessionRead> {
    if (this.answering.has(lessonId)) {
      return { ok: false, outcome: { ok: false, why: "already_answering_lesson" } };
    }
    const port = this.board.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    this.answering.add(lessonId);
    try {
      const sent = await ask(port, "POST", `/lessons/${encodeURIComponent(lessonId)}/session`);
      if (sent.ok !== true) return { ok: false, outcome: sent.outcome };
      return { ok: true, started: sent.body as LessonSessionStarted };
    } finally {
      this.answering.delete(lessonId);
    }
  }

  /**
   * Agree or disagree with one retro item. Fleet starts a Session for an Armada or
   * Manifest item and saves a Kit item; the item comes back as it now stands.
   */
  async answer(lessonId: string, answer: "agree" | "disagree"): Promise<LessonAnswer> {
    if (this.answering.has(lessonId)) {
      return { ok: false, outcome: { ok: false, why: "already_answering_lesson" } };
    }
    const port = this.board.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    this.answering.add(lessonId);
    try {
      const sent = await ask(port, "POST", `/lessons/${encodeURIComponent(lessonId)}/${answer}`);
      if (sent.ok !== true) return { ok: false, outcome: sent.outcome };
      return { ok: true, lesson: sent.body as Lesson };
    } finally {
      this.answering.delete(lessonId);
    }
  }
}
