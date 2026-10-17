// What the guided review keeps while the window is open, so leaving Retros and coming back finds
// the same card (`docs/concepts/retro.md`, *Reviewing*).
//
// **A store the host owns, not the page's state.** The page unmounts when the rail goes elsewhere,
// and everything a person built in the review (the order, the place in it, a thread of questions
// per item, what was put back, the duplicate boxes) went with it. The host makes one of these for
// the window and hands it to the page; a page given none makes its own, which holds for as long as
// that page does. **Held per repository pick, and for as long as the window is open**: nothing is
// written to disk, so a restart starts a review again.
//
// **The store makes the model call**, not the page. A person who leaves while Fleet is reading and
// comes back later finds the answer, because the call's result lands here whether or not anything
// is watching.

import type { ReviewTurn } from "@armada/components";
import type { Outcome } from "@armada/protocol";

import type { LessonReview, LessonReviewRead } from "./review-wire";

/** One item's thread of questions, kept by id so moving between cards loses nothing. */
export type Thread = { turns: ReviewTurn[]; draft: string; pending: boolean; failure?: string };

/** Where the review's one model call stands. */
export type Made = { state: "loading" } | { state: "failed"; outcome: Outcome } | { state: "ready"; review: LessonReview };

/** One pick's review as the person has left it. */
export type Held = {
  made: Made;
  /** Epoch milliseconds the model answered, for `Reviewed 12 minutes ago`. Absent until it has. */
  reviewedAt?: number;
  /** Where the person is in the cards still to answer. */
  at: number;
  /** Items the review set aside that the person put back in the queue, in the order put. */
  putBack: string[];
  threads: Record<string, Thread>;
  /** Cards whose duplicates are rejected with them: the box ticked. */
  also: string[];
  /** Which run of the review this is. A late answer to an earlier run is not this one's. */
  generation: number;
};

export type ReviewHold = {
  /** What is held for a pick, or `undefined` before its review was asked for. */
  get: (key: string) => Held | undefined;
  /**
   * Ask for a pick's review unless it already has one, held or on its way. **The only way a page
   * starts the model call on entering**, so returning to a held review makes none.
   */
  begin: (key: string, run: () => Promise<LessonReviewRead>) => void;
  /** Throw the held state away and ask again: `Review again`, and Retry after a failure. */
  again: (key: string, run: () => Promise<LessonReviewRead>) => void;
  /**
   * Change what is held. With `generation`, only if the review is still that run: a question
   * answered after `Review again` belongs to the review it was asked in.
   */
  update: (key: string, change: (held: Held) => Held, generation?: number) => void;
  /** Whether the page is showing the review and not the list. Remembered with the rest, as the tab is. */
  reviewing: () => boolean;
  setReviewing: (on: boolean) => void;
  subscribe: (listener: () => void) => () => void;
};

export function createReviewHold(now: () => number = Date.now): ReviewHold {
  const held = new Map<string, Held>();
  const listeners = new Set<() => void>();
  let showing = false;
  let runs = 0;
  const emit = () => listeners.forEach((one) => one());

  function start(key: string, run: () => Promise<LessonReviewRead>): void {
    runs += 1;
    const generation = runs;
    held.set(key, { made: { state: "loading" }, at: 0, putBack: [], threads: {}, also: [], generation });
    emit();
    void run().then((answer) => {
      const was = held.get(key);
      if (was === undefined || was.generation !== generation) return;
      held.set(
        key,
        answer.ok
          ? { ...was, made: { state: "ready", review: answer.review }, reviewedAt: now() }
          : { ...was, made: { state: "failed", outcome: answer.outcome } },
      );
      emit();
    });
  }

  return {
    get: (key) => held.get(key),
    begin: (key, run) => {
      if (!held.has(key)) start(key, run);
    },
    again: start,
    update: (key, change, generation) => {
      const was = held.get(key);
      if (was === undefined || (generation !== undefined && was.generation !== generation)) return;
      held.set(key, change(was));
      emit();
    },
    reviewing: () => showing,
    setReviewing: (on) => {
      if (showing === on) return;
      showing = on;
      emit();
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
  };
}
