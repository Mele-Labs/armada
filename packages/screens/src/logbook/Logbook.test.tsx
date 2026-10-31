// The Logbook page as the owner works it (`docs/concepts/logbook.md`): every repository's entries in panels, three tabs by
// state, the three acts on an entry, and Fleet's refusal drawn where the act was. Entries folded in from run memory and the
// machine note carry no acts, since Fleet refuses every one.
//
// Browser tests, `Lessons.test.tsx`'s reason: the claims are what the screen does with an answer.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { LogbookAnswer, LogbookEntry, LogbookForgotten, LogbookOrganizeApplied, LogbookOrganized, LogbookRead, LogbookTidy, OrganizeProposal } from "@armada/protocol";

import { Logbook } from "./Logbook";
import { forgetTidyWatch } from "./tidy-watch";
import { mount, rerender, unmount } from "../mounted";

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

type Logbook_Organize = (action: "merge" | "stale" | "bugs") => Promise<LogbookOrganized>;
type Logbook_Apply = (action: "merge" | "stale" | "bugs", proposals: OrganizeProposal[]) => Promise<LogbookOrganizeApplied>;

const NONE = async (): Promise<LogbookOrganized> => ({ ok: true, action: "merge", proposals: [], failed: false });
const NOTHING = async (): Promise<LogbookOrganizeApplied> => ({ ok: true, action: "merge", applied: 0, reported: [] });

type Acts = {
  edit?: (id: string, text: string) => Promise<LogbookAnswer>;
  lock?: (id: string, locked: boolean) => Promise<LogbookAnswer>;
  forget?: (id: string) => Promise<LogbookForgotten>;
  organize?: Logbook_Organize;
  apply?: Logbook_Apply;
};

function opened(entries: LogbookEntry[], acts: Acts = {}) {
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: true, entries, tidy: { running: false } }));
  const edit = vi.fn(acts.edit ?? (async (id: string, text: string): Promise<LogbookAnswer> => ({ ok: true, entry: { ...entries.find((one) => one.id === id)!, text, edited: true } })));
  const lock = vi.fn(acts.lock ?? (async (id: string, locked: boolean): Promise<LogbookAnswer> => ({ ok: true, entry: { ...entries.find((one) => one.id === id)!, locked } })));
  const forget = vi.fn(acts.forget ?? (async (): Promise<LogbookForgotten> => ({ ok: true })));
  const organize = vi.fn(acts.organize ?? NONE);
  const apply = vi.fn(acts.apply ?? NOTHING);
  mount(<Logbook onRead={read} onEdit={edit} onLock={lock} onForget={forget} onOrganize={organize} onApplyOrganize={apply} />);
  return { read, edit, lock, forget, organize, apply };
}

/** Picks a state from the State menu. */
const inState = async (name: string) => {
  await page.getByRole("button", { name: "State" }).click();
  await page.getByRole("menuitem", { name }).click();
};

const entries = () => page.getByRole("grid", { name: "Entries" });
/** An entry's line in the log, by the words its title starts with. */
const line = (name: RegExp) => entries().getByRole("row", { name });
const detail = () => page.getByRole("region", { name: "Entry" });

test("every repository, Habits and This Mac are panels of one page, read once", async () => {
  const { read } = opened([WIDGETS, LEDGER, HABIT, MACHINE]);
  await expect.element(page.getByRole("rowgroup", { name: "widgets" })).toBeVisible();
  await expect.element(page.getByRole("rowgroup", { name: "ledger" })).toBeVisible();
  await expect.element(page.getByRole("rowgroup", { name: "Habits" })).toBeVisible();
  await expect.element(page.getByRole("rowgroup", { name: "This Mac" })).toBeVisible();
  expect(read).toHaveBeenCalledTimes(1);
});

test("the tabs show the entries in one state each: In use first, then New and Archived", async () => {
  opened([WIDGETS, NEW, OLD]);
  await expect.element(line(/Run specs with bin\/rspec/)).toBeVisible();
  expect(line(/Seen once/).elements()).toHaveLength(0);
  await inState("New");
  await expect.element(line(/Seen once/)).toBeVisible();
  expect(line(/Run specs with bin\/rspec/).elements()).toHaveLength(0);
  await inState("Archived");
  await expect.element(line(/Faded words/)).toBeVisible();
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
  await line(/Chrome is not installed/).click();
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
  await expect.element(line(/Run specs with bin\/rspec/)).toBeVisible();
});

test("a Logbook Fleet could not read says so in place of the panels", async () => {
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: false, outcome: { ok: false, why: "not_connected" } }));
  mount(<Logbook onRead={read} onEdit={vi.fn()} onLock={vi.fn()} onForget={vi.fn()} onOrganize={NONE} onApplyOrganize={NOTHING} />);
  await expect.element(page.getByText("Fleet serves no Logbook yet.")).toBeVisible();
});

const status = () => page.getByRole("status");

const EARLIER = { merged: 4, at: "2026-10-20T08:00:00.000Z", failed: false };
const FINISHED = "2026-10-23T09:00:00.000Z";
const SEVERAL = 3_000;

/** A Logbook whose Tidy state the test moves, as the weekly pass moves it; the poll's timer is the only fake. */
function tidying(first: LogbookTidy, entries: LogbookEntry[] = [WIDGETS]) {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
  const fleet = { tidy: first, entries };
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: true, entries: fleet.entries, tidy: fleet.tidy }));
  const open = () => mount(<Logbook onRead={read} onEdit={vi.fn()} onLock={vi.fn()} onForget={vi.fn()} onOrganize={NONE} onApplyOrganize={NOTHING} />);
  open();
  return { fleet, read, reopen: () => (unmount(), open()) };
}

afterEach(() => {
  vi.useRealTimers();
  forgetTidyWatch();
});

test.each([
  ["Nothing to merge", { merged: 0, at: FINISHED, failed: false }],
  ["Merged 1 entry", { merged: 1, at: FINISHED, failed: false }],
  ["Tidy failed after merging 3 entries", { merged: 3, at: FINISHED, failed: true }],
  ["Tidy failed before merging anything", { merged: 0, at: FINISHED, failed: true }],
])("a finished pass reads %s", async (words, last) => {
  const { fleet } = tidying({ running: true, last: EARLIER });
  await expect.element(status()).toHaveTextContent("Tidying");
  fleet.tidy = { running: false, last };
  await vi.advanceTimersByTimeAsync(SEVERAL);
  await expect.element(status()).toHaveTextContent(words);
});

test("a page opened while a pass runs says Tidying and polls until it ends", async () => {
  const { fleet, read } = tidying({ running: true, last: EARLIER });
  await expect.element(status()).toHaveTextContent("Tidying");
  const reads = read.mock.calls.length;
  await vi.advanceTimersByTimeAsync(SEVERAL);
  expect(read.mock.calls.length).toBeGreaterThan(reads);
  fleet.tidy = { running: false, last: { merged: 2, at: FINISHED, failed: false } };
  await vi.advanceTimersByTimeAsync(SEVERAL);
  await expect.element(status()).toHaveTextContent("Merged 2 entries");
});

test("a pass that ended while the page was closed is stated once when it is opened again", async () => {
  const { fleet, reopen } = tidying({ running: true, last: EARLIER });
  await expect.element(status()).toHaveTextContent("Tidying");
  fleet.tidy = { running: false, last: { merged: 5, at: FINISHED, failed: false } };
  reopen();
  await expect.element(status()).toHaveTextContent("Merged 5 entries");
  reopen();
  await expect.element(line(/Run specs with bin\/rspec/)).toBeVisible();
  await expect.element(status()).toHaveTextContent("");
});

test("a pass that ended before the page opened is not announced", async () => {
  const { read } = tidying({ running: false, last: EARLIER });
  await expect.element(line(/Run specs with bin\/rspec/)).toBeVisible();
  await vi.advanceTimersByTimeAsync(SEVERAL * 3);
  expect(read).toHaveBeenCalledTimes(1);
  await expect.element(status()).toHaveTextContent("");
});

const organizeMenu = () => page.getByRole("button", { name: "Organize" });
const choose = async (name: string) => {
  await organizeMenu().click();
  await page.getByRole("menuitem", { name }).click();
};

const DUPLICATE = entry("10", { section: "repo", repository: "/Users/user/widgets", text: "Run the specs with bin/rspec." });

test("Organize has no Tidy button, and offers its three looks", async () => {
  opened([WIDGETS]);
  await expect.element(organizeMenu()).toBeVisible();
  expect(page.getByRole("button", { name: "Tidy now" }).elements()).toHaveLength(0);
  await organizeMenu().click();
  await expect.element(page.getByRole("menuitem", { name: "Merge duplicates" })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: "Find stale entries" })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: "Find Armada bugs" })).toBeVisible();
});

test("Merge duplicates asks for the merge action and shows the kept entry with the ones it replaces", async () => {
  const { organize } = opened([WIDGETS, DUPLICATE], {
    organize: async () => ({ ok: true, action: "merge", proposals: [{ id: "1", entry_ids: ["1", "10"], text: "Run specs with bin/rspec.", note: "" }], failed: false }),
  });
  await choose("Merge duplicates");
  await expect.element(page.getByText("Keeps")).toBeVisible();
  await expect.element(page.getByText("Replaces")).toBeVisible();
  await expect.element(page.getByRole("list", { name: "Proposals" }).getByText("Run the specs with bin/rspec.")).toBeVisible();
  expect(organize).toHaveBeenCalledWith("merge");
});

test("Apply sends the proposals back as Organize answered them, then reads the entries again and states the count", async () => {
  const proposal = { id: "1", entry_ids: ["1", "10"], text: "Run specs with bin/rspec.", note: "" };
  const { apply, read } = opened([WIDGETS, DUPLICATE], {
    organize: async () => ({ ok: true, action: "merge", proposals: [proposal], failed: false }),
    apply: async () => ({ ok: true, action: "merge", applied: 1, reported: [] }),
  });
  await choose("Merge duplicates");
  await page.getByRole("button", { name: "Apply all" }).click();
  await expect.element(status()).toHaveTextContent("Merged 1 entry");
  expect(apply).toHaveBeenCalledWith("merge", [proposal]);
  expect(read).toHaveBeenCalledTimes(2);
});

test("a failed look says Failed in the status line, beside the proposals that did arrive", async () => {
  opened([WIDGETS], {
    organize: async () => ({ ok: true, action: "stale", proposals: [{ id: "1", entry_ids: ["1"], text: WIDGETS.text, note: "Not confirmed in 60 days" }], failed: true }),
  });
  await choose("Find stale entries");
  await expect.element(status()).toHaveTextContent("Failed");
  await expect.element(page.getByText("Not confirmed in 60 days")).toBeVisible();
});

test("a look with nothing to propose says Nothing found", async () => {
  opened([WIDGETS]);
  await choose("Find stale entries");
  await expect.element(status()).toHaveTextContent("Nothing found");
});

const GIT = entry("11", { section: "repo", repository: "/Users/user/widgets", text: "Rebase before pushing.", topic: "git" });

test("entries group under their topic, and the Topic menu narrows the page to one", async () => {
  opened([WIDGETS, GIT]);
  await expect.element(line(/Rebase before pushing/)).toBeVisible();
  await expect.element(line(/Run specs with bin\/rspec/)).toBeVisible();
  await page.getByRole("button", { name: "Topic" }).click();
  await page.getByRole("menuitem", { name: "Git" }).click();
  await expect.element(line(/Rebase before pushing/)).toBeVisible();
  expect(line(/Run specs with bin\/rspec/).elements()).toHaveLength(0);
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

test("an entry's line reads its title, or the first clause of its rule where it has none", async () => {
  opened([
    entry("8", { section: "repo", repository: "/Users/user/widgets", title: "Specs through bin/rspec", text: "Run specs with bin/rspec, never rake." }),
    entry("9", { section: "repo", repository: "/Users/user/widgets", text: "Pass `--merge` (the queue refuses a squash): a bare merge is stopped. Always." }),
  ]);
  await expect.element(line(/^.*Specs through bin\/rspec/)).toBeVisible();
  expect(line(/never rake/).elements()).toHaveLength(0);
  // No title: the words up to the first stop, without the quotes or the aside.
  await expect.element(entries().getByText("Pass --merge", { exact: true })).toBeVisible();
  // The rule itself is the open entry's, whole.
  await expect.element(detail().getByText(/never rake/)).toBeVisible();
});

test("the retro items are a tab of their own, drawn in place of the entries", async () => {
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: true, entries: [WIDGETS], tidy: { running: false } }));
  mount(<Logbook onRead={read} onEdit={vi.fn()} onLock={vi.fn()} onForget={vi.fn()} onOrganize={NONE} onApplyOrganize={NOTHING} fixes={<p>The retro items</p>} />);
  await expect.element(line(/Run specs with bin\/rspec/)).toBeVisible();
  expect(page.getByText("The retro items").elements()).toHaveLength(0);
  await inState("Retros");
  await expect.element(page.getByRole("region", { name: "Retros" }).getByText("The retro items")).toBeVisible();
  expect(page.getByRole("grid", { name: "Entries" }).elements()).toHaveLength(0);
  await inState("In use");
  await expect.element(line(/Run specs with bin\/rspec/)).toBeVisible();
  expect(page.getByText("The retro items").elements()).toHaveLength(0);
});

test("asked for the retro items, the Logbook opens on Retros, and turns to it again when asked while open", async () => {
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: true, entries: [WIDGETS], tidy: { running: false } }));
  const at = (toFixes: number) => <Logbook onRead={read} onEdit={vi.fn()} onLock={vi.fn()} onForget={vi.fn()} onOrganize={NONE} onApplyOrganize={NOTHING} fixes={<p>The retro items</p>} toFixes={toFixes} />;
  mount(at(1));
  await expect.element(page.getByText("The retro items")).toBeVisible();
  await inState("In use");
  await expect.element(line(/Run specs with bin\/rspec/)).toBeVisible();
  rerender(at(2));
  await expect.element(page.getByText("The retro items")).toBeVisible();
});
