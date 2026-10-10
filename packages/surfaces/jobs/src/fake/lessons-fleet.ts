// What the mock Fleet does with an answer to a retro item, so a walk can agree
// with one item, disagree with another and read the saved ones. The rule is
// `docs/concepts/retro.md`: an Armada or Manifest item proposes a Job at the
// approval gate, a Kit item is saved (and its command added to Kit's allowlist, where
// it carries one), a disagreed item is discarded.

import type { JobSummary, Lesson, LessonAnswer, LessonState } from "@armada/protocol";

import type { AskLessonRead, LessonAsk, LessonReview, LessonReviewRead } from "../review-wire";

/** The Job an agreed Armada or Manifest item proposes, at the approval gate. */
function proposedFor(lesson: Lesson, jobs: readonly JobSummary[], at: string): JobSummary {
  const ordinal = jobs.length + 1;
  return {
    id: `01M2E0PROPOSED${String(ordinal).padStart(2, "0")}FROMRETRO`,
    handle: `${ordinal}-${(lesson.title ?? lesson.statement).toLowerCase().replace(/[^a-z0-9]+/g, "-").slice(0, 40)}`,
    title: lesson.fix ?? lesson.title ?? lesson.statement,
    status: "awaiting_approval",
    workflow_id: "",
    owner_manifest_id: jobs[0]?.owner_manifest_id ?? "",
    origin: "manual",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: at,
  };
}

/** The listing as `GET /lessons?state=` answers it. */
export function listed(lessons: readonly Lesson[], state: "open" | "accepted"): Lesson[] {
  return lessons.filter((one) => one.state === state);
}

/** One answer taken: the list after it, the item as it now stands, and the Job it proposed, if any. */
export function answered(
  lessons: readonly Lesson[],
  id: string,
  answer: "agree" | "disagree",
  jobs: readonly JobSummary[],
  at: string,
): { lessons: Lesson[]; answer: LessonAnswer; job?: JobSummary } {
  const was = lessons.find((one) => one.id === id);
  // No such item: refused as Fleet refuses one it does not hold.
  if (was === undefined) {
    return { lessons: [...lessons], answer: { ok: false, outcome: { ok: false, why: "not_connected" } } };
  }
  const state: LessonState = answer === "disagree" ? "discarded" : was.lands_in === "kit" ? "accepted" : "agreed";
  const job = state === "agreed" ? proposedFor(was, jobs, at) : undefined;
  // Accepting a Kit item that carries a change applies it, and the item keeps what was applied.
  const applied = state === "accepted" ? was.change : undefined;
  const now: Lesson = {
    ...was,
    state,
    ...(job === undefined ? {} : { job_proposed: job.id }),
    ...(applied === undefined ? {} : { applied }),
  };
  return {
    lessons: lessons.map((one) => (one.id === id ? now : one)),
    answer: { ok: true, lesson: now },
    ...(job === undefined ? {} : { job }),
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
  const earlier = ask.history.length === 0 ? "" : " That is the same answer as before: the record holds nothing more.";
  return { ok: true, answer: { answer: `The record cited for this item does not show more than its own words say.${earlier}` } };
}
