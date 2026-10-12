// Leaving Retros in the middle of a review and coming back (`docs/concepts/retro.md`, *Reviewing*):
// the same card, its thread, what was put back and the duplicate boxes are there, the model is not
// asked again, and what was answered meanwhile is gone. Review again asks on purpose.
//
// "Leaving" is unmounting the page and mounting another over the same `ReviewHold`, which is what
// the host does with the window's own: the rail goes elsewhere and returns.
//
// Browser tests, `Lessons.review.test.tsx`'s reason.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { Lesson } from "@armada/protocol";

import { Lessons } from "./Lessons";
import { createReviewHold, type ReviewHold } from "./review-hold";
import type { AskLessonRead, LessonReview, LessonReviewRead } from "./review-wire";
import { mount, unmount } from "@armada/screens/src/mounted";

afterEach(unmount);

const AT = "2026-10-02T22:05:00.000Z";

function lesson(over: Partial<Lesson> & Pick<Lesson, "id" | "statement" | "who">): Lesson {
  return { job_id: "01K6JOB3", handle: "3-retire-guides", at: AT, evidence: [], state: "open", ...over };
}

const ARMADA = lesson({ id: "l-armada", who: "fleet", lands_in: "armada", statement: "s", title: "The gate blamed the Drone", what: "w", fix: "f" });
const TWIN = lesson({ ...ARMADA, id: "l-twin", job_id: "01K6JOB2", handle: "2-fix-gate", title: "A stale main blamed the Drone" });
const KIT = lesson({ id: "l-kit", who: "drone", lands_in: "kit", statement: "s", title: "A Drone waited on grep", what: "w", fix: "f" });
const MANIFEST = lesson({ id: "l-manifest", who: "fleet", lands_in: "manifest", statement: "s", title: "A docs edit set off every Rust test", what: "w", fix: "f" });
const STALE = lesson({ id: "l-stale", who: "owner", lands_in: "manifest", statement: "Old.", title: "A very old complaint" });

const review: LessonReview = {
  model: "sonnet",
  entries: [
    { lesson_id: "l-armada", merged_ids: ["l-twin"], reason: "First reason." },
    { lesson_id: "l-kit", merged_ids: [], reason: "Second reason." },
    { lesson_id: "l-manifest", merged_ids: [], reason: "Third reason." },
  ],
  set_aside: [{ lesson_id: "l-stale", why: "Fixed since." }],
};

const all = [ARMADA, TWIN, KIT, MANIFEST, STALE];

/** A page over `hold`, as the host mounts it. Each call is one visit to Retros. */
function visit(
  hold: ReviewHold,
  over: {
    lessons?: Lesson[];
    repository?: string | null;
    reviewed?: () => Promise<LessonReviewRead>;
    asked?: () => Promise<AskLessonRead>;
  } = {},
) {
  const reviewLessons = vi.fn(over.reviewed ?? (async () => ({ ok: true as const, review })));
  const askLesson = vi.fn(over.asked ?? (async () => ({ ok: true as const, answer: { answer: "Two Jobs." } })));
  mount(
    <Lessons
      onReadLessons={async () => ({ ok: true, lessons: over.lessons ?? all })}
      onReadRetro={async () => ({ ok: true, retro: { job_id: "j", state: "pending", record: {} } })}
      onAgreeLesson={async () => ({ ok: true, lesson: { ...ARMADA, state: "agreed", job_proposed: "01K7JOB" } })}
      onDisagreeLesson={async (id) => ({ ok: true, lesson: lesson({ ...ARMADA, id, state: "discarded" }) })}
      onReviewLessons={reviewLessons}
      onAskLesson={askLesson}
      hold={hold}
      repository={over.repository ?? null}
      floor={false}
    />,
  );
  return { reviewLessons, askLesson };
}

const start = async () => {
  await expect.element(page.getByRole("button", { name: "Review", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Review", exact: true }).click();
};

test("leaving mid-review and coming back resumes at the same card with its thread, and asks the model nothing", async () => {
  const hold = createReviewHold();
  const first = visit(hold);
  await start();
  await page.getByRole("textbox", { name: "Ask about this item" }).fill("Which Jobs?");
  await page.getByRole("button", { name: "Send" }).click();
  await expect.element(page.getByText("Two Jobs.")).toBeVisible();
  await page.getByRole("textbox", { name: "Ask about this item" }).fill("half a thought");
  await page.getByRole("button", { name: "Skip" }).click();
  await expect.element(page.getByText("2 of 3")).toBeVisible();
  expect(first.reviewLessons).toHaveBeenCalledTimes(1);

  unmount();
  const back = visit(hold);
  // Straight into the review: no Review press, and the same second card.
  await expect.element(page.getByText("2 of 3")).toBeVisible();
  await expect.element(page.getByRole("heading", { name: "A Drone waited on grep" })).toBeVisible();
  expect(back.reviewLessons).not.toHaveBeenCalled();
  // The first card's thread and half-typed question are there to go back to.
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect.element(page.getByText("Which Jobs?")).toBeVisible();
  await expect.element(page.getByText("Two Jobs.")).toBeVisible();
  await expect.element(page.getByRole("textbox", { name: "Ask about this item" })).toHaveValue("half a thought");
});

test("what was put back and the duplicate box are kept", async () => {
  const hold = createReviewHold();
  visit(hold);
  await start();
  await page.getByRole("checkbox", { name: "Also reject the 1 duplicate" }).click();
  await page.getByRole("button", { name: "Set aside (1)" }).click();
  await page.getByRole("button", { name: "Put back in the queue" }).click();
  await expect.element(page.getByText("1 of 4")).toBeVisible();

  unmount();
  visit(hold);
  await expect.element(page.getByText("1 of 4")).toBeVisible();
  await expect.element(page.getByRole("checkbox", { name: "Also reject the 1 duplicate" })).toBeChecked();
});

test("an item answered meanwhile is gone on return, and the order of the rest is not changed", async () => {
  const hold = createReviewHold();
  visit(hold);
  await start();
  await expect.element(page.getByText("1 of 3")).toBeVisible();

  unmount();
  // The first item was answered elsewhere, say from its Job's own sheet.
  const back = visit(hold, { lessons: [KIT, MANIFEST, STALE] });
  await expect.element(page.getByText("1 of 2")).toBeVisible();
  await expect.element(page.getByRole("heading", { name: "A Drone waited on grep" })).toBeVisible();
  expect(back.reviewLessons).not.toHaveBeenCalled();
});

test("Review mode itself is remembered, and so is the list", async () => {
  const hold = createReviewHold();
  visit(hold);
  await start();
  await expect.element(page.getByText("1 of 3")).toBeVisible();
  await page.getByRole("button", { name: "Back to the list" }).click();
  await expect.element(page.getByRole("tab", { name: "Armada" })).toBeVisible();

  unmount();
  visit(hold);
  // Left on the list, so the list is what comes back, with Review beside the tabs.
  await expect.element(page.getByRole("button", { name: "Review", exact: true })).toBeVisible();
  expect(page.getByText("1 of 3").elements()).toHaveLength(0);
});

test("pressing Review after the list resumes the held review and makes no new call", async () => {
  const hold = createReviewHold();
  const one = visit(hold);
  await start();
  await page.getByRole("button", { name: "Skip" }).click();
  await page.getByRole("button", { name: "Back to the list" }).click();
  await start();
  await expect.element(page.getByText("2 of 3")).toBeVisible();
  expect(one.reviewLessons).toHaveBeenCalledTimes(1);
});

test("it says how old the review is, and Review again asks on purpose and starts from the first card", async () => {
  const hold = createReviewHold(() => Date.now() - 12 * 60_000);
  const one = visit(hold);
  await start();
  await expect.element(page.getByText("Reviewed 12 minutes ago")).toBeVisible();
  await page.getByRole("textbox", { name: "Ask about this item" }).fill("Which Jobs?");
  await page.getByRole("button", { name: "Send" }).click();
  await expect.element(page.getByText("Two Jobs.")).toBeVisible();
  await page.getByRole("button", { name: "Skip" }).click();
  await expect.element(page.getByText("2 of 3")).toBeVisible();

  await page.getByRole("button", { name: "Review again" }).click();
  await expect.element(page.getByText("1 of 3")).toBeVisible();
  expect(one.reviewLessons).toHaveBeenCalledTimes(2);
  // The held thread went with the old review.
  expect(page.getByText("Two Jobs.").elements()).toHaveLength(0);
});

test("another repository pick has its own review, and the first is still held", async () => {
  const hold = createReviewHold();
  const one = visit(hold, { repository: "/work/a" });
  await start();
  await page.getByRole("button", { name: "Skip" }).click();
  await expect.element(page.getByText("2 of 3")).toBeVisible();

  unmount();
  // Reviewing is on, so the new pick starts its own review without a press.
  const other = visit(hold, { repository: "/work/b" });
  await expect.element(page.getByText("1 of 3")).toBeVisible();
  expect(other.reviewLessons).toHaveBeenCalledTimes(1);

  unmount();
  const first = visit(hold, { repository: "/work/a" });
  await expect.element(page.getByText("2 of 3")).toBeVisible();
  expect(first.reviewLessons).not.toHaveBeenCalled();
  expect(one.reviewLessons).toHaveBeenCalledTimes(1);
});

test("leaving while Fleet is still reading: the answer lands, and coming back finds it with no second call", async () => {
  const hold = createReviewHold();
  let release: (read: LessonReviewRead) => void = () => {};
  const one = visit(hold, { reviewed: () => new Promise<LessonReviewRead>((done) => (release = done)) });
  await start();
  await expect.element(page.getByText("Fleet is reading the open items")).toBeVisible();

  unmount();
  release({ ok: true, review });
  const back = visit(hold);
  await expect.element(page.getByText("1 of 3")).toBeVisible();
  expect(back.reviewLessons).not.toHaveBeenCalled();
  expect(one.reviewLessons).toHaveBeenCalledTimes(1);
});

test("a question still out when the person leaves is answered into the held thread", async () => {
  const hold = createReviewHold();
  let release: (read: AskLessonRead) => void = () => {};
  visit(hold, { asked: () => new Promise<AskLessonRead>((done) => (release = done)) });
  await start();
  await page.getByRole("textbox", { name: "Ask about this item" }).fill("Which Jobs?");
  await page.getByRole("button", { name: "Send" }).click();
  await expect.element(page.getByRole("textbox", { name: "Ask about this item" })).toBeDisabled();

  unmount();
  release({ ok: true, answer: { answer: "Two Jobs." } });
  await vi.waitFor(() => expect(hold.get("")?.threads["l-armada"]?.pending).toBe(false));
  visit(hold);
  await expect.element(page.getByText("Two Jobs.")).toBeVisible();
});

test("a failed review is held as failed, and Retry asks again", async () => {
  const hold = createReviewHold();
  let calls = 0;
  const one = visit(hold, {
    reviewed: async () => (++calls === 1 ? { ok: false, outcome: { ok: false, why: "not_connected" } } : { ok: true, review }),
  });
  await start();
  await expect.element(page.getByText("The review could not be made")).toBeVisible();
  await page.getByRole("button", { name: "Retry" }).click();
  await expect.element(page.getByText("1 of 3")).toBeVisible();
  expect(one.reviewLessons).toHaveBeenCalledTimes(2);
});
