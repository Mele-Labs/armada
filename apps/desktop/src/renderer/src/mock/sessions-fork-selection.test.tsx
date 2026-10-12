// Fork from words selected in a message, on a live Session, through `App`: a bar over a selection
// inside one message offers Fork from here; Esc and a selection across two messages take it away; and
// pressing it asks Fleet to fork at that message, opens the new Session with the words quoted in its
// box, and folds only the old conversation up to that message. The walk is `walks/session-fork-selection.ts`.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "./scenario";
import type { Scenario } from "./scenario";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const AT = "2026-10-07T13:48:02.000Z";
const LIVE = "01LIVEAAAAAAAAAAAAAAAAAAAA";

const FOUND = "The test builds its expected timestamp from the wall clock, and the store stamps the row from its own read of it.";
const WORDS = "the store stamps the row from its own read of it";

const thread = (): Record<string, SessionRow[]> => ({
  [LIVE]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "store_open fails about one run in ten. Find out why." },
    { kind: "message", id: "m2", at: AT, from: { kind: "agent" }, text: FOUND },
    { kind: "message", id: "m3", at: AT, from: { kind: "you" }, text: "Is that the only place it reads the clock?" },
    { kind: "message", id: "m4", at: AT, from: { kind: "agent" }, text: "No. Two more: the retention sweep and a fixture helper." },
  ],
});

function served(fleet: FakeSessionsFleet): Scenario {
  const board = onBoard([job("escalated", { id: "01JOB", handle: "52-the-retry-loop", title: "The retry loop", owner_manifest_id: "armada" })], {
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  return fleet.scenario(board);
}

async function open() {
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  const list = page.getByRole("region", { name: "Sessions" });
  await userEvent.click(list.getByRole("tab", { name: "Active" }));
  await userEvent.click(list.getByRole("button", { name: "A live hosted one" }));
}

/** The text node under `row` holding `words`. */
function holding(row: string, words: string): { node: Text; at: number } {
  const block = document.querySelector(`[data-fork-row="${row}"]`);
  if (block === null) throw new Error(`no message ${row} is on the page`);
  const all = document.createTreeWalker(block, NodeFilter.SHOW_TEXT);
  for (let node = all.nextNode(); node !== null; node = all.nextNode()) {
    const at = node.textContent?.indexOf(words) ?? -1;
    if (at >= 0) return { node: node as Text, at };
  }
  throw new Error(`message ${row} does not hold "${words}"`);
}

/** Select `words` inside message `row`, as a drag ending in the text would leave them. */
function select(row: string, words: string) {
  const { node, at } = holding(row, words);
  window.getSelection()!.setBaseAndExtent(node, at, node, at + words.length);
  document.dispatchEvent(new Event("selectionchange"));
}

const bar = () => page.getByRole("toolbar", { name: "Selection" });
const forkHere = () => page.getByRole("button", { name: "Fork from here" });
const message = () => page.getByRole("textbox", { name: "Message" });

test("Words selected inside one message of a live Session offer Fork from here, and Esc takes the bar away", async () => {
  mount(served(new FakeSessionsFleet([hosted(LIVE, { title: "A live hosted one" })], thread())));
  await onScreen();
  await open();

  await expect.element(bar()).not.toBeInTheDocument();
  select("m2", WORDS);
  await expect.element(bar()).toBeVisible();
  await expect.element(forkHere()).toBeVisible();

  await userEvent.keyboard("{Escape}");
  await expect.element(bar()).not.toBeInTheDocument();
});

test("A selection that runs from one message into the next offers nothing", async () => {
  mount(served(new FakeSessionsFleet([hosted(LIVE, { title: "A live hosted one" })], thread())));
  await onScreen();
  await open();

  const from = holding("m2", WORDS);
  const to = holding("m3", "Is that the only place");
  window.getSelection()!.setBaseAndExtent(from.node, from.at, to.node, to.at + 7);
  document.dispatchEvent(new Event("selectionchange"));

  await expect.element(bar()).not.toBeInTheDocument();
});

test("Fork from here asks Fleet to fork at that message, quotes the words, and folds the conversation up to it and no further", async () => {
  const fleet = new FakeSessionsFleet([hosted(LIVE, { title: "A live hosted one" })], thread());
  mount(served(fleet));
  await onScreen();
  await open();

  select("m2", WORDS);
  await userEvent.click(forkHere());

  await expect.poll(() => fleet.calls.forkedAt).toEqual([{ sessionId: LIVE, at: "m2" }]);
  const folded = page.getByRole("group", { name: "Forked from A live hosted one" });
  await expect.element(folded).toBeVisible();
  await expect.element(message()).toHaveTextContent(`> ${WORDS}`);

  await userEvent.click(page.getByRole("button", { name: "Forked from A live hosted one", exact: true }));
  await expect.element(folded.getByText("Find out why.")).toBeVisible();
  await expect.element(folded.getByText(WORDS, { exact: false })).toBeVisible();
  // What was said after the message it was forked at is not in the copy.
  await expect.element(folded.getByText("Is that the only place")).not.toBeInTheDocument();
  await expect.element(folded.getByText("Two more")).not.toBeInTheDocument();
});
