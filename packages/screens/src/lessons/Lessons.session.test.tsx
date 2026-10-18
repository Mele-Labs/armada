// Start Job (`docs/concepts/retro.md`, *Where Bridge draws it*): on the list, on
// a Session's retro sheet and in the guided review. A press asks Fleet to start the Session and opens it
// running, Fleet having sent the brief; a refusal is the card's alert, in Fleet's words, with
// the buttons kept.
//
// Browser tests, `Lessons.test.tsx`'s reason.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { Lesson, Outcome, Retro, RetroItem } from "@armada/protocol";

import { Lessons, SessionRetroSheet } from "./Lessons";
import type { LessonSessionRead } from "./session-wire";
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
  fix: "Compare against origin/main.",
});
const KIT = lesson({
  id: "l-kit",
  who: "drone",
  lands_in: "kit",
  statement: "A Drone waited on grep.",
  title: "A Drone had to wait for grep to be allowed",
  what: "It asked to run grep on a check log.",
  fix: "Add grep to the allowlist.",
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

const BRIEF = "The gate blamed the Drone for Fleet's own mistake\n\nCompare against origin/main.";
const started = (id: string): LessonSessionRead => ({
  ok: true,
  started: { lesson: { ...ARMADA, id, state: "agreed", session_proposed: "s-01J8ZQ4M" }, session_id: "s-01J8ZQ4M", brief: BRIEF },
});
const refusal: Outcome = {
  ok: false,
  why: "refused",
  error: { code: "fleet.lesson_armada_not_served", message: "This Fleet does not serve Armada.", run_id: "r", fields: {}, chain: [] },
};

const card = (title: RegExp | string) => page.getByRole("listitem").filter({ hasText: title });
const no = async () => ({ ok: false as const, outcome: { ok: false as const, why: "not_connected" as const } });

function opened(items: Lesson[], start: (id: string) => Promise<LessonSessionRead> = async (id) => started(id)) {
  const startSession = vi.fn(start);
  const openSession = vi.fn();
  mount(
    <Lessons
      onReadLessons={async () => ({ ok: true, lessons: items })}
      onReadRetro={async () => ({ ok: true, retro: { session: SESSION, state: "pending", record: {} } })}
      onAgreeLesson={no}
      onDisagreeLesson={no}
      onStartSession={startSession}
      onOpenSession={openSession}
      repository={null}
      floor={false}
    />,
  );
  return { startSession, openSession };
}

const buttonsOf = (title: RegExp) =>
  card(title)
    .getByRole("button", { name: /^(Start Job|Accept|Update Kit|Reject change)$/ })
    .elements()
    .map((one) => one.textContent);

test("Start Job sits before Reject change on an Armada or Manifest item, and not on a Kit item", async () => {
  opened([ARMADA, KIT, MANIFEST]);
  await expect.element(card(/every Rust test/)).toBeVisible();
  expect(buttonsOf(/blamed the Drone/)).toEqual(["Start Job", "Reject change"]);
  expect(buttonsOf(/every Rust test/)).toEqual(["Start Job", "Reject change"]);
  expect(buttonsOf(/grep to be allowed/)).toEqual(["Accept", "Reject change"]);
});

test("the page offers no Start Job where the host gave it no Session to open", async () => {
  mount(
    <Lessons
      onReadLessons={async () => ({ ok: true, lessons: [ARMADA] })}
      onReadRetro={async () => ({ ok: true, retro: { session: SESSION, state: "pending", record: {} } })}
      onAgreeLesson={no}
      onDisagreeLesson={no}
      repository={null}
      floor={false}
    />,
  );
  await expect.element(card(/blamed the Drone/)).toBeVisible();
  expect(page.getByRole("button", { name: "Start Job" }).elements()).toHaveLength(0);
});

test("pressing it starts the Session, opens it, and leaves the card as Job started with a link", async () => {
  const { startSession, openSession } = opened([ARMADA]);
  await page.getByRole("button", { name: "Start Job" }).click();
  expect(startSession).toHaveBeenCalledWith("l-armada");
  await vi.waitFor(() => expect(openSession).toHaveBeenCalledWith("s-01J8ZQ4M"));
  const one = card(/blamed the Drone/);
  await expect.element(one.getByText("Job started", { exact: true })).toBeVisible();
  expect(buttonsOf(/blamed the Drone/)).toEqual([]);
  await one.getByRole("button", { name: "Open Job" }).click();
  expect(openSession).toHaveBeenLastCalledWith("s-01J8ZQ4M");
});

test("a refusal is the card's alert in Fleet's words, nothing opens, and both buttons stay", async () => {
  const { openSession } = opened([ARMADA], async () => ({ ok: false, outcome: refusal }));
  await page.getByRole("button", { name: "Start Job" }).click();
  await expect.element(page.getByText("The answer was not taken")).toBeVisible();
  await expect.element(page.getByText("This Fleet does not serve Armada.")).toBeVisible();
  expect(openSession).not.toHaveBeenCalled();
  expect(buttonsOf(/blamed the Drone/)).toEqual(["Start Job", "Reject change"]);
});

test("while the Session is being started the other answer is held", async () => {
  let release: (read: LessonSessionRead) => void = () => {};
  opened([ARMADA], () => new Promise<LessonSessionRead>((done) => (release = done)));
  await page.getByRole("button", { name: "Start Job" }).click();
  await expect.element(page.getByRole("button", { name: "Reject change" })).toBeDisabled();
  release(started("l-armada"));
  await expect.element(page.getByText("Job started", { exact: true })).toBeVisible();
});

test("an item Fleet lists as agreed by a Session reads Job started, and its link opens that Session", async () => {
  const retro: Retro = {
    session: SESSION,
    state: "written",
    record: {},
    items: [
      {
        id: "a-session",
        who: "fleet",
        lands_in: "armada",
        statement: "s",
        title: "Started by a Session",
        what: "w",
        fix: "f",
        evidence: [],
        state: "agreed",
        session_proposed: "s-01J8ZQ4M",
      } satisfies RetroItem,
    ],
  };
  const openSession = vi.fn();
  mount(
    <SessionRetroSheet
      subject={{ kind: "session", id: "s-01K6Q3" }}
      session="Retire two guides"
      read={async () => ({ ok: true, retro })}
      onAgreeLesson={no}
      onDisagreeLesson={no}
      sessions={{ start: async (id) => started(id), open: openSession }}
      floor={false}
      onClose={() => {}}
    />,
  );
  const sheet = page.getByRole("dialog", { name: "Retro" });
  await expect.element(sheet.getByText("Job started", { exact: true })).toBeVisible();
  expect(sheet.getByRole("button", { name: "Start Job" }).elements()).toHaveLength(0);
  await sheet.getByRole("button", { name: "Open Job" }).click();
  expect(openSession).toHaveBeenCalledWith("s-01J8ZQ4M");
});

test("the retro sheet offers Start Job on an open Armada item and starts it", async () => {
  const retro: Retro = {
    session: SESSION,
    state: "written",
    record: {},
    items: [{ id: "l-armada", who: "fleet", lands_in: "armada", statement: "s", title: "Open one", what: "w", fix: "f", evidence: [], state: "open" }],
  };
  const openSession = vi.fn();
  mount(
    <SessionRetroSheet
      subject={{ kind: "session", id: "s-01K6Q3" }}
      session="Retire two guides"
      read={async () => ({ ok: true, retro })}
      onAgreeLesson={no}
      onDisagreeLesson={no}
      sessions={{ start: async (id) => started(id), open: openSession }}
      floor={false}
      onClose={() => {}}
    />,
  );
  const sheet = page.getByRole("dialog", { name: "Retro" });
  await sheet.getByRole("button", { name: "Start Job" }).click();
  await vi.waitFor(() => expect(openSession).toHaveBeenCalledWith("s-01J8ZQ4M"));
});

test("in the review a started Session advances like any answer", async () => {
  const startSession = vi.fn(async (id: string) => started(id));
  const openSession = vi.fn();
  mount(
    <Lessons
      onReadLessons={async () => ({ ok: true, lessons: [ARMADA, MANIFEST] })}
      onReadRetro={async () => ({ ok: true, retro: { session: SESSION, state: "pending", record: {} } })}
      onAgreeLesson={no}
      onDisagreeLesson={no}
      onStartSession={startSession}
      onOpenSession={openSession}
      onReviewLessons={async () => ({
        ok: true,
        review: {
          model: "sonnet",
          entries: [
            { lesson_id: "l-armada", merged_ids: [], reason: "First." },
            { lesson_id: "l-manifest", merged_ids: [], reason: "Second." },
          ],
          set_aside: [],
        },
      })}
      onAskLesson={async () => ({ ok: true, answer: { answer: "x" } })}
      repository={null}
      floor={false}
    />,
  );
  await page.getByRole("button", { name: "Review", exact: true }).click();
  await expect.element(page.getByText("1 of 2")).toBeVisible();
  await page.getByRole("button", { name: "Start Job" }).click();
  expect(startSession).toHaveBeenCalledWith("l-armada");
  await expect.element(page.getByText("2 of 2")).toBeVisible();
  await expect.element(page.getByRole("heading", { name: /set off every Rust test/ })).toBeVisible();
  expect(openSession).toHaveBeenCalledWith("s-01J8ZQ4M");
});

test("a Session that started but whose brief did not go is opened, and the card says why in Fleet's words", async () => {
  const { openSession } = opened([ARMADA], async (id) => {
    const ok = started(id);
    return ok.ok ? { ok: true, started: { ...ok.started, send_failed: "The agent could not be reached." } } : ok;
  });
  await page.getByRole("button", { name: "Start Job" }).click();
  await vi.waitFor(() => expect(openSession).toHaveBeenCalledWith("s-01J8ZQ4M"));
  await expect.element(page.getByText("The Job started, but the request was not sent")).toBeVisible();
  await expect.element(page.getByText("The agent could not be reached.")).toBeVisible();
  await expect.element(page.getByText("Job started", { exact: true })).toBeVisible();
});

test("the button says plainly that it begins work now", async () => {
  opened([ARMADA]);
  await expect.element(page.getByRole("button", { name: "Start Job" })).toBeVisible();
  await page.getByRole("button", { name: "Start Job" }).hover();
  await expect.element(page.getByText(/begins work at once/)).toBeVisible();
});
