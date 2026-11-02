// The Logbook page as the owner works it (`docs/concepts/logbook.md`): every repository's entries in panels, three tabs by
// state, the three acts on an entry, and Fleet's refusal drawn where the act was. Entries folded in from run memory and the
// machine note carry no acts, since Fleet refuses every one.
//
// Browser tests, `Lessons.test.tsx`'s reason: the claims are what the screen does with an answer.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { LogbookAnswer, LogbookEntry, LogbookForgotten, LogbookOrganize, LogbookOrganizeApplied, LogbookOrganizeDismissed, LogbookOrganizing, LogbookRead, LogbookTidy, OrganizeProposal } from "@armada/protocol";

import { useStateMenu } from "@armada/components";

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

type Logbook_Organize = (action: "merge" | "stale" | "bugs") => Promise<LogbookOrganizing>;
type Logbook_Apply = (action: "merge" | "stale" | "bugs", proposals: OrganizeProposal[]) => Promise<LogbookOrganizeApplied>;

const NONE = async (): Promise<LogbookOrganizing> => ({ ok: true, running: true });
const DISMISSED = async (): Promise<LogbookOrganizeDismissed> => ({ ok: true });
const NOTHING = async (): Promise<LogbookOrganizeApplied> => ({ ok: true, action: "merge", applied: 0, reported: [] });

type Acts = {
  edit?: (id: string, text: string) => Promise<LogbookAnswer>;
  lock?: (id: string, locked: boolean) => Promise<LogbookAnswer>;
  forget?: (id: string) => Promise<LogbookForgotten>;
  organize?: Logbook_Organize;
  apply?: Logbook_Apply;
  /** What Fleet holds of Organize when the page opens. */
  held?: LogbookOrganize;
};

function opened(entries: LogbookEntry[], acts: Acts = {}) {
  // Organize as Fleet holds it; a test moves it as the background look does.
  const fleet = { organize: acts.held ?? { running: false } };
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: true, entries, tidy: { running: false }, organize: fleet.organize }));
  const edit = vi.fn(acts.edit ?? (async (id: string, text: string): Promise<LogbookAnswer> => ({ ok: true, entry: { ...entries.find((one) => one.id === id)!, text, edited: true } })));
  const lock = vi.fn(acts.lock ?? (async (id: string, locked: boolean): Promise<LogbookAnswer> => ({ ok: true, entry: { ...entries.find((one) => one.id === id)!, locked } })));
  const forget = vi.fn(acts.forget ?? (async (): Promise<LogbookForgotten> => ({ ok: true })));
  const organize = vi.fn(acts.organize ?? NONE);
  const apply = vi.fn(acts.apply ?? NOTHING);
  const dismiss = vi.fn(DISMISSED);
  mount(<Logbook onRead={read} onEdit={edit} onLock={lock} onForget={forget} onOrganize={organize} onDismissOrganize={dismiss} onApplyOrganize={apply} />);
  return { read, edit, lock, forget, organize, apply, dismiss, fleet };
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
  mount(<Logbook onRead={read} onEdit={vi.fn()} onLock={vi.fn()} onForget={vi.fn()} onOrganize={NONE} onDismissOrganize={DISMISSED} onApplyOrganize={NOTHING} />);
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
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: true, entries: fleet.entries, tidy: fleet.tidy, organize: { running: false } }));
  const open = () => mount(<Logbook onRead={read} onEdit={vi.fn()} onLock={vi.fn()} onForget={vi.fn()} onOrganize={NONE} onDismissOrganize={DISMISSED} onApplyOrganize={NOTHING} />);
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

const MERGE = { id: "1", entry_ids: ["1", "10"], text: "Run specs with bin/rspec.", note: "" };
const HELD_MERGE: LogbookOrganize = { running: false, last: { action: "merge", proposals: [MERGE], failed: false } };

test("Merge duplicates starts the look and waits on it, then shows the kept entry with the ones it replaces when Fleet holds them", async () => {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
  const { organize, fleet, read } = opened([WIDGETS, DUPLICATE]);
  await choose("Merge duplicates");
  expect(organize).toHaveBeenCalledWith("merge");
  await expect.element(status()).toHaveTextContent("Finding duplicates");
  expect(page.getByRole("list", { name: "Proposals" }).elements()).toHaveLength(0);
  fleet.organize = HELD_MERGE;
  await vi.advanceTimersByTimeAsync(SEVERAL);
  expect(read).toHaveBeenCalledTimes(2);
  await expect.element(page.getByText("Keeps")).toBeVisible();
  await expect.element(page.getByText("Replaces")).toBeVisible();
  await expect.element(page.getByRole("list", { name: "Proposals" }).getByText("Run the specs with bin/rspec.")).toBeVisible();
});

test("a look still running when the page opens shows the running status and keeps Organize closed", async () => {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
  const { fleet } = opened([WIDGETS, DUPLICATE], { held: { running: true } });
  await expect.element(status()).toHaveTextContent("Organizing");
  await expect.element(organizeMenu()).toBeDisabled();
  fleet.organize = HELD_MERGE;
  await vi.advanceTimersByTimeAsync(SEVERAL);
  await expect.element(page.getByRole("list", { name: "Proposals" }).getByText("Run the specs with bin/rspec.")).toBeVisible();
  await expect.element(organizeMenu()).toBeEnabled();
});

test("proposals Fleet holds from an earlier look are drawn when the page opens", async () => {
  opened([WIDGETS, DUPLICATE], { held: HELD_MERGE });
  await expect.element(page.getByRole("list", { name: "Proposals" }).getByText("Run the specs with bin/rspec.")).toBeVisible();
  await expect.element(status()).toHaveTextContent("");
});

test("Apply sends the proposals back as Organize held them, then reads the entries again and states the count", async () => {
  const { apply, read } = opened([WIDGETS, DUPLICATE], {
    held: HELD_MERGE,
    apply: async () => ({ ok: true, action: "merge", applied: 1, reported: [] }),
  });
  await page.getByRole("button", { name: "Apply all" }).click();
  await expect.element(status()).toHaveTextContent("Merged 1 entry");
  expect(apply).toHaveBeenCalledWith("merge", [MERGE]);
  expect(read).toHaveBeenCalledTimes(2);
  expect(page.getByRole("list", { name: "Proposals" }).elements()).toHaveLength(0);
});

test("Close tells Fleet to drop the held proposals and clears the cards, which stay gone on the next read", async () => {
  const { dismiss, apply } = opened([WIDGETS, DUPLICATE], { held: HELD_MERGE });
  await page.getByRole("button", { name: "Close" }).click();
  expect(dismiss).toHaveBeenCalledTimes(1);
  expect(page.getByRole("list", { name: "Proposals" }).elements()).toHaveLength(0);
  expect(apply).not.toHaveBeenCalled();
});

test("a failed look says Failed in the status line, beside the proposals that did arrive", async () => {
  opened([WIDGETS], {
    held: { running: false, last: { action: "stale", proposals: [{ id: "1", entry_ids: ["1"], text: WIDGETS.text, note: "Not confirmed in 60 days" }], failed: true } },
  });
  await expect.element(status()).toHaveTextContent("Failed");
  await expect.element(page.getByText("Not confirmed in 60 days")).toBeVisible();
});

test("a look with nothing to propose says Nothing found, and the empty result is dropped from Fleet", async () => {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
  const { fleet, dismiss } = opened([WIDGETS]);
  await choose("Find stale entries");
  fleet.organize = { running: false, last: { action: "stale", proposals: [], failed: false } };
  await vi.advanceTimersByTimeAsync(SEVERAL);
  await expect.element(status()).toHaveTextContent("Nothing found");
  expect(dismiss).toHaveBeenCalledTimes(1);
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

/** What a host draws under Retros: its own frames hold the State menu, so Retros can be left from them. */
function Fixes() {
  return (
    <div>
      {useStateMenu()}
      <p>The retro items</p>
    </div>
  );
}

test("the retro items are a tab of their own, drawn in place of the entries", async () => {
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: true, entries: [WIDGETS], tidy: { running: false }, organize: { running: false } }));
  mount(<Logbook onRead={read} onEdit={vi.fn()} onLock={vi.fn()} onForget={vi.fn()} onOrganize={NONE} onDismissOrganize={DISMISSED} onApplyOrganize={NOTHING} fixes={<Fixes />} />);
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
  const read = vi.fn(async (): Promise<LogbookRead> => ({ ok: true, entries: [WIDGETS], tidy: { running: false }, organize: { running: false } }));
  const at = (toFixes: number) => <Logbook onRead={read} onEdit={vi.fn()} onLock={vi.fn()} onForget={vi.fn()} onOrganize={NONE} onDismissOrganize={DISMISSED} onApplyOrganize={NOTHING} fixes={<Fixes />} toFixes={toFixes} />;
  mount(at(1));
  await expect.element(page.getByText("The retro items")).toBeVisible();
  await inState("In use");
  await expect.element(line(/Run specs with bin\/rspec/)).toBeVisible();
  rerender(at(2));
  await expect.element(page.getByText("The retro items")).toBeVisible();
});

// The common look (`docs/concepts/logbook.md`): the controls are the log panel's own strip, the head is the thread's 36px one, and
// the Record and Evidence are flat rows under a legend.

const RICH = entry("11", {
  section: "repo",
  repository: "/Users/user/widgets",
  title: "Specs through bin/rspec",
  text: "Run specs with bin/rspec, never rake.",
  topic: "testing",
  evidence: [{ job_id: "j-1", job_title: "Fix the specs", cite: "refusal:1", at: AT }],
  history: [{ text: "Folded in.", until: AT, by: "merged" as const }],
});

/** The log panel the entries are drawn in, and the page's own top. */
const logPanel = () => entries().element().closest(".armada-logbook__log")!;
const root = () => document.querySelector(".armada-logbook")!;
const top = (one: Element) => Math.round(one.getBoundingClientRect().top);

test("State, Topic and Organize are in the log panel's strip, and nothing sits between the page's top and the panels", async () => {
  opened([RICH]);
  await expect.element(entries()).toBeVisible();
  for (const name of ["State", "Topic", "Organize"]) {
    const control = page.getByRole("button", { name }).element();
    expect(control.closest(".armada-logbook__log"), name).toBe(logPanel());
  }
  // The log starts at the page's own top edge, so no row of controls is above it.
  expect(top(logPanel())).toBe(top(root()));
  expect(top(logPanel().firstElementChild!)).toBe(top(root()));
});

test("neither the entries nor an open entry's Record, Evidence and Earlier draw a column heading", async () => {
  opened([RICH]);
  await expect.element(page.getByRole("table", { name: "Evidence" })).toBeVisible();
  await expect.element(page.getByRole("table", { name: "Earlier" })).toBeVisible();
  expect(page.getByRole("columnheader").elements()).toHaveLength(0);
});

test("the open entry's head holds its title and its acts, and the Record's first row is the id", async () => {
  opened([RICH]);
  const title = detail().getByRole("heading", { level: 2, name: "Specs through bin/rspec" });
  await expect.element(title).toBeVisible();
  const head = title.element().parentElement!;
  for (const name of [/^Edit/, /^Lock/, /^Forget/]) {
    expect(head.contains(detail().getByRole("button", { name }).element()), String(name)).toBe(true);
  }
  // What it is is the Record's, so the head does not repeat the kind.
  expect(head.textContent).not.toMatch(/Convention/);
  const first = detail().getByRole("table", { name: "Record" }).getByRole("row").first().element();
  expect(first.textContent).toBe("Id11");
});

test("the entry's acts are ghost, and Save stays primary", async () => {
  opened([RICH]);
  for (const name of [/^Edit/, /^Lock/, /^Forget/]) await expect.element(detail().getByRole("button", { name })).toHaveAttribute("data-variant", "ghost");
  await detail().getByRole("button", { name: /^Edit/ }).click();
  await expect.element(detail().getByRole("button", { name: "Save" })).toHaveAttribute("data-variant", "primary");
});

test("Organize's Apply all stays primary, and the card's own Apply and Dismiss are ghost", async () => {
  opened([WIDGETS, DUPLICATE], { held: HELD_MERGE });
  const proposal = page.getByRole("region", { name: "Proposal" });
  await expect.element(proposal.getByRole("button", { name: "Apply all" })).toHaveAttribute("data-variant", "primary");
  await expect.element(proposal.getByRole("button", { name: "Apply", exact: true })).toHaveAttribute("data-variant", "ghost");
  await expect.element(proposal.getByRole("button", { name: "Dismiss" })).toHaveAttribute("data-variant", "ghost");
});

test("s, t and o open State, Topic and Organize from the list", async () => {
  opened([RICH]);
  await expect.element(entries()).toBeVisible();
  (entries().element() as HTMLElement).focus();
  await userEvent.keyboard("s");
  await expect.element(page.getByRole("menuitem", { name: "In use" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  expect(page.getByRole("menu").elements()).toHaveLength(0);
  (entries().element() as HTMLElement).focus();
  await userEvent.keyboard("t");
  await expect.element(page.getByRole("menuitem", { name: "All topics" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  expect(page.getByRole("menu").elements()).toHaveLength(0);
  (entries().element() as HTMLElement).focus();
  await userEvent.keyboard("o");
  await expect.element(page.getByRole("menuitem", { name: "Merge duplicates" })).toBeVisible();
});

test("the arrows move the open entry, Enter goes to it and Esc comes back to the list", async () => {
  opened([WIDGETS, LEDGER]);
  await expect.element(detail().getByRole("heading", { name: "Run specs with bin/rspec" })).toBeVisible();
  (entries().element() as HTMLElement).focus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(detail().getByRole("heading", { name: "Migrations run on port 5433" })).toBeVisible();
  await userEvent.keyboard("{Enter}");
  await expect.element(detail()).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  await expect.element(entries()).toHaveFocus();
});
