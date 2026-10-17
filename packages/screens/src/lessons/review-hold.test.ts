import { expect, test, vi } from "vitest";

import { createReviewHold } from "./review-hold";
import { ageWords } from "./lesson-review";
import type { LessonReview, LessonReviewRead } from "./review-wire";

const review: LessonReview = { model: "sonnet", entries: [], set_aside: [] };
const ok = async (): Promise<LessonReviewRead> => ({ ok: true, review });

test("begin asks once per pick and a second begin is the same review", async () => {
  const hold = createReviewHold();
  const run = vi.fn(ok);
  hold.begin("a", run);
  hold.begin("a", run);
  expect(run).toHaveBeenCalledTimes(1);
  expect(hold.get("a")?.made.state).toBe("loading");
  await vi.waitFor(() => expect(hold.get("a")?.made.state).toBe("ready"));
  hold.begin("a", run);
  expect(run).toHaveBeenCalledTimes(1);
  hold.begin("b", run);
  expect(run).toHaveBeenCalledTimes(2);
});

test("the answer lands with its time, whether or not anything is subscribed", async () => {
  const hold = createReviewHold(() => 1234);
  hold.begin("a", ok);
  await vi.waitFor(() => expect(hold.get("a")?.reviewedAt).toBe(1234));
});

test("again throws the place away, and an answer to the earlier run is not the new run's", async () => {
  const hold = createReviewHold();
  let late: (read: LessonReviewRead) => void = () => {};
  hold.begin("a", () => new Promise<LessonReviewRead>((done) => (late = done)));
  hold.again("a", ok);
  await vi.waitFor(() => expect(hold.get("a")?.made.state).toBe("ready"));
  hold.update("a", (was) => ({ ...was, at: 3 }));
  late({ ok: false, outcome: { ok: false, why: "not_connected" } });
  await Promise.resolve();
  expect(hold.get("a")?.made.state).toBe("ready");
  hold.again("a", ok);
  expect(hold.get("a")?.at).toBe(0);
});

test("an update from an earlier run is dropped", async () => {
  const hold = createReviewHold();
  hold.begin("a", ok);
  const was = hold.get("a")!.generation;
  hold.again("a", ok);
  hold.update("a", (one) => ({ ...one, at: 9 }), was);
  expect(hold.get("a")?.at).toBe(0);
});

test("listeners hear a change, and the mode is held", () => {
  const hold = createReviewHold();
  const heard = vi.fn();
  const off = hold.subscribe(heard);
  hold.setReviewing(true);
  expect(hold.reviewing()).toBe(true);
  hold.setReviewing(true);
  expect(heard).toHaveBeenCalledTimes(1);
  off();
  hold.setReviewing(false);
  expect(heard).toHaveBeenCalledTimes(1);
});

test("the age is in whole units, rounded down", () => {
  const minute = 60_000;
  expect([0, 59_000, minute, 90_000, 12 * minute, 60 * minute, 150 * minute, 24 * 60 * minute, 49 * 60 * minute].map(ageWords)).toEqual([
    "just now",
    "just now",
    "1 minute ago",
    "1 minute ago",
    "12 minutes ago",
    "1 hour ago",
    "2 hours ago",
    "1 day ago",
    "2 days ago",
  ]);
});
