// The Logbook page as the owner works it (`docs/concepts/logbook.md`): every repository's entries in panels, three tabs by
// state, the three acts on an entry, and Fleet's refusal drawn where the act was. Entries folded in from run memory and the
// machine note carry no acts, since Fleet refuses every one.
//
// Browser tests, `Lessons.test.tsx`'s reason: the claims are what the screen does with an answer.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { LogbookAnswer, LogbookEntry, LogbookForgotten, LogbookRead, LogbookTidied, LogbookTidy } from "@armada/protocol";

import { Logbook } from "./Logbook";
import { mount, unmount } from "../mounted";

afterEach(unmount);

const AT = "2026-10-20T09:00:00.000Z";

function entry(id: string, fields: Partial<LogbookEntry> & Pick<LogbookEntry, "section" | "text">): LogbookEntry {
  return {
    id,
    kind: "convention",
    evidence: [],
    confirmed_by: 2,
    first_seen: AT,
    last_confirmed: AT,
    state: "standing",
    locked: false,
    edited: false,
    history: [],
    ...fields,
  };
}

const WIDGETS = entry("1", { section: "repo", repository: "/Users/user/widgets", text: "Run specs with bin/rspec." });
const LEDGER = entry("2", { section: "repo", repository: "/Users/user/ledger", text: "Migrations run on port 5433." });
const HABIT = entry("3", { section: "habits", kind: "habit", text: "Never run rustfmt." });
const NEW = entry("4", { section: "repo", repository: "/Users/user/widgets", text: "Seen once.", state: "candidate" });
const OLD = entry("5", { section: "habits", kind: "habit", text: "Faded words.", state: "faded" });
const RUNS = entry("runs-00000000000000ff", { section: "repo", repository: "/Users/user/widgets", text: "Run memory says use pnpm." });
const MACHINE = entry("machine-00000000000000aa", { section: "machine", kind: "environment", text: "Chrome is not installed." });

const REFUSED = {
  ok: false as const,
  outcome: { ok: false as const, why: "refused" as const, error: { code: "fleet.logbook_text_refused", message: "Text over 400 characters is refused.", run_id: "r1", fields: {}, chain: [] } },
};

type Acts = {
  edit?: (id: string, text: string) => Promise<LogbookAnswer>;
  lock?: (id: string, locked: boolean) => Promise<LogbookAnswer>;
  forget?: (id: string) => Promise<LogbookForgotten>;
  tidy?: () => Promise<LogbookTidied>;
};

function opened(entries: LogbookEntry[], acts: Acts = {}) {
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: true, entries, tidy: { running: false } }));
  const edit = vi.fn(acts.edit ?? (async (id: string, text: string): Promise<LogbookAnswer> => ({ ok: true, entry: { ...entries.find((one) => one.id === id)!, text, edited: true } })));
  const lock = vi.fn(acts.lock ?? (async (id: string, locked: boolean): Promise<LogbookAnswer> => ({ ok: true, entry: { ...entries.find((one) => one.id === id)!, locked } })));
  const forget = vi.fn(acts.forget ?? (async (): Promise<LogbookForgotten> => ({ ok: true })));
  const tidy = vi.fn(acts.tidy ?? (async (): Promise<LogbookTidied> => ({ ok: true, running: false })));
  mount(<Logbook onRead={read} onEdit={edit} onLock={lock} onForget={forget} onTidy={tidy} />);
  return { read, edit, lock, forget, tidy };
}

const entries = () => page.getByRole("listbox", { name: "Entries" });
const detail = () => page.getByRole("region", { name: "Entry" });

test("every repository, Habits and This Mac are panels of one page, read once", async () => {
  const { read } = opened([WIDGETS, LEDGER, HABIT, MACHINE]);
  await expect.element(page.getByRole("group", { name: "widgets" })).toBeVisible();
  await expect.element(page.getByRole("group", { name: "ledger" })).toBeVisible();
  await expect.element(page.getByRole("group", { name: "Habits" })).toBeVisible();
  await expect.element(page.getByRole("group", { name: "This Mac" })).toBeVisible();
  expect(read).toHaveBeenCalledTimes(1);
});

test("the tabs show the entries in one state each: In use first, then New and Archived", async () => {
  opened([WIDGETS, NEW, OLD]);
  await expect.element(entries().getByText("Run specs with bin/rspec.")).toBeVisible();
  expect(entries().getByText("Seen once.").elements()).toHaveLength(0);
  await page.getByRole("tab", { name: "New" }).click();
  await expect.element(entries().getByText("Seen once.")).toBeVisible();
  expect(entries().getByText("Run specs with bin/rspec.").elements()).toHaveLength(0);
  await page.getByRole("tab", { name: "Archived" }).click();
  await expect.element(entries().getByText("Faded words.")).toBeVisible();
});

test("Edit saves the new text to the entry's id, and Lock and Forget go to theirs", async () => {
  const { edit, lock, forget } = opened([WIDGETS, LEDGER]);
  await expect.element(detail().getByText("Run specs with bin/rspec.")).toBeVisible();
  await detail().getByRole("button", { name: /^Edit/ }).click();
  const box = detail().getByRole("textbox", { name: "Entry text" });
  await box.fill("Run specs with bin/rspec --fail-fast.");
  await detail().getByRole("button", { name: "Save" }).click();
  expect(edit).toHaveBeenCalledWith("1", "Run specs with bin/rspec --fail-fast.");

  await detail().getByRole("button", { name: /^Lock/ }).click();
  expect(lock).toHaveBeenCalledWith("1", true);

  await detail().getByRole("button", { name: /^Forget/ }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Forget", exact: true }).click();
  expect(forget).toHaveBeenCalledWith("1");
  // The next entry takes its place.
  await expect.element(detail().getByText("Migrations run on port 5433.")).toBeVisible();
});

test("e, l and f act on the open entry and Esc backs out", async () => {
  const { lock, forget } = opened([WIDGETS, LEDGER]);
  await expect.element(detail().getByText("Run specs with bin/rspec.")).toBeVisible();
  (entries().element() as HTMLElement).focus();
  await userEvent.keyboard("l");
  expect(lock).toHaveBeenCalledWith("1", true);

  await userEvent.keyboard("e");
  const box = detail().getByRole("textbox", { name: "Entry text" });
  await expect.element(box).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  expect(detail().getByRole("textbox", { name: "Entry text" }).elements()).toHaveLength(0);

  (entries().element() as HTMLElement).focus();
  await userEvent.keyboard("f");
  await expect.element(page.getByRole("dialog", { name: "Forget this entry" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  expect(forget).not.toHaveBeenCalled();
  expect(page.getByRole("dialog").elements()).toHaveLength(0);

  // Esc in the detail returns to the list.
  (detail().element() as HTMLElement).focus();
  await userEvent.keyboard("{Escape}");
  await expect.element(entries()).toHaveFocus();
});

test("an entry run memory or the machine note keeps shows no acts, and its keys do nothing", async () => {
  const { edit, lock, forget } = opened([RUNS, MACHINE]);
  await expect.element(detail().getByText("Run memory says use pnpm.")).toBeVisible();
  for (const name of [/^Edit/, /^Lock/, /^Unlock/, /^Forget/]) expect(detail().getByRole("button", { name }).elements()).toHaveLength(0);
  (entries().element() as HTMLElement).focus();
  await userEvent.keyboard("e");
  await userEvent.keyboard("l");
  await userEvent.keyboard("f");
  expect(detail().getByRole("textbox").elements()).toHaveLength(0);
  expect(page.getByRole("dialog").elements()).toHaveLength(0);
  expect(edit).not.toHaveBeenCalled();
  expect(lock).not.toHaveBeenCalled();
  expect(forget).not.toHaveBeenCalled();
  // The machine note, one entry down, has none either.
  await page.getByRole("option", { name: /Chrome is not installed/ }).click();
  await expect.element(detail().getByText("Chrome is not installed.")).toBeVisible();
  expect(detail().getByRole("button", { name: /^Forget/ }).elements()).toHaveLength(0);
});

test("Fleet's refusal is drawn on the entry in its own words, and the text stays as it was", async () => {
  opened([WIDGETS], { edit: async () => REFUSED });
  await detail().getByRole("button", { name: /^Edit/ }).click();
  await detail().getByRole("textbox", { name: "Entry text" }).fill("x".repeat(401));
  await detail().getByRole("button", { name: "Save" }).click();
  await expect.element(detail().getByText("Text over 400 characters is refused.")).toBeVisible();
  await expect.element(detail().getByText("Not saved")).toBeVisible();
  await expect.element(detail().getByText("Run specs with bin/rspec.")).toBeVisible();
});

test("a refused Forget leaves the entry and says so", async () => {
  opened([WIDGETS], { forget: async () => REFUSED });
  await detail().getByRole("button", { name: /^Forget/ }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Forget", exact: true }).click();
  await expect.element(detail().getByText("Not forgotten")).toBeVisible();
  await expect.element(entries().getByText("Run specs with bin/rspec.")).toBeVisible();
});

test("a Logbook Fleet could not read says so in place of the panels", async () => {
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: false, outcome: { ok: false, why: "not_connected" } }));
  mount(<Logbook onRead={read} onEdit={vi.fn()} onLock={vi.fn()} onForget={vi.fn()} onTidy={vi.fn()} />);
  await expect.element(page.getByText("Fleet serves no Logbook yet.")).toBeVisible();
});

const tidyButton = () => page.getByRole("button", { name: "Tidy now" });
const status = () => page.getByRole("status");

const EARLIER = { merged: 4, at: "2026-10-20T08:00:00.000Z", failed: false };
const FINISHED = "2026-10-23T09:00:00.000Z";
const SEVERAL = 3_000;

/** A Logbook whose Tidy state and entries the test moves; the poll's timer is the only fake. */
function tidying(first: LogbookTidy, entries: LogbookEntry[] = [WIDGETS]) {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
  const fleet = { tidy: first, entries };
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: true, entries: fleet.entries, tidy: fleet.tidy }));
  const onTidy = vi.fn(async (): Promise<LogbookTidied> => {
    fleet.tidy = { ...fleet.tidy, running: true };
    return { ok: true, running: true };
  });
  mount(<Logbook onRead={read} onEdit={vi.fn()} onLock={vi.fn()} onForget={vi.fn()} onTidy={onTidy} />);
  return { fleet, read, onTidy };
}

afterEach(() => vi.useRealTimers());

test("Tidy now says Tidying until the read says the pass ended, then states what it merged and reads the entries again", async () => {
  const survivor = entry("6", { section: "repo", repository: "/Users/user/widgets", text: "Survivor.", state: "standing" });
  const { fleet, read, onTidy } = tidying({ running: false, last: EARLIER }, [WIDGETS, survivor]);
  await expect.element(entries().getByText("Run specs with bin/rspec.")).toBeVisible();
  await expect.element(status()).not.toHaveTextContent("Merged");
  await tidyButton().click();
  await expect.element(tidyButton()).toHaveAttribute("aria-busy", "true");
  await expect.element(status()).toHaveTextContent("Tidying");
  await tidyButton().click({ force: true });
  expect(onTidy).toHaveBeenCalledTimes(1);
  await vi.advanceTimersByTimeAsync(SEVERAL);
  await expect.element(status()).toHaveTextContent("Tidying");
  fleet.tidy = { running: false, last: { merged: 12, at: FINISHED, failed: false } };
  fleet.entries = [survivor];
  await vi.advanceTimersByTimeAsync(SEVERAL);
  await expect.element(status()).toHaveTextContent("Merged 12 entries");
  await expect.element(tidyButton()).not.toHaveAttribute("aria-busy");
  expect(entries().getByText("Run specs with bin/rspec.").elements()).toHaveLength(0);
  const reads = read.mock.calls.length;
  await vi.advanceTimersByTimeAsync(SEVERAL * 3);
  expect(read).toHaveBeenCalledTimes(reads);
});

test.each([
  ["Nothing to merge", { merged: 0, at: FINISHED, failed: false }],
  ["Merged 1 entry", { merged: 1, at: FINISHED, failed: false }],
  ["Tidy failed after merging 3 entries", { merged: 3, at: FINISHED, failed: true }],
  ["Tidy failed after merging 1 entry", { merged: 1, at: FINISHED, failed: true }],
  ["Tidy failed before merging anything", { merged: 0, at: FINISHED, failed: true }],
])("a finished pass reads %s", async (words, last) => {
  const { fleet } = tidying({ running: false });
  await expect.element(entries().getByText("Run specs with bin/rspec.")).toBeVisible();
  await tidyButton().click();
  fleet.tidy = { running: false, last };
  await vi.advanceTimersByTimeAsync(SEVERAL);
  await expect.element(status()).toHaveTextContent(words);
});

test("a page opened while a pass runs already says Tidying, and states the result when it ends", async () => {
  const { fleet } = tidying({ running: true, last: EARLIER });
  await expect.element(tidyButton()).toHaveAttribute("aria-busy", "true");
  await expect.element(status()).toHaveTextContent("Tidying");
  fleet.tidy = { running: false, last: { merged: 2, at: FINISHED, failed: false } };
  await vi.advanceTimersByTimeAsync(SEVERAL);
  await expect.element(status()).toHaveTextContent("Merged 2 entries");
});

test("a pass that ended before the page opened is not announced", async () => {
  const { read } = tidying({ running: false, last: EARLIER });
  await expect.element(entries().getByText("Run specs with bin/rspec.")).toBeVisible();
  await vi.advanceTimersByTimeAsync(SEVERAL * 3);
  expect(read).toHaveBeenCalledTimes(1);
  await expect.element(status()).toHaveTextContent("");
});

test("a refused Tidy says why in Fleet's words and leaves the list as it was", async () => {
  const { read } = opened([WIDGETS], { tidy: async () => REFUSED });
  await tidyButton().click();
  await expect.element(page.getByRole("status")).toHaveTextContent("Text over 400 characters is refused.");
  expect(read).toHaveBeenCalledTimes(1);
});

test("an earlier text Tidy merged in is marked Merged, apart from one the person or Armada replaced", async () => {
  const history = [
    { text: "Folded in.", until: "2026-10-19T09:00:00.000Z", by: "merged" as const },
    { text: "Folded in too.", until: "2026-10-19T09:00:00.000Z", by: "merged" as const },
    { text: "Rewritten.", until: "2026-10-18T09:00:00.000Z", by: "distil" as const },
    { text: "Mine.", until: "2026-10-17T09:00:00.000Z", by: "person" as const },
  ];
  opened([entry("7", { section: "repo", repository: "/Users/user/widgets", text: "Survivor.", history })]);
  // One pass folds several in at one moment, so each is drawn once.
  await expect.element(detail().getByText("Folded in.")).toBeVisible();
  await expect.element(detail().getByText("Folded in too.")).toBeVisible();
  expect(detail().getByText(/^Merged/).elements()).toHaveLength(2);
  await expect.element(detail().getByText(/^Rewritten by Armada/)).toBeVisible();
  await expect.element(detail().getByText(/^Replaced by you/)).toBeVisible();
});
