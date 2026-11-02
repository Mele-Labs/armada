// Retros inside the Logbook, on the common look (`docs/concepts/retro.md`, `docs/concepts/logbook.md`): the page's State menu is
// the way to Retros and the way back, the list's strip holds Show and Review beside it, and the open item's head holds its title
// and acts. The host is the Logbook, which is what hands Retros the State menu.
//
// Browser tests, `Logbook.test.tsx`'s reason: the claims are what the screen does with an answer.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Lesson, LogbookEntry, LogbookRead } from "@armada/protocol";

import { Lessons } from "../lessons/Lessons";
import { Logbook } from "./Logbook";
import { mount, unmount } from "../mounted";

afterEach(unmount);

const AT = "2026-10-02T22:05:00.000Z";
const SESSION = { id: "s-01K6Q3", title: "Retire two guides" };

const ENTRY: LogbookEntry = {
  id: "1",
  kind: "convention",
  section: "repo",
  repository: "/Users/user/widgets",
  text: "Run specs with bin/rspec.",
  evidence: [],
  confirmed_by: 2,
  first_seen: AT,
  last_confirmed: AT,
  state: "standing",
  locked: false,
  edited: false,
  history: [],
};

const lesson = (over: Partial<Lesson> & Pick<Lesson, "id" | "statement" | "who">): Lesson => ({ handle: SESSION.id, session: SESSION, at: AT, evidence: [], state: "open", ...over });

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

/** The Logbook holding Retros, opened on it, over the open items and the accepted ones. */
function opened(open: Lesson[], saved: Lesson[] = []) {
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: true, entries: [ENTRY], tidy: { running: false }, organize: { running: false } }));
  const lessons = vi.fn(async (state: string) => ({ ok: true as const, lessons: state === "accepted" ? saved : open }));
  mount(
    <Logbook
      onRead={read}
      onEdit={vi.fn()}
      onLock={vi.fn()}
      onForget={vi.fn()}
      onOrganize={vi.fn()}
      onDismissOrganize={vi.fn()}
      onApplyOrganize={vi.fn()}
      toFixes={1}
      fixes={
        <Lessons
          embedded
          onReadLessons={lessons}
          onReadRetro={async () => ({ ok: true, retro: { session: SESSION, state: "pending", record: {} } })}
          onAgreeLesson={vi.fn()}
          onDisagreeLesson={vi.fn()}
          onStartSession={vi.fn()}
          onOpenSession={() => {}}
          onReviewLessons={vi.fn()}
          onAskLesson={vi.fn()}
          repository={null}
          floor={false}
        />
      }
    />,
  );
}

const retros = () => page.getByRole("grid", { name: "Retros" });
const item = () => page.getByRole("region", { name: "Retro item" });
const entriesGrid = () => page.getByRole("grid", { name: "Entries" });
const inMenu = async (button: string, choice: string) => {
  await page.getByRole("button", { name: button, exact: true }).click();
  await page.getByRole("menuitem", { name: choice, exact: true }).click();
};
const strip = () => page.getByRole("button", { name: "State", exact: true }).element().closest(".armada-logbook__strip")!;

test("Open and Accepted are a dropdown in the Retros list's strip, with State and Review, and not tabs", async () => {
  opened([ARMADA], [KIT]);
  await expect.element(retros()).toBeVisible();
  for (const name of ["State", "Show", "Review"]) {
    expect(strip().contains(page.getByRole("button", { name, exact: true }).element()), name).toBe(true);
  }
  expect(page.getByRole("tab").elements()).toHaveLength(0);
  await inMenu("Show", "Accepted");
  await expect.element(retros().getByRole("row", { name: /grep to be allowed/ })).toBeVisible();
  expect(retros().getByRole("row", { name: /blamed the Drone/ }).elements()).toHaveLength(0);
});

test("State reaches Retros and leaves it, from the list", async () => {
  opened([ARMADA]);
  await expect.element(retros()).toBeVisible();
  expect(entriesGrid().elements()).toHaveLength(0);
  await inMenu("State", "In use");
  await expect.element(entriesGrid()).toBeVisible();
  expect(retros().elements()).toHaveLength(0);
  await inMenu("State", "Retros");
  await expect.element(retros()).toBeVisible();
});

test("with no retro items under Open or Accepted the frames and the strip are drawn, so State is still reachable", async () => {
  opened([], []);
  await expect.element(item()).toBeVisible();
  await expect.element(page.getByRole("button", { name: "State", exact: true })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Show", exact: true })).toBeVisible();
  expect(retros().elements()).toHaveLength(0);
  await inMenu("Show", "Accepted");
  await expect.element(item()).toBeVisible();
  await inMenu("State", "In use");
  await expect.element(entriesGrid()).toBeVisible();
});

test("the open item's head holds its title and its acts, and the card keeps the rest; the acts are ghost", async () => {
  opened([ARMADA]);
  const title = item().getByRole("heading", { level: 2, name: /blamed the Drone/ });
  await expect.element(title).toBeVisible();
  const head = title.element().parentElement!;
  for (const name of ["Start Job", "Reject change"]) {
    const act = item().getByRole("button", { name, exact: true });
    expect(head.contains(act.element()), name).toBe(true);
    await expect.element(act).toHaveAttribute("data-variant", "ghost");
  }
  const card = item().getByRole("listitem");
  expect(card.getByRole("button", { name: /^(Start Job|Accept|Reject change)$/ }).elements()).toHaveLength(0);
  await expect.element(card.getByText(/two commits behind origin/)).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Review", exact: true })).toHaveAttribute("data-variant", "ghost");
});

test("s opens State and the arrows, Enter and Esc work, with the Logbook around Retros", async () => {
  opened([ARMADA, KIT]);
  await expect.element(retros()).toBeVisible();
  (retros().element() as HTMLElement).focus();
  await userEvent.keyboard("s");
  await expect.element(page.getByRole("menuitem", { name: "In use", exact: true })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  (retros().element() as HTMLElement).focus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(item().getByRole("heading", { name: /grep to be allowed/ })).toBeVisible();
  await userEvent.keyboard("{Enter}");
  await expect.element(item()).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  await expect.element(retros()).toHaveFocus();
});
