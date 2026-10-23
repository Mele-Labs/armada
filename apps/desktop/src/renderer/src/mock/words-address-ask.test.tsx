// Fleet's question about where a taught pattern opens in a repository, as a real Fleet raises it: a `words_address` item that the deck
// draws open, with the sites it knows as options, and whose own `Other` is the deck's free text and not a choice of its own. The
// answer reaches the repository's rules, and the key links. Over `teach-anything` with the question raised that way.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { build } from "./scenarios/teach-anything";
import { mount, onScreen, unmountAfterEach } from "./testing";
import { kit, toSessions } from "./sessions/walk-kit";
import { ARMADA_TITLE, KEY_MESSAGE, LEDGER_TITLE } from "./sessions/teach";
import { button, deckRow, inside, region, role, text, walkThrough } from "./walk";

unmountAfterEach();

test("the question draws with the known sites as its options and one Other, and answering it links the key", async () => {
  mount(build(false, true));
  await onScreen();
  const { thread } = kit(false);
  const needs = region("Needs you");
  await walkThrough([
    toSessions,
    { press: deckRow(ARMADA_TITLE), say: "" },
    { select: "STORE-1234", in: inside(thread, text(KEY_MESSAGE.slice(0, 40))), say: "" },
    { press: inside(role("toolbar", "Selection"), button("Teach")), say: "" },
    { type: "A Jira key. Link it to our Jira", into: inside(role("dialog", "Teach"), role("textbox", "Instruction")), say: "" },
    { press: inside(role("dialog", "Teach"), button(/^armada$/)), say: "" },
    { key: "Meta+Enter", on: inside(role("dialog", "Teach"), role("textbox", "Instruction")), say: "" },
    { press: inside(role("dialog", "Teach"), button("Save")), say: "" },
    toSessions,
    { press: deckRow(LEDGER_TITLE), say: "" },
    { press: button("Needs you", { exact: true }), say: "" },
    { look: inside(needs, text("Which Jira site does ledger use?")), say: "" },
  ]);
  await expect.element(page.getByText("acme.atlassian.net", { exact: true })).toBeVisible();
  await expect.element(page.getByText("Other", { exact: true })).toHaveLength(1);
  await walkThrough([
    { press: inside(needs, text("acme-payments.atlassian.net")), say: "" },
    { press: inside(needs, button("Answer", { exact: true })), say: "" },
    { look: inside(thread, role("link", "LED-210")), say: "" },
  ]);
});
