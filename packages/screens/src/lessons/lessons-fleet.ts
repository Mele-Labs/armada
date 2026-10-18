// What the mock Fleet does with an answer to a retro item, so a walk can agree
// with one item, disagree with another and read the saved ones. The rule is
// `docs/concepts/retro.md`: an Armada or Manifest item starts a Session, a Kit item is saved (and its command added to Kit's allowlist, where
// it carries one), a disagreed item is discarded.

import type { Lesson, LessonAnswer, LessonState } from "@armada/protocol";

import type { AskLessonRead, LessonAsk, LessonReview, LessonReviewRead } from "./review-wire";
import type { LessonSessionRead } from "./session-wire";

/** The listing as `GET /lessons?state=` answers it. */
export function listed(lessons: readonly Lesson[], state: "open" | "accepted"): Lesson[] {
  return lessons.filter((one) => one.state === state);
}

/** One answer taken: the list after it, and the item as it now stands. */
export function answered(
  lessons: readonly Lesson[],
  id: string,
  answer: "agree" | "disagree",
): { lessons: Lesson[]; answer: LessonAnswer } {
  const was = lessons.find((one) => one.id === id);
  // No such item: refused as Fleet refuses one it does not hold.
  if (was === undefined) {
    return { lessons: [...lessons], answer: { ok: false, outcome: { ok: false, why: "not_connected" } } };
  }
  // Only a Kit item has an agree of its own: any other is agreed by starting a Session.
  if (answer === "agree" && was.lands_in !== "kit") {
    return { lessons: [...lessons], answer: { ok: false, outcome: { ok: false, why: "not_connected" } } };
  }
  const state: LessonState = answer === "disagree" ? "discarded" : "accepted";
  // Accepting a Kit item that carries a change applies it, and the item keeps what was applied.
  const applied = state === "accepted" ? was.change : undefined;
  const now: Lesson = {
    ...was,
    state,
    ...(applied === undefined ? {} : { applied }),
  };
  return {
    lessons: lessons.map((one) => (one.id === id ? now : one)),
    answer: { ok: true, lesson: now },
  };
}

/**
 * Agreeing by a Session: `answered`'s sibling. A Kit item is refused as Fleet refuses it, an item that is not open
 * answers with the Session it already started, and the brief is the item's request, sent as the first message.
 * The Session itself is not minted: this mock holds no Sessions, so the id names one Sessions would hold.
 */
export function sessionStarted(lessons: readonly Lesson[], id: string): { lessons: Lesson[]; read: LessonSessionRead } {
  const was = lessons.find((one) => one.id === id) as Lesson | undefined;
  if (was === undefined) {
    return { lessons: [...lessons], read: { ok: false, outcome: { ok: false, why: "not_connected" } } };
  }
  if (was.lands_in === "kit" || was.lands_in === undefined) {
    return {
      lessons: [...lessons],
      read: {
        ok: false,
        outcome: {
          ok: false,
          why: "refused",
          error: {
            code: "fleet.lesson_session_nothing_to_work_on",
            message: "Kit is no repository, so there is nothing for a Job to work on. Accept it instead.",
            run_id: "mock",
            fields: {},
            chain: [],
          },
        },
      },
    };
  }
  const session_id = was.session_proposed ?? `s-${id.replace(/[^a-z0-9]/gi, "").slice(0, 8).toUpperCase().padEnd(8, "0")}`;
  const now: Lesson = { ...was, state: "agreed", session_proposed: session_id };
  const brief = [was.title ?? was.statement, was.what, was.fix === undefined ? undefined : `Change: ${was.fix}`]
    .filter((one): one is string => one !== undefined && one !== "")
    .join("\n\n");
  return {
    lessons: lessons.map((one) => (one.id === id ? now : one)),
    read: { ok: true, started: { lesson: now, session_id, brief } },
  };
}

/**
 * What the mock Fleet's review makes of the open items: newest first as listed, an item with the
 * same headline as an earlier one folded into it, and one kept before a place was named set aside.
 * Nothing the real model weighs; enough to walk every state of the guided review.
 */
export function reviewed(lessons: readonly Lesson[]): LessonReviewRead {
  const open = listed(lessons, "open");
  const entries: LessonReview["entries"] = [];
  const set_aside: LessonReview["set_aside"] = [];
  for (const one of open) {
    if (one.lands_in === undefined) {
      set_aside.push({ lesson_id: one.id, why: "It was kept before a fix place was named, so it cannot be acted on." });
      continue;
    }
    const headline = (one.title ?? one.statement).toLowerCase();
    const twin = entries.find((entry) => {
      const first = open.find((other) => other.id === entry.lesson_id);
      return first !== undefined && (first.title ?? first.statement).toLowerCase() === headline;
    });
    if (twin !== undefined) twin.merged_ids.push(one.id);
    else {
      entries.push({
        lesson_id: one.id,
        merged_ids: [],
        reason: one.lands_in === "armada" ? "Armada's own fault cost a Job." : "It names a change you can make today.",
      });
    }
  }
  return { ok: true, review: { model: "mock", entries, set_aside } };
}

/** A canned answer to a question, so the thread can be walked. An empty question is refused as Fleet refuses it. */
export function askedAbout(lessons: readonly Lesson[], id: string, ask: LessonAsk): AskLessonRead {
  const lesson = lessons.find((one) => one.id === id);
  if (lesson === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
  if (ask.question.trim() === "") return { ok: false, outcome: { ok: false, why: "not_connected" } };
  const earlier = (ask.history ?? []).length === 0 ? "" : " That is the same answer as before: the record holds nothing more.";
  return { ok: true, answer: { answer: `The record cited for this item does not show more than its own words say.${earlier}` } };
}
