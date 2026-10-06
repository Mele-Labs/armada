// What the mock Fleet does with an answer to a retro item, so a walk can agree
// with one item, disagree with another and read the saved ones. The rule is
// `docs/concepts/retro.md`: an Armada or Manifest item proposes a Job at the
// approval gate, a Kit item is saved (and its command added to Kit's allowlist, where
// it carries one), a disagreed item is discarded.

import type { JobSummary, Lesson, LessonAnswer, LessonState } from "@armada/protocol";

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
