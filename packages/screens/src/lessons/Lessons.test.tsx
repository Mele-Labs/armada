// The Lessons page and a Session's retro sheet, as the owner reads and answers them
// (`docs/concepts/retro.md`): each item as words, a headline, what happened and
// a fix, and Agree and Disagree under it.
//
// Browser tests, `Worktrees.test.tsx`'s reason: the claims are what a surface
// does with an answer, and a story cannot mount a screen.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { Lesson, LessonAnswer, LessonState, Retro, RetroRead, RetroSubject } from "@armada/protocol";

import { Lessons, SessionRetroSheet } from "./Lessons";
import { mount, unmount } from "../mounted";

afterEach(unmount);

const AT = "2026-10-02T22:05:00.000Z";

const SESSION = { id: "s-01K6Q3", title: "Retire two guides" };

function lesson(over: Partial<Lesson> & Pick<Lesson, "id" | "statement" | "who">): Lesson {
  return { handle: "s-01K6Q3", session: SESSION, at: AT, evidence: [], state: "open", ...over };
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
  readRetro: (subject: RetroSubject) => Promise<RetroRead> = async () => ({ ok: true, retro: { session: SESSION, state: "pending", record: {} } }),
) {
  const read = vi.fn(async (state: LessonState | "open" | "accepted") => ({
    ok: true as const,
    lessons: state === "accepted" ? saved : open,
  }));
  const agree = vi.fn(acts.agree ?? (async (id: string) => ({ ok: true as const, lesson: lesson({ ...KIT, id, state: "accepted" }) })));
  const disagree = vi.fn(acts.disagree ?? (async (id: string) => ({ ok: true as const, lesson: lesson({ ...ARMADA, id, state: "discarded" }) })));
  const start = vi.fn(async (id: string) => ({
    ok: true as const,
    started: { lesson: lesson({ ...ARMADA, id, state: "agreed", session_proposed: "s-NEW" }), session_id: "s-NEW", brief: "b" },
  }));
  const openSession = vi.fn();
  mount(
    <Lessons
      onReadLessons={read}
      onReadRetro={readRetro}
      onAgreeLesson={agree}
      onDisagreeLesson={disagree}
      onStartSession={start}
      onOpenSession={openSession}
      repository={null}
      floor={false}
    />,
  );
  return { read, agree, disagree, start, open: openSession };
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
  expect(one.getByRole("button", { name: /^(Start Job|Accept|Reject change)$/ }).elements().map((b) => b.textContent)).toEqual(["Start Job", "Reject change"]);
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
  const buttons = (title: RegExp) => card(title).getByRole("button", { name: /^(Start Job|Accept|Reject change)$/ }).elements().map((b) => b.textContent);
  expect(buttons(/blamed the Drone/)).toEqual(["Start Job", "Reject change"]);
  expect(buttons(/grep to be allowed/)).toEqual(["Accept", "Reject change"]);
  expect(buttons(/every Rust test/)).toEqual(["Start Job", "Reject change"]);
});

test("each Agree names what it does for its item's place, and Disagree discards", async () => {
  opened([ARMADA, KIT, MANIFEST]);
  // The bubble opens after the tooltip delay, so its words are what is waited for.
  const tip = async (title: RegExp, name: string, words: string) => {
    await card(title).getByRole("button", { name, exact: true }).hover();
    await expect.element(card(title).getByText(words, { exact: true })).toBeVisible();
  };
  await tip(/blamed the Drone/, "Start Job", "Starts a Job on this repository and sends it this request now. It begins work at once.");
  await tip(/grep to be allowed/, "Accept", "Saves it under Accepted.");
  await tip(/every Rust test/, "Start Job", "Starts a Job on this repository and sends it this request now. It begins work at once.");
  await tip(/every Rust test/, "Reject change", "Discards it.");
});

test("an item written before the headline draws its statement as the body, with no answers, since Fleet refuses both", async () => {
  opened([OLD]);
  const one = card(/in the dock/);
  await expect.element(one.getByText(/waited in the dock/)).toBeVisible();
  expect(one.getByRole("heading").elements()).toHaveLength(0);
  expect(one.getByText("What would change", { exact: true }).elements()).toHaveLength(0);
  expect(one.getByRole("button", { name: /^(Start Job|Accept|Reject change)$/ }).elements()).toHaveLength(0);
});

test("starting a Session on an Armada item opens it, and leaves a link that opens it again", async () => {
  const { start, open } = opened([ARMADA]);
  await card(/blamed the Drone/).getByRole("button", { name: "Start Job", exact: true }).click();
  expect(start).toHaveBeenCalledWith("l-armada");
  const one = card(/blamed the Drone/);
  await expect.element(one.getByText("Job started")).toBeVisible();
  expect(open).toHaveBeenCalledWith("s-NEW");
  expect(one.getByRole("button", { name: "Start Job", exact: true }).elements()).toHaveLength(0);
  await one.getByRole("button", { name: "Open Job" }).click();
  expect(open).toHaveBeenCalledTimes(2);
});

test("agreeing with a Kit item, and disagreeing with any, takes it off the list", async () => {
  const { agree, disagree } = opened([KIT, MANIFEST]);
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
    card(title).getByRole("button", { name: /^(Start Job|Accept|Update Kit|Reject change)$/ }).elements().map((b) => b.textContent);
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
  opened([KIT], [], { agree: async () => ({ ok: false, outcome: { ok: false, why: "already_answering_lesson" } }) });
  await card(/grep to be allowed/).getByRole("button", { name: "Accept", exact: true }).click();
  await expect.element(page.getByText(/already in flight/)).toBeVisible();
  expect(card(/grep to be allowed/).getByRole("button", { name: /^(Start Job|Accept|Reject change)$/ }).elements()).toHaveLength(2);
});

test("Open and Accepted sit beside the places, and Accepted reads the saved items with no answers on them", async () => {
  const { read } = opened([ARMADA], [SAVED]);
  await expect.element(card(/blamed the Drone/)).toBeVisible();
  expect(read).toHaveBeenLastCalledWith("open");
  await page.getByRole("tab", { name: "Accepted", exact: true }).click();
  await expect.element(card(/allowed once already/)).toBeVisible();
  expect(read).toHaveBeenLastCalledWith("accepted");
  expect(card(/blamed the Drone/).elements()).toHaveLength(0);
  expect(card(/allowed once already/).getByRole("button", { name: /^(Start Job|Accept|Reject change)$/ }).elements()).toHaveLength(0);
  await page.getByRole("tab", { name: "Open", exact: true }).click();
  await expect.element(card(/blamed the Drone/)).toBeVisible();
  const tabs = page.getByRole("tab").elements().map((t) => t.textContent);
  expect(tabs).toEqual(["All", "Armada", "Kit", "Manifest", "Open", "Accepted"]);
});

test("opening an item from an older Session retro reads that retro, and an unnumbered item reads the newest", async () => {
  const session = { id: "s-01J8ZQ4M", title: "Fix the flaky store test" };
  const of = (id: string, title: string) => lesson({ ...KIT, id, handle: session.id, session, title });
  const readRetro = vi.fn(async () => ({ ok: true as const, retro: RETRO }));
  opened([of("s-01J8ZQ4M-r2-1", "A newer pain"), of("s-01J8ZQ4M-r1-1", "An older pain"), ARMADA], [], {}, readRetro);
  await expect.element(card(/An older pain/)).toBeVisible();
  await card(/An older pain/).getByRole("button", { name: /Fix the flaky store test/ }).click();
  await expect.element(page.getByRole("dialog", { name: "Retro" })).toBeVisible();
  expect(readRetro).toHaveBeenLastCalledWith({ kind: "session", id: "s-01J8ZQ4M", n: 1 });
  await page.getByRole("dialog", { name: "Retro" }).getByRole("button", { name: "Close" }).click();
  await card(/blamed the Drone/).getByRole("button", { name: /Retire two guides/ }).click();
  await expect.poll(() => readRetro.mock.lastCall).toEqual([{ kind: "session", id: "s-01K6Q3" }]);
});

test("an empty list draws nothing", async () => {
  opened([]);
  await expect.element(page.getByRole("tab", { name: "Open", exact: true })).toBeVisible();
  expect(page.getByRole("list", { name: "Retros" }).elements()).toHaveLength(0);
});

const RETRO: Retro = {
  session: SESSION,
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
    failed_tools: [{ cite: "check:1", at: AT, tool: "gate", tried: "armada.yml changed" }],
  },
};

test("the retro sheet draws the same item, keeps its evidence behind a control, and answers the same way", async () => {
  const start = vi.fn(async (id: string) => ({
    ok: true as const,
    started: { lesson: lesson({ ...ARMADA, id, state: "agreed", session_proposed: "s-NEW" }), session_id: "s-NEW", brief: "b" },
  }));
  mount(
    <SessionRetroSheet
      subject={{ kind: "session", id: "s-01K6Q3" }}
      session="Retire two guides"
      read={async () => ({ ok: true, retro: RETRO })}
      onAgreeLesson={async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } })}
      onDisagreeLesson={async () => ({ ok: true, lesson: lesson({ ...ARMADA, state: "discarded" }) })}
      sessions={{ start, open: () => {} }}
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
  await sheet.getByRole("button", { name: "Start Job", exact: true }).click();
  expect(start).toHaveBeenCalledWith("l-armada");
  await expect.element(sheet.getByText("Job started")).toBeVisible();
});

test("the retro sheet shows an answered item as it stands, and offers buttons only on an open one", async () => {
  const base = { who: "fleet" as const, lands_in: "armada" as const, evidence: [] };
  const retro: Retro = {
    session: SESSION,
    state: "written",
    record: {},
    items: [
      { ...base, id: "a-open", statement: "Open one", title: "Open one", what: "w", fix: "f", state: "open" },
      { ...base, id: "a-agreed", statement: "Agreed one", title: "Agreed one", what: "w", fix: "f", state: "agreed", session_proposed: "s-NEW" },
      { ...base, id: "a-kit", lands_in: "kit" as const, statement: "Saved one", title: "Saved one", what: "w", fix: "f", state: "accepted" },
      { ...base, id: "a-no", statement: "Discarded one", title: "Discarded one", what: "w", fix: "f", state: "discarded" },
      { ...base, id: "a-old", statement: "Old one", title: "Old one", what: "w", fix: "f" },
    ],
  };
  const open = vi.fn();
  mount(
    <SessionRetroSheet
      subject={{ kind: "session", id: "s-01K6Q3" }}
      session="Retire two guides"
      read={async () => ({ ok: true, retro })}
      onAgreeLesson={async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } })}
      onDisagreeLesson={async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } })}
      sessions={{ start: async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }), open }}
      floor={false}
      onClose={() => {}}
    />,
  );
  const sheet = page.getByRole("dialog", { name: "Retro" });
  const one = (title: string) => sheet.getByRole("listitem").filter({ hasText: title });
  await expect.element(one("Open one")).toBeVisible();
  const answers = (title: string) =>
    one(title).getByRole("button", { name: /^(Start Job|Accept|Reject change)$/ }).elements();
  expect(answers("Open one")).toHaveLength(2);
  expect(answers("Agreed one")).toHaveLength(0);
  expect(one("Agreed one").element().querySelector(".armada-lesson__settled")?.textContent).toMatch(/^Job started/);
  await one("Agreed one").getByRole("button", { name: "Open Job" }).click();
  expect(open).toHaveBeenCalledWith("s-NEW");
  await expect.element(one("Saved one").getByText("Accepted", { exact: true })).toBeVisible();
  expect(answers("Saved one")).toHaveLength(0);
  // Discarded draws nothing, and an item with no state is read and left alone.
  expect(one("Discarded one").elements()).toHaveLength(0);
  expect(answers("Old one")).toHaveLength(0);
});

test("the retro sheet draws Update Kit on an open item with a change, and Updated Kit with its command once applied", async () => {
  const base = { who: "drone" as const, lands_in: "kit" as const, evidence: [] };
  const retro: Retro = {
    session: SESSION,
    state: "written",
    record: {},
    items: [
      { ...base, id: "k-open", statement: "s", title: "Open Kit one", what: "w", fix: "f", state: "open", change: COMMAND },
      { ...base, id: "k-done", statement: "s", title: "Done Kit one", what: "w", fix: "f", state: "accepted", change: COMMAND, applied: COMMAND },
    ],
  };
  mount(
    <SessionRetroSheet
      subject={{ kind: "session", id: "s-01K6Q3" }}
      session="Retire two guides"
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

// Evidence on the Retros list: the rows a Session's retro holds, read once per retro.

const HELD: Retro = {
  session: SESSION,
  state: "written",
  record: {
    failed_tools: [{ cite: "check:1", at: AT, tool: "gate", tried: "armada.yml changed" }],
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
  expect(readRetro).toHaveBeenCalledWith({ kind: "session", id: "s-01K6Q3" });
  // A cite the record does not hold is left out.
  expect(one.element().querySelectorAll(".armada-retro__cite")).toHaveLength(1);
  // The answers stay as they were.
  expect(one.getByRole("button", { name: /^(Start Job|Reject change)$/ }).elements()).toHaveLength(2);
});

test("list evidence: two items of one retro make one read", async () => {
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
  expect(one.getByRole("button", { name: /^(Start Job|Reject change)$/ }).elements()).toHaveLength(2);
  fail = false;
  await one.getByRole("button", { name: "Evidence" }).click();
  await expect.element(one.getByText("armada.yml changed")).toBeVisible();
  expect(one.getByText("Retros could not be read").elements()).toHaveLength(0);
});

test("list evidence: while the read is out Evidence is pending and the other buttons stay usable", async () => {
  let release: (read: RetroRead) => void = () => {};
  const readRetro = vi.fn(() => new Promise<RetroRead>((done) => (release = done)));
  const { start } = opened([WITH_CHECK], [], {}, readRetro);
  const one = card(/blamed the Drone/);
  await one.getByRole("button", { name: "Evidence" }).click();
  await expect.element(one.getByRole("button", { name: "Evidence" })).toHaveAttribute("data-pending", "true");
  await one.getByRole("button", { name: "Start Job", exact: true }).click();
  expect(start).toHaveBeenCalled();
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
