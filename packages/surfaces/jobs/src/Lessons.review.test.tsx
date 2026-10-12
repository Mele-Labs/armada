// The Retros page's guided review, as a person walks it: Review beside the
// tabs, one card at a time in the model's order, the same answers as the list,
// a thread of questions per card and the items set aside.
//
// Browser tests, `Lessons.test.tsx`'s reason: the claims are what a surface does
// with an answer, and a story cannot mount a screen.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Lesson, LessonAnswer, Outcome } from "@armada/protocol";

import { Lessons } from "./Lessons";
import type { AskLessonRead, AskTurn, LessonReview, LessonReviewRead } from "./review-wire";
import { mount, unmount } from "@armada/screens/src/mounted";

afterEach(unmount);

const AT = "2026-10-02T22:05:00.000Z";

function lesson(over: Partial<Lesson> & Pick<Lesson, "id" | "statement" | "who">): Lesson {
  return { job_id: "01K6JOB3", handle: "3-retire-guides", at: AT, evidence: [], state: "open", ...over };
}

const ARMADA = lesson({
  id: "l-armada",
  who: "fleet",
  lands_in: "armada",
  statement: "The gate compared against a stale main.",
  title: "The gate blamed the Drone for Fleet's own mistake",
  what: "It compared the step against a local main two commits behind origin.",
  fix: "Compare against origin/main.",
});
const TWIN = lesson({ ...ARMADA, id: "l-twin", job_id: "01K6JOB2", handle: "2-fix-gate", title: "A stale main blamed the Drone" });
const KIT = lesson({
  id: "l-kit",
  who: "drone",
  lands_in: "kit",
  statement: "A Drone waited on grep.",
  title: "A Drone had to wait for grep to be allowed",
  what: "It asked to run grep on a check log.",
  fix: "Add grep to the allowlist.",
  change: { kind: "allow_command", command: "grep -r x .armada" },
});
const MANIFEST = lesson({
  id: "l-manifest",
  who: "fleet",
  lands_in: "manifest",
  statement: "A docs edit ran every Rust test.",
  title: "A docs edit set off every Rust test",
  what: "4211 tests.",
  fix: "Run only xtask's tests.",
});
const STALE = lesson({ id: "l-stale", who: "owner", lands_in: "manifest", statement: "Old.", title: "A very old complaint" });

const review = (over: Partial<LessonReview> = {}): LessonReview => ({
  model: "sonnet",
  entries: [
    { lesson_id: "l-armada", merged_ids: ["l-twin"], reason: "Armada's own fault cost two Jobs a rerun." },
    { lesson_id: "l-kit", merged_ids: [], reason: "One command unblocks every Drone." },
    { lesson_id: "l-manifest", merged_ids: [], reason: "It wastes seven minutes a docs edit." },
  ],
  set_aside: [{ lesson_id: "l-stale", why: "Two weeks old and fixed since." }],
  ...over,
});

const all = [ARMADA, TWIN, KIT, MANIFEST, STALE];

type Wiring = {
  reviewed?: () => Promise<LessonReviewRead>;
  asked?: (id: string, question: string, history: AskTurn[]) => Promise<AskLessonRead>;
  agree?: (id: string) => Promise<LessonAnswer>;
  disagree?: (id: string) => Promise<LessonAnswer>;
};

function opened(wiring: Wiring = {}, lessons: Lesson[] = all) {
  const read = vi.fn(async () => ({ ok: true as const, lessons }));
  const reviewLessons = vi.fn(wiring.reviewed ?? (async () => ({ ok: true as const, review: review() })));
  const askLesson = vi.fn(wiring.asked ?? (async () => ({ ok: true as const, answer: { answer: "Two Jobs, by the gate's runs." } })));
  const agree = vi.fn(wiring.agree ?? (async (id: string) => ({ ok: true as const, lesson: lesson({ ...ARMADA, id, state: "agreed", job_proposed: "01K7JOB" }) })));
  const disagree = vi.fn(wiring.disagree ?? (async (id: string) => ({ ok: true as const, lesson: lesson({ ...ARMADA, id, state: "discarded" }) })));
  mount(
    <Lessons
      onReadLessons={read}
      onReadRetro={async () => ({ ok: true, retro: { job_id: "j", state: "pending", record: {} } })}
      onAgreeLesson={agree}
      onDisagreeLesson={disagree}
      onReviewLessons={reviewLessons}
      onAskLesson={askLesson}
      repository={null}
      floor={false}
    />,
  );
  return { read, reviewLessons, askLesson, agree, disagree };
}

const start = async () => {
  await expect.element(page.getByRole("button", { name: "Review", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Review", exact: true }).click();
};

const refused: Outcome = { ok: false, why: "not_connected" };

test("Review sits beside the tabs and asks for the review once on entering", async () => {
  const { reviewLessons } = opened();
  await expect.element(page.getByRole("tab", { name: "Armada" })).toBeVisible();
  expect(reviewLessons).not.toHaveBeenCalled();
  await start();
  await expect.element(page.getByText("1 of 3")).toBeVisible();
  expect(reviewLessons).toHaveBeenCalledTimes(1);
});

test("one card at a time, with the model's reason and the list's own answers", async () => {
  opened();
  await start();
  await expect.element(page.getByText("Why this one")).toBeVisible();
  await expect.element(page.getByText("Armada's own fault cost two Jobs a rerun.")).toBeVisible();
  await expect.element(page.getByRole("heading", { name: "The gate blamed the Drone for Fleet's own mistake" })).toBeVisible();
  // Only the first card is drawn.
  expect(page.getByRole("heading", { name: /grep/ }).elements()).toHaveLength(0);
  await expect.element(page.getByRole("button", { name: "Create Job" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Reject change" })).toBeVisible();
});

test("an answer is the list's answer, and the review moves on to the next card", async () => {
  const { agree } = opened();
  await start();
  await page.getByRole("button", { name: "Create Job" }).click();
  expect(agree).toHaveBeenCalledWith("l-armada");
  await expect.element(page.getByText("2 of 3")).toBeVisible();
  // The Kit item carries a change, so its words are the list's: Update Kit.
  await expect.element(page.getByRole("button", { name: "Update Kit" })).toBeVisible();
});

test("a refused Update Kit stays on the card in Fleet's words with both buttons", async () => {
  const { agree } = opened({ agree: async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }) });
  await start();
  await page.getByRole("button", { name: "Skip" }).click();
  await page.getByRole("button", { name: "Update Kit" }).click();
  expect(agree).toHaveBeenCalledWith("l-kit");
  await expect.element(page.getByText("The answer was not taken")).toBeVisible();
  await expect.element(page.getByText("2 of 3")).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Reject change" })).toBeVisible();
});

test("Skip keeps the item open and Back returns to it", async () => {
  const { agree, disagree } = opened();
  await start();
  await page.getByRole("button", { name: "Skip" }).click();
  await expect.element(page.getByText("2 of 3")).toBeVisible();
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect.element(page.getByText("1 of 3")).toBeVisible();
  expect(agree).not.toHaveBeenCalled();
  expect(disagree).not.toHaveBeenCalled();
  // Nothing before the first card to go back to.
  await expect.element(page.getByRole("button", { name: "Back", exact: true })).toBeDisabled();
});

test("skipping past the last card offers the ones skipped again, then the list", async () => {
  opened();
  await start();
  await page.getByRole("button", { name: "Skip" }).click();
  await page.getByRole("button", { name: "Skip" }).click();
  await page.getByRole("button", { name: "Skip" }).click();
  await expect.element(page.getByText("That was the last one")).toBeVisible();
  await page.getByRole("button", { name: /Go through the 3 you skipped/ }).click();
  await expect.element(page.getByText("1 of 3")).toBeVisible();
});

test("answering every card ends on Nothing left to review, and the way back to the list reads it again", async () => {
  const { read } = opened();
  await start();
  await page.getByRole("button", { name: "Reject change" }).click();
  await page.getByRole("button", { name: "Reject change" }).click();
  await page.getByRole("button", { name: "Reject change" }).click();
  await expect.element(page.getByText("Nothing left to review")).toBeVisible();
  const before = read.mock.calls.length;
  await page.getByRole("button", { name: "Back to the list" }).first().click();
  await expect.element(page.getByRole("tab", { name: "Armada" })).toBeVisible();
  await vi.waitFor(() => expect(read.mock.calls.length).toBeGreaterThan(before));
});

test("a card that stands for duplicates says so, and the duplicates are named on request", async () => {
  opened();
  await start();
  const seen = page.getByRole("button", { name: "Also seen in 1 other Job" });
  await expect.element(seen).toHaveAttribute("aria-expanded", "false");
  await seen.click();
  await expect.element(page.getByText("A stale main blamed the Drone")).toBeVisible();
});

test("Reject asks nothing extra of the duplicates unless the box is ticked", async () => {
  const { disagree } = opened();
  await start();
  const box = page.getByRole("checkbox", { name: "Also reject the 1 duplicate" });
  await expect.element(box).not.toBeChecked();
  await page.getByRole("button", { name: "Reject change" }).click();
  await expect.element(page.getByText("2 of 3")).toBeVisible();
  expect(disagree.mock.calls.map((call) => call[0])).toEqual(["l-armada"]);
});

test("with the box ticked Reject also rejects the duplicates, after the card itself", async () => {
  const { disagree } = opened();
  await start();
  await page.getByRole("checkbox", { name: "Also reject the 1 duplicate" }).click();
  await page.getByRole("button", { name: "Reject change" }).click();
  await vi.waitFor(() => expect(disagree.mock.calls.map((call) => call[0])).toEqual(["l-armada", "l-twin"]));
});

test("a duplicate Fleet will not reject is said so, and the card was still rejected", async () => {
  opened({
    disagree: async (id) =>
      id === "l-twin"
        ? { ok: false, outcome: refused }
        : { ok: true, lesson: lesson({ ...ARMADA, id, state: "discarded" }) },
  });
  await start();
  await page.getByRole("checkbox", { name: "Also reject the 1 duplicate" }).click();
  await page.getByRole("button", { name: "Reject change" }).click();
  await expect.element(page.getByText("Some duplicates were not rejected")).toBeVisible();
});

test("a question goes to Fleet with the thread so far and its answer is drawn under the card", async () => {
  const { askLesson } = opened();
  await start();
  const field = page.getByRole("textbox", { name: "Ask about this item" });
  await field.fill("Which Jobs did this cost?");
  await page.getByRole("button", { name: "Send" }).click();
  await expect.element(page.getByText("Two Jobs, by the gate's runs.")).toBeVisible();
  expect(askLesson).toHaveBeenCalledWith("l-armada", "Which Jobs did this cost?", []);
  await field.fill("And the cost in time?");
  await page.getByRole("button", { name: "Send" }).click();
  await vi.waitFor(() => expect(askLesson).toHaveBeenCalledTimes(2));
  expect(askLesson.mock.calls[1]).toEqual([
    "l-armada",
    "And the cost in time?",
    [
      { role: "person", text: "Which Jobs did this cost?" },
      { role: "fleet", text: "Two Jobs, by the gate's runs." },
    ],
  ]);
});

test("a thread and a half-typed question survive moving to another card and back", async () => {
  opened();
  await start();
  await page.getByRole("textbox", { name: "Ask about this item" }).fill("Which Jobs?");
  await page.getByRole("button", { name: "Send" }).click();
  await expect.element(page.getByText("Two Jobs, by the gate's runs.")).toBeVisible();
  await page.getByRole("textbox", { name: "Ask about this item" }).fill("half a thought");
  await page.getByRole("button", { name: "Skip" }).click();
  await expect.element(page.getByText("2 of 3")).toBeVisible();
  expect(page.getByText("Two Jobs, by the gate's runs.").elements()).toHaveLength(0);
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect.element(page.getByText("Which Jobs?")).toBeVisible();
  await expect.element(page.getByText("Two Jobs, by the gate's runs.")).toBeVisible();
  await expect.element(page.getByRole("textbox", { name: "Ask about this item" })).toHaveValue("half a thought");
});

test("a question is held while it is out, and a refusal comes back in Fleet's words with the question to send again", async () => {
  let release: (read: AskLessonRead) => void = () => {};
  const { askLesson } = opened({ asked: () => new Promise<AskLessonRead>((done) => (release = done)) });
  await start();
  await page.getByRole("textbox", { name: "Ask about this item" }).fill("Why?");
  await page.getByRole("button", { name: "Send" }).click();
  await expect.element(page.getByRole("textbox", { name: "Ask about this item" })).toBeDisabled();
  release({ ok: false, outcome: refused });
  await expect.element(page.getByText("The question was not answered")).toBeVisible();
  await expect.element(page.getByRole("textbox", { name: "Ask about this item" })).toHaveValue("Why?");
  expect(askLesson).toHaveBeenCalledTimes(1);
});

test("the set aside are folded, say why, and Put back in the queue returns one to the end of it", async () => {
  opened();
  await start();
  const fold = page.getByRole("button", { name: "Set aside (1)" });
  await expect.element(fold).toHaveAttribute("aria-expanded", "false");
  expect(page.getByText("Two weeks old and fixed since.").elements()).toHaveLength(0);
  await fold.click();
  await expect.element(page.getByText("Two weeks old and fixed since.")).toBeVisible();
  await page.getByRole("button", { name: "Put back in the queue" }).click();
  await expect.element(page.getByText("1 of 4")).toBeVisible();
  expect(page.getByRole("button", { name: /Set aside/ }).elements()).toHaveLength(0);
});

test("the arrow keys move and no letter answers", async () => {
  const { agree, disagree } = opened();
  await start();
  await userEvent.keyboard("{ArrowRight}");
  await expect.element(page.getByText("2 of 3")).toBeVisible();
  await userEvent.keyboard("{ArrowLeft}");
  await expect.element(page.getByText("1 of 3")).toBeVisible();
  await userEvent.keyboard("ayjrc");
  expect(agree).not.toHaveBeenCalled();
  expect(disagree).not.toHaveBeenCalled();
  await expect.element(page.getByText("1 of 3")).toBeVisible();
});

test("arrow keys typed in the question field move the caret and not the card", async () => {
  opened();
  await start();
  await page.getByRole("textbox", { name: "Ask about this item" }).click();
  await userEvent.keyboard("{ArrowRight}");
  await expect.element(page.getByText("1 of 3")).toBeVisible();
});

test("while Fleet reads, the page says so; when it cannot, Fleet's words and a Retry that asks again", async () => {
  let calls = 0;
  const { reviewLessons } = opened({
    reviewed: async () => {
      calls += 1;
      return calls === 1 ? { ok: false, outcome: refused } : { ok: true, review: review() };
    },
  });
  await start();
  await expect.element(page.getByText("The review could not be made")).toBeVisible();
  await expect.element(page.getByText("Fleet is not connected. Nothing was sent.")).toBeVisible();
  await page.getByRole("button", { name: "Retry" }).click();
  await expect.element(page.getByText("1 of 3")).toBeVisible();
  expect(reviewLessons).toHaveBeenCalledTimes(2);
});

test("an empty review says there is nothing to review and leads back to the list", async () => {
  opened({ reviewed: async () => ({ ok: true, review: review({ entries: [], set_aside: [] }) }) }, []);
  await start();
  await expect.element(page.getByText("Nothing to review")).toBeVisible();
  await page.getByRole("button", { name: "Back to the list" }).first().click();
  await expect.element(page.getByRole("tab", { name: "Armada" })).toBeVisible();
});

test("an item the page no longer holds is left out of the queue", async () => {
  opened({}, [ARMADA, TWIN, MANIFEST, STALE]);
  await start();
  await expect.element(page.getByText("1 of 2")).toBeVisible();
});

test("the page offers no Review where the host gave it no review", async () => {
  mount(
    <Lessons
      onReadLessons={async () => ({ ok: true, lessons: [ARMADA] })}
      onReadRetro={async () => ({ ok: true, retro: { job_id: "j", state: "pending", record: {} } })}
      onAgreeLesson={async () => ({ ok: false, outcome: refused })}
      onDisagreeLesson={async () => ({ ok: false, outcome: refused })}
      repository={null}
      floor={false}
    />,
  );
  await expect.element(page.getByRole("tab", { name: "Armada" })).toBeVisible();
  expect(page.getByRole("button", { name: "Review", exact: true }).elements()).toHaveLength(0);
});
