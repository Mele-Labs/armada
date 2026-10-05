// The Lessons page and a Job's retro sheet, as the owner reads and answers them
// (`docs/concepts/retro.md`): each item as words, a headline, what happened and
// a fix, and Agree and Disagree under it.
//
// Browser tests, `Worktrees.test.tsx`'s reason: the claims are what a surface
// does with an answer, and a story cannot mount a screen.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { JobRetro, Lesson, LessonAnswer, LessonState } from "@armada/protocol";

import { JobRetroSheet, Lessons } from "./Lessons";
import { mount, unmount } from "./mounted";

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
  fix: "Compare against origin/main, where the branch is cut from.",
});
const KIT = lesson({
  id: "l-kit",
  who: "drone",
  lands_in: "kit",
  statement: "A Drone waited on grep.",
  title: "A Drone had to wait for grep to be allowed",
  what: "It asked to run grep on a check log.",
  fix: "Add grep on .armada/checks to the allowlist.",
});
const MANIFEST = lesson({
  id: "l-manifest",
  who: "fleet",
  lands_in: "manifest",
  statement: "A docs edit ran every Rust test.",
  title: "A docs edit set off every Rust test",
  what: "4211 tests and a 7.5 minute compile.",
  fix: "Run only xtask's tests when only apps/ or packages/ change.",
});
const OLD = lesson({
  id: "l-old",
  who: "owner",
  statement: "The Judge's question waited in the dock while the plan was read twice.",
});
const SAVED = lesson({ ...KIT, id: "l-saved", state: "accepted", title: "grep was allowed once already" });

type Acts = { agree?: (id: string) => Promise<LessonAnswer>; disagree?: (id: string) => Promise<LessonAnswer> };

/** Mount the page over an open list and a saved one, recording what was read. */
function opened(open: Lesson[], saved: Lesson[] = [], acts: Acts = {}) {
  const read = vi.fn(async (state: LessonState | "open" | "accepted") => ({
    ok: true as const,
    lessons: state === "accepted" ? saved : open,
  }));
  const agree = vi.fn(acts.agree ?? (async (id: string) => ({ ok: true as const, lesson: lesson({ ...ARMADA, id, state: "agreed", job_proposed: "01K7JOB" }) })));
  const disagree = vi.fn(acts.disagree ?? (async (id: string) => ({ ok: true as const, lesson: lesson({ ...ARMADA, id, state: "discarded" }) })));
  const opens = vi.fn();
  const job = vi.fn();
  mount(
    <Lessons
      onReadLessons={read}
      onReadRetro={async () => ({ ok: true, retro: { job_id: "j", state: "pending", record: {} } })}
      onAgreeLesson={agree}
      onDisagreeLesson={disagree}
      onOpenJob={job}
      repository={null}
      floor={false}
    />,
  );
  return { read, agree, disagree, opens, job };
}

const card = (title: RegExp | string) => page.getByRole("listitem").filter({ hasText: title });

/** The label row's words in a card or a sheet. The marks beside them carry a tooltip's copy of each. */
const wordsOf = (scope: ReturnType<typeof card>): string[] =>
  [...scope.element().querySelectorAll(".armada-lesson__word")].map((one) => one.textContent ?? "");

test("an item reads as words for whose way and where it lands, a headline, what happened, a fix and two answers", async () => {
  opened([ARMADA]);
  const one = card(/blamed the Drone/);
  await expect.element(one).toBeVisible();
  // Words, never marks alone: the label row says them.
  expect(wordsOf(one)).toEqual(["Fleet", "Armada"]);
  // The marks stay beside the words, with their tooltips' names.
  await expect.element(one.getByRole("img", { name: "Fleet" })).toBeInTheDocument();
  await expect.element(one.getByRole("img", { name: "Lands in Armada" })).toBeInTheDocument();
  await expect.element(one.getByRole("heading", { name: "The gate blamed the Drone for Fleet's own mistake" })).toBeVisible();
  await expect.element(one.getByText(/two commits behind origin/)).toBeVisible();
  await expect.element(one.getByText("Fix", { exact: true })).toBeVisible();
  await expect.element(one.getByText(/Compare against origin\/main/)).toBeVisible();
  expect(one.getByRole("button", { name: /^(Agree|Disagree)$/ }).elements().map((b) => b.textContent)).toEqual(["Agree", "Disagree"]);
});

test("the three whose-ways and three places are each a word", async () => {
  opened([ARMADA, KIT, MANIFEST, OLD]);
  await expect.element(card(/in the dock/)).toBeVisible();
  expect(wordsOf(card(/grep to be allowed/))).toEqual(["Drone", "Kit"]);
  expect(wordsOf(card(/every Rust test/))).toEqual(["Fleet", "Manifest"]);
  // Written before a place was named: whose way alone.
  expect(wordsOf(card(/in the dock/))).toEqual(["You"]);
});

test("each Agree names what it does for its item's place, and Disagree discards", async () => {
  opened([ARMADA, KIT, MANIFEST]);
  // The bubble opens after the tooltip delay, so its words are what is waited for.
  const tip = async (title: RegExp, name: string, words: string) => {
    await card(title).getByRole("button", { name, exact: true }).hover();
    await expect.element(card(title).getByText(words, { exact: true })).toBeVisible();
  };
  await tip(/blamed the Drone/, "Agree", "Proposes a Job on Armada's repository");
  await tip(/grep to be allowed/, "Agree", "Saves it under Accepted");
  await tip(/every Rust test/, "Agree", "Proposes a Job on the Manifest's repository");
  await tip(/every Rust test/, "Disagree", "Discards it");
});

test("an item written before the headline draws its statement as the body, with no answers, since Fleet refuses both", async () => {
  opened([OLD]);
  const one = card(/in the dock/);
  await expect.element(one.getByText(/waited in the dock/)).toBeVisible();
  expect(one.getByRole("heading").elements()).toHaveLength(0);
  expect(one.getByText("Fix", { exact: true }).elements()).toHaveLength(0);
  expect(one.getByRole("button", { name: /^(Agree|Disagree)$/ }).elements()).toHaveLength(0);
});

test("agreeing with an Armada item leaves a link to the Job it proposed, and the link opens it", async () => {
  const { agree, job } = opened([ARMADA]);
  await card(/blamed the Drone/).getByRole("button", { name: "Agree", exact: true }).click();
  expect(agree).toHaveBeenCalledWith("l-armada");
  const one = card(/blamed the Drone/);
  await expect.element(one.getByText("Agreed")).toBeVisible();
  expect(one.getByRole("button", { name: "Agree", exact: true }).elements()).toHaveLength(0);
  await one.getByRole("button", { name: "Proposed Job" }).click();
  expect(job).toHaveBeenCalledWith("01K7JOB");
});

test("agreeing with a Kit item, and disagreeing with any, takes it off the list", async () => {
  const { agree, disagree } = opened([KIT, MANIFEST], [], {
    agree: async (id) => ({ ok: true, lesson: lesson({ ...KIT, id, state: "accepted" }) }),
  });
  await card(/grep to be allowed/).getByRole("button", { name: "Agree", exact: true }).click();
  expect(agree).toHaveBeenCalledWith("l-kit");
  await expect.poll(() => card(/grep to be allowed/).elements()).toHaveLength(0);
  await card(/every Rust test/).getByRole("button", { name: "Disagree", exact: true }).click();
  expect(disagree).toHaveBeenCalledWith("l-manifest");
  await expect.poll(() => card(/every Rust test/).elements()).toHaveLength(0);
});

test("a refused answer says why, and the item stays with both answers", async () => {
  opened([ARMADA], [], { agree: async () => ({ ok: false, outcome: { ok: false, why: "already_answering_lesson" } }) });
  await card(/blamed the Drone/).getByRole("button", { name: "Agree", exact: true }).click();
  await expect.element(page.getByText(/already in flight/)).toBeVisible();
  expect(card(/blamed the Drone/).getByRole("button", { name: /^(Agree|Disagree)$/ }).elements()).toHaveLength(2);
});

test("Open and Accepted sit beside the places, and Accepted reads the saved items with no answers on them", async () => {
  const { read } = opened([ARMADA], [SAVED]);
  await expect.element(card(/blamed the Drone/)).toBeVisible();
  expect(read).toHaveBeenLastCalledWith("open");
  await page.getByRole("tab", { name: "Accepted", exact: true }).click();
  await expect.element(card(/allowed once already/)).toBeVisible();
  expect(read).toHaveBeenLastCalledWith("accepted");
  expect(card(/blamed the Drone/).elements()).toHaveLength(0);
  expect(card(/allowed once already/).getByRole("button", { name: /^(Agree|Disagree)$/ }).elements()).toHaveLength(0);
  await page.getByRole("tab", { name: "Open", exact: true }).click();
  await expect.element(card(/blamed the Drone/)).toBeVisible();
  const tabs = page.getByRole("tab").elements().map((t) => t.textContent);
  expect(tabs).toEqual(["All", "Armada", "Kit", "Manifest", "Open", "Accepted"]);
});

test("an empty list draws nothing", async () => {
  opened([]);
  await expect.element(page.getByRole("tab", { name: "Open", exact: true })).toBeVisible();
  expect(page.getByRole("list", { name: "Lessons" }).elements()).toHaveLength(0);
});

const RETRO: JobRetro = {
  job_id: "01K6JOB3",
  state: "written",
  items: [
    {
      id: "l-armada",
      who: "fleet",
      lands_in: "armada",
      statement: "s",
      title: "The gate blamed the Drone for Fleet's own mistake",
      what: "It compared against a stale main.",
      fix: "Compare against origin/main.",
      evidence: ["check:1"],
    },
  ],
  record: {
    failed_checks: [{ cite: "check:1", name: "out_of_bounds", run: "gate", produced: "armada.yml changed" }],
  },
};

test("the retro sheet draws the same item, keeps its evidence behind a control, and answers the same way", async () => {
  const agree = vi.fn(async (id: string) => ({ ok: true as const, lesson: lesson({ ...ARMADA, id, state: "agreed", job_proposed: "01K7JOB" }) }));
  mount(
    <JobRetroSheet
      jobId="01K6JOB3"
      job="Job 3"
      read={async () => ({ ok: true, retro: RETRO })}
      onAgreeLesson={agree}
      onDisagreeLesson={async () => ({ ok: true, lesson: lesson({ ...ARMADA, state: "discarded" }) })}
      floor={false}
      onClose={() => {}}
    />,
  );
  const sheet = page.getByRole("dialog", { name: "Retro" });
  await expect.element(sheet.getByRole("heading", { name: /blamed the Drone/ })).toBeVisible();
  await expect.element(sheet.getByRole("listitem")).toBeVisible();
  expect(wordsOf(sheet)).toEqual(["Fleet", "Armada"]);
  await expect.element(sheet.getByText(/Compare against origin\/main/)).toBeVisible();
  // Collapsed until asked for.
  expect(sheet.getByText("armada.yml changed").elements()).toHaveLength(0);
  await sheet.getByRole("button", { name: "Evidence" }).click();
  await expect.element(sheet.getByText("armada.yml changed")).toBeVisible();
  await sheet.getByRole("button", { name: "Agree", exact: true }).click();
  expect(agree).toHaveBeenCalledWith("l-armada");
  await expect.element(sheet.getByText("Agreed")).toBeVisible();
});
