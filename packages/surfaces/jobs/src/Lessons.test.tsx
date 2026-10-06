// The Lessons page and a Job's retro sheet, as the owner reads and answers them
// (`docs/concepts/retro.md`): each item as words, a headline, what happened and
// a fix, and Agree and Disagree under it.
//
// Browser tests, `Worktrees.test.tsx`'s reason: the claims are what a surface
// does with an answer, and a story cannot mount a screen.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { JobRetro, Lesson, LessonAnswer, LessonState, RetroRead } from "@armada/protocol";

import { JobRetroSheet, Lessons } from "./Lessons";
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
function opened(
  open: Lesson[],
  saved: Lesson[] = [],
  acts: Acts = {},
  readRetro: (jobId: string) => Promise<RetroRead> = async () => ({ ok: true, retro: { job_id: "j", state: "pending", record: {} } }),
) {
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
      onReadRetro={readRetro}
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
  await expect.element(one.getByText("What would change", { exact: true })).toBeVisible();
  await expect.element(one.getByText(/Compare against origin\/main/)).toBeVisible();
  expect(one.getByRole("button", { name: /^(Create Job|Accept|Reject change)$/ }).elements().map((b) => b.textContent)).toEqual(["Create Job", "Reject change"]);
});

test("the three whose-ways and three places are each a word", async () => {
  opened([ARMADA, KIT, MANIFEST, OLD]);
  await expect.element(card(/in the dock/)).toBeVisible();
  expect(wordsOf(card(/grep to be allowed/))).toEqual(["Drone", "Kit"]);
  expect(wordsOf(card(/every Rust test/))).toEqual(["Fleet", "Manifest"]);
  // Written before a place was named: whose way alone.
  expect(wordsOf(card(/in the dock/))).toEqual(["You"]);
});

test("whose way and where the fix lands are one arrow, each end its own hue, with no arrow where no place is named", async () => {
  opened([ARMADA, OLD]);
  await expect.element(card(/blamed the Drone/)).toBeVisible();
  const route = card(/blamed the Drone/).element().querySelector(".armada-lesson__route")!;
  // Who, the arrow, then where: both ends words, each carrying the hue its name has.
  expect([...route.children].map((one) => (one.classList.contains("armada-lesson__arrow") ? "arrow" : "label"))).toEqual([
    "label",
    "arrow",
    "label",
  ]);
  expect([...route.querySelectorAll("[data-hue]")].map((one) => one.getAttribute("data-hue"))).toEqual(["fleet", "armada"]);
  expect(route.querySelector(".armada-lesson__arrow")?.getAttribute("aria-hidden")).toBe("true");
  const old = card(/in the dock/).element().querySelector(".armada-lesson__route")!;
  expect(old.querySelector(".armada-lesson__arrow")).toBeNull();
});

test("each answer names what it does for its item's place", async () => {
  opened([ARMADA, KIT, MANIFEST]);
  await expect.element(card(/every Rust test/)).toBeVisible();
  const buttons = (title: RegExp) => card(title).getByRole("button", { name: /^(Create Job|Accept|Reject change)$/ }).elements().map((b) => b.textContent);
  expect(buttons(/blamed the Drone/)).toEqual(["Create Job", "Reject change"]);
  expect(buttons(/grep to be allowed/)).toEqual(["Accept", "Reject change"]);
  expect(buttons(/every Rust test/)).toEqual(["Create Job", "Reject change"]);
});

test("each Agree names what it does for its item's place, and Disagree discards", async () => {
  opened([ARMADA, KIT, MANIFEST]);
  // The bubble opens after the tooltip delay, so its words are what is waited for.
  const tip = async (title: RegExp, name: string, words: string) => {
    await card(title).getByRole("button", { name, exact: true }).hover();
    await expect.element(card(title).getByText(words, { exact: true })).toBeVisible();
  };
  await tip(/blamed the Drone/, "Create Job", "Turn this into a Job that applies the change. It waits for your approval on the Board.");
  await tip(/grep to be allowed/, "Accept", "Saves it under Accepted.");
  await tip(/every Rust test/, "Create Job", "Turn this into a Job that applies the change. It waits for your approval on the Board.");
  await tip(/every Rust test/, "Reject change", "Discards it.");
});

test("an item written before the headline draws its statement as the body, with no answers, since Fleet refuses both", async () => {
  opened([OLD]);
  const one = card(/in the dock/);
  await expect.element(one.getByText(/waited in the dock/)).toBeVisible();
  expect(one.getByRole("heading").elements()).toHaveLength(0);
  expect(one.getByText("What would change", { exact: true }).elements()).toHaveLength(0);
  expect(one.getByRole("button", { name: /^(Create Job|Accept|Reject change)$/ }).elements()).toHaveLength(0);
});

test("agreeing with an Armada item leaves a link to the Job it proposed, and the link opens it", async () => {
  const { agree, job } = opened([ARMADA]);
  await card(/blamed the Drone/).getByRole("button", { name: "Create Job", exact: true }).click();
  expect(agree).toHaveBeenCalledWith("l-armada");
  const one = card(/blamed the Drone/);
  await expect.element(one.getByText("Agreed")).toBeVisible();
  expect(one.getByRole("button", { name: "Create Job", exact: true }).elements()).toHaveLength(0);
  await one.getByRole("button", { name: "Proposed Job" }).click();
  expect(job).toHaveBeenCalledWith("01K7JOB");
});

test("agreeing with a Kit item, and disagreeing with any, takes it off the list", async () => {
  const { agree, disagree } = opened([KIT, MANIFEST], [], {
    agree: async (id) => ({ ok: true, lesson: lesson({ ...KIT, id, state: "accepted" }) }),
  });
  await card(/grep to be allowed/).getByRole("button", { name: "Accept", exact: true }).click();
  expect(agree).toHaveBeenCalledWith("l-kit");
  await expect.poll(() => card(/grep to be allowed/).elements()).toHaveLength(0);
  await card(/every Rust test/).getByRole("button", { name: "Reject change", exact: true }).click();
  expect(disagree).toHaveBeenCalledWith("l-manifest");
  await expect.poll(() => card(/every Rust test/).elements()).toHaveLength(0);
});

const COMMAND = { kind: "allow_command" as const, command: "grep -n" };
const KIT_CHANGE = lesson({
  id: "l-kit-change",
  who: "drone",
  lands_in: "kit",
  statement: "A Drone was refused grep.",
  title: "A Drone was refused grep on a check log",
  what: "It asked to run grep on a check log, and was refused.",
  fix: "Add grep -n to the allowlist.",
  change: COMMAND,
});

test("a Kit item that carries a change reads Update Kit with its own tooltip, and one without keeps Accept", async () => {
  opened([KIT_CHANGE, KIT]);
  const buttons = (title: RegExp) =>
    card(title).getByRole("button", { name: /^(Create Job|Accept|Update Kit|Reject change)$/ }).elements().map((b) => b.textContent);
  await expect.element(card(/refused grep on a check log/)).toBeVisible();
  expect(buttons(/refused grep on a check log/)).toEqual(["Update Kit", "Reject change"]);
  expect(buttons(/grep to be allowed/)).toEqual(["Accept", "Reject change"]);
  await card(/refused grep on a check log/).getByRole("button", { name: "Update Kit", exact: true }).hover();
  await expect
    .element(
      card(/refused grep on a check log/).getByText(
        "Adds this command to your Kit's allowed commands. You can remove it from the Kit page.",
        { exact: true },
      ),
    )
    .toBeVisible();
});

test("pressing Update Kit sends the agree, then the card reads Updated Kit with the applied command in monospace", async () => {
  const { agree } = opened([KIT_CHANGE], [], {
    agree: async (id) => ({ ok: true, lesson: lesson({ ...KIT_CHANGE, id, state: "accepted", applied: COMMAND }) }),
  });
  await card(/refused grep on a check log/).getByRole("button", { name: "Update Kit", exact: true }).click();
  expect(agree).toHaveBeenCalledWith("l-kit-change");
  const one = card(/refused grep on a check log/);
  await expect.element(one.getByText("Updated Kit", { exact: true })).toBeVisible();
  const command = one.getByText("grep -n", { exact: true });
  await expect.element(command).toBeVisible();
  expect(command.element().classList.contains("mono")).toBe(true);
  expect(one.getByRole("button", { name: /^(Update Kit|Accept|Reject change)$/ }).elements()).toHaveLength(0);
});

test("a refused Update Kit shows the card's alert in Fleet's own words and keeps both buttons", async () => {
  const words = "rm -rf is destructive and Fleet will not run it, so it cannot be allowed.";
  opened([KIT_CHANGE], [], {
    agree: async () => ({
      ok: false,
      outcome: {
        ok: false,
        why: "refused",
        error: { code: "fleet.kit_change_refused", message: words, run_id: "r", fields: {}, chain: [] },
      },
    }),
  });
  const one = card(/refused grep on a check log/);
  await one.getByRole("button", { name: "Update Kit", exact: true }).click();
  await expect.element(one.getByText(words)).toBeVisible();
  await expect.element(one.getByText("The answer was not taken")).toBeVisible();
  expect(one.getByRole("button", { name: /^(Update Kit|Reject change)$/ }).elements()).toHaveLength(2);
});

test("Accepted reads Updated Kit and the command on an item that applied one, and nothing on one that did not", async () => {
  const applied = lesson({ ...KIT_CHANGE, id: "l-applied", state: "accepted", applied: COMMAND });
  opened([], [applied, SAVED]);
  await page.getByRole("tab", { name: "Accepted", exact: true }).click();
  await expect.element(card(/refused grep on a check log/).getByText("Updated Kit", { exact: true })).toBeVisible();
  await expect.element(card(/refused grep on a check log/).getByText("grep -n", { exact: true })).toBeVisible();
  expect(card(/allowed once already/).getByText("Updated Kit").elements()).toHaveLength(0);
});

test("a refused answer says why, and the item stays with both answers", async () => {
  opened([ARMADA], [], { agree: async () => ({ ok: false, outcome: { ok: false, why: "already_answering_lesson" } }) });
  await card(/blamed the Drone/).getByRole("button", { name: "Create Job", exact: true }).click();
  await expect.element(page.getByText(/already in flight/)).toBeVisible();
  expect(card(/blamed the Drone/).getByRole("button", { name: /^(Create Job|Accept|Reject change)$/ }).elements()).toHaveLength(2);
});

test("Open and Accepted sit beside the places, and Accepted reads the saved items with no answers on them", async () => {
  const { read } = opened([ARMADA], [SAVED]);
  await expect.element(card(/blamed the Drone/)).toBeVisible();
  expect(read).toHaveBeenLastCalledWith("open");
  await page.getByRole("tab", { name: "Accepted", exact: true }).click();
  await expect.element(card(/allowed once already/)).toBeVisible();
  expect(read).toHaveBeenLastCalledWith("accepted");
  expect(card(/blamed the Drone/).elements()).toHaveLength(0);
  expect(card(/allowed once already/).getByRole("button", { name: /^(Create Job|Accept|Reject change)$/ }).elements()).toHaveLength(0);
  await page.getByRole("tab", { name: "Open", exact: true }).click();
  await expect.element(card(/blamed the Drone/)).toBeVisible();
  const tabs = page.getByRole("tab").elements().map((t) => t.textContent);
  expect(tabs).toEqual(["All", "Armada", "Kit", "Manifest", "Open", "Accepted"]);
});

test("an empty list draws nothing", async () => {
  opened([]);
  await expect.element(page.getByRole("tab", { name: "Open", exact: true })).toBeVisible();
  expect(page.getByRole("list", { name: "Retros" }).elements()).toHaveLength(0);
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
      state: "open",
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
  await sheet.getByRole("button", { name: "Create Job", exact: true }).click();
  expect(agree).toHaveBeenCalledWith("l-armada");
  await expect.element(sheet.getByText("Agreed")).toBeVisible();
});

test("the retro sheet shows an answered item as it stands, and offers buttons only on an open one", async () => {
  const base = { who: "fleet" as const, lands_in: "armada" as const, evidence: [] };
  const retro: JobRetro = {
    job_id: "01K6JOB3",
    state: "written",
    record: {},
    items: [
      { ...base, id: "a-open", statement: "Open one", title: "Open one", what: "w", fix: "f", state: "open" },
      { ...base, id: "a-agreed", statement: "Agreed one", title: "Agreed one", what: "w", fix: "f", state: "agreed", job_proposed: "01K7JOB" },
      { ...base, id: "a-kit", lands_in: "kit" as const, statement: "Saved one", title: "Saved one", what: "w", fix: "f", state: "accepted" },
      { ...base, id: "a-no", statement: "Discarded one", title: "Discarded one", what: "w", fix: "f", state: "discarded" },
      { ...base, id: "a-old", statement: "Old one", title: "Old one", what: "w", fix: "f" },
    ],
  };
  const job = vi.fn();
  mount(
    <JobRetroSheet
      jobId="01K6JOB3"
      job="Job 3"
      read={async () => ({ ok: true, retro })}
      onAgreeLesson={async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } })}
      onDisagreeLesson={async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } })}
      onOpenJob={job}
      floor={false}
      onClose={() => {}}
    />,
  );
  const sheet = page.getByRole("dialog", { name: "Retro" });
  const one = (title: string) => sheet.getByRole("listitem").filter({ hasText: title });
  await expect.element(one("Open one")).toBeVisible();
  const answers = (title: string) =>
    one(title).getByRole("button", { name: /^(Create Job|Accept|Reject change)$/ }).elements();
  expect(answers("Open one")).toHaveLength(2);
  expect(answers("Agreed one")).toHaveLength(0);
  expect(one("Agreed one").element().querySelector(".armada-lesson__settled")?.textContent).toMatch(/^Agreed/);
  await one("Agreed one").getByRole("button", { name: "Proposed Job" }).click();
  expect(job).toHaveBeenCalledWith("01K7JOB");
  await expect.element(one("Saved one").getByText("Accepted", { exact: true })).toBeVisible();
  expect(answers("Saved one")).toHaveLength(0);
  // Discarded draws nothing, and an item with no state is read and left alone.
  expect(one("Discarded one").elements()).toHaveLength(0);
  expect(answers("Old one")).toHaveLength(0);
});

test("the retro sheet draws Update Kit on an open item with a change, and Updated Kit with its command once applied", async () => {
  const base = { who: "drone" as const, lands_in: "kit" as const, evidence: [] };
  const retro: JobRetro = {
    job_id: "01K6JOB3",
    state: "written",
    record: {},
    items: [
      { ...base, id: "k-open", statement: "s", title: "Open Kit one", what: "w", fix: "f", state: "open", change: COMMAND },
      { ...base, id: "k-done", statement: "s", title: "Done Kit one", what: "w", fix: "f", state: "accepted", change: COMMAND, applied: COMMAND },
    ],
  };
  mount(
    <JobRetroSheet
      jobId="01K6JOB3"
      job="Job 3"
      read={async () => ({ ok: true, retro })}
      onAgreeLesson={async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } })}
      onDisagreeLesson={async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } })}
      floor={false}
      onClose={() => {}}
    />,
  );
  const sheet = page.getByRole("dialog", { name: "Retro" });
  const one = (title: string) => sheet.getByRole("listitem").filter({ hasText: title });
  await expect.element(one("Open Kit one")).toBeVisible();
  await expect.element(one("Open Kit one").getByRole("button", { name: "Update Kit", exact: true })).toBeVisible();
  await expect.element(one("Done Kit one").getByText("Updated Kit", { exact: true })).toBeVisible();
  await expect.element(one("Done Kit one").getByText("grep -n", { exact: true })).toBeVisible();
});

// Evidence on the Retros list: the rows a Job's retro holds, read once per Job.

const HELD: JobRetro = {
  job_id: "01K6JOB3",
  state: "written",
  record: {
    failed_checks: [{ cite: "check:1", name: "out_of_bounds", run: "gate", produced: "armada.yml changed" }],
    refusals: [{ cite: "refusal:1", at: "2026-10-03T03:51:40.028Z", tool: "grep", tried: "grep on a check log", because: "not allowed" }],
  },
};
const WITH_CHECK = { ...ARMADA, evidence: ["check:1", "check:9"] };
const WITH_REFUSAL = { ...KIT, evidence: ["refusal:1"] };

test("list evidence: a list card offers Evidence and expands the rows it cites, in place, after one read", async () => {
  const readRetro = vi.fn(async () => ({ ok: true as const, retro: HELD }));
  opened([WITH_CHECK, OLD], [], {}, readRetro);
  const one = card(/blamed the Drone/);
  await expect.element(one).toBeVisible();
  // An item citing nothing offers none.
  expect(card(/in the dock/).getByRole("button", { name: "Evidence" }).elements()).toHaveLength(0);
  expect(readRetro).not.toHaveBeenCalled();
  await one.getByRole("button", { name: "Evidence" }).click();
  await expect.element(one.getByText("armada.yml changed")).toBeVisible();
  expect(readRetro).toHaveBeenCalledTimes(1);
  expect(readRetro).toHaveBeenCalledWith("01K6JOB3");
  // A cite the record does not hold is left out.
  expect(one.element().querySelectorAll(".armada-retro__cite")).toHaveLength(1);
  // The answers stay as they were.
  expect(one.getByRole("button", { name: /^(Create Job|Reject change)$/ }).elements()).toHaveLength(2);
});

test("list evidence: two items of one Job make one read", async () => {
  const readRetro = vi.fn(async () => ({ ok: true as const, retro: HELD }));
  opened([WITH_CHECK, WITH_REFUSAL], [], {}, readRetro);
  await card(/blamed the Drone/).getByRole("button", { name: "Evidence" }).click();
  await expect.element(card(/blamed the Drone/).getByText("armada.yml changed")).toBeVisible();
  await card(/grep to be allowed/).getByRole("button", { name: "Evidence" }).click();
  await expect.element(card(/grep to be allowed/).getByText("grep on a check log · not allowed")).toBeVisible();
  expect(readRetro).toHaveBeenCalledTimes(1);
});

test("list evidence: a failed read shows the card's alert, leaves the card intact and can be pressed again", async () => {
  let fail = true;
  const readRetro = vi.fn(async (): Promise<RetroRead> =>
    fail ? { ok: false, outcome: { ok: false, why: "not_connected" } } : { ok: true, retro: HELD },
  );
  opened([WITH_CHECK, WITH_REFUSAL], [], {}, readRetro);
  const one = card(/blamed the Drone/);
  await one.getByRole("button", { name: "Evidence" }).click();
  await expect.element(one.getByText("Retros could not be read")).toBeVisible();
  expect(card(/grep to be allowed/).getByText("Retros could not be read").elements()).toHaveLength(0);
  await expect.element(one.getByText(/two commits behind origin/)).toBeVisible();
  expect(one.getByRole("button", { name: /^(Create Job|Reject change)$/ }).elements()).toHaveLength(2);
  fail = false;
  await one.getByRole("button", { name: "Evidence" }).click();
  await expect.element(one.getByText("armada.yml changed")).toBeVisible();
  expect(one.getByText("Retros could not be read").elements()).toHaveLength(0);
});

test("list evidence: while the read is out Evidence is pending and the other buttons stay usable", async () => {
  let release: (read: RetroRead) => void = () => {};
  const readRetro = vi.fn(() => new Promise<RetroRead>((done) => (release = done)));
  const { agree } = opened([WITH_CHECK], [], {}, readRetro);
  const one = card(/blamed the Drone/);
  await one.getByRole("button", { name: "Evidence" }).click();
  await expect.element(one.getByRole("button", { name: "Evidence" })).toHaveAttribute("data-pending", "true");
  await one.getByRole("button", { name: "Create Job", exact: true }).click();
  expect(agree).toHaveBeenCalled();
  release({ ok: true, retro: HELD });
  await expect.element(one.getByText("armada.yml changed")).toBeVisible();
});

test("list evidence: when the retro holds none of the cited rows nothing is said and the control goes", async () => {
  const readRetro = vi.fn(async () => ({ ok: true as const, retro: { ...HELD, record: {} } }));
  opened([WITH_CHECK], [], {}, readRetro);
  const one = card(/blamed the Drone/);
  await one.getByRole("button", { name: "Evidence" }).click();
  await expect.poll(() => one.getByRole("button", { name: "Evidence" }).elements().length).toBe(0);
  expect(one.element().querySelectorAll(".armada-retro__cites")).toHaveLength(0);
  expect(one.getByText("Retros could not be read").elements()).toHaveLength(0);
});

test("list evidence: the window regaining focus drops what was read, and the next press reads again", async () => {
  const readRetro = vi.fn(async () => ({ ok: true as const, retro: HELD }));
  opened([WITH_CHECK], [], {}, readRetro);
  const one = card(/blamed the Drone/);
  await one.getByRole("button", { name: "Evidence" }).click();
  await expect.element(one.getByText("armada.yml changed")).toBeVisible();
  window.dispatchEvent(new Event("focus"));
  await expect.poll(() => one.element().querySelectorAll(".armada-retro__cite").length).toBe(0);
  await one.getByRole("button", { name: "Evidence" }).click();
  await expect.element(one.getByText("armada.yml changed")).toBeVisible();
  expect(readRetro).toHaveBeenCalledTimes(2);
});

test("list evidence: the failure's Retry reads again and a second success expands the rows", async () => {
  let fail = true;
  const readRetro = vi.fn(async (): Promise<RetroRead> =>
    fail ? { ok: false, outcome: { ok: false, why: "not_connected" } } : { ok: true, retro: HELD },
  );
  opened([WITH_CHECK], [], {}, readRetro);
  const one = card(/blamed the Drone/);
  await one.getByRole("button", { name: "Evidence" }).click();
  await expect.element(one.getByText("Retros could not be read")).toBeVisible();
  fail = false;
  await one.getByRole("button", { name: "Retry", exact: true }).click();
  await expect.element(one.getByText("armada.yml changed")).toBeVisible();
  expect(readRetro).toHaveBeenCalledTimes(2);
  expect(one.getByText("Retros could not be read").elements()).toHaveLength(0);
  expect(one.getByRole("button", { name: "Retry" }).elements()).toHaveLength(0);
});
