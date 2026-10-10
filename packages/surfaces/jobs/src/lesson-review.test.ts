import { expect, test } from "vitest";
import type { Lesson } from "@armada/protocol";

import { reasonOf, resolved } from "./lesson-review";
import type { LessonReview } from "./review-wire";

const one = (id: string): Lesson => ({ id, job_id: "j", handle: "1-x", at: "2026-10-02T22:05:00.000Z", who: "fleet", statement: id, evidence: [], state: "open" });

const review: LessonReview = {
  model: "sonnet",
  entries: [
    { lesson_id: "a", merged_ids: ["b", "gone"], reason: "First." },
    { lesson_id: "gone", merged_ids: [], reason: "Answered since." },
    { lesson_id: "c", merged_ids: [], reason: "not ranked" },
  ],
  set_aside: [
    { lesson_id: "d", why: "Stale." },
    { lesson_id: "gone", why: "Also gone." },
  ],
};

test("the queue keeps the model's order and drops what the page no longer holds", () => {
  const { queue, aside } = resolved(review, [one("a"), one("b"), one("c"), one("d")]);
  expect(queue.map((entry) => entry.lesson.id)).toEqual(["a", "c"]);
  expect(queue[0]?.merged.map((twin) => twin.id)).toEqual(["b"]);
  expect(aside.map((entry) => [entry.lesson.id, entry.why])).toEqual([["d", "Stale."]]);
});

test("an item Fleet appended without a ranking says so in words", () => {
  expect(reasonOf("not ranked")).toBe("The review did not rank this one.");
  expect(reasonOf("It costs a rerun.")).toBe("It costs a rerun.");
});
