// A Session's retro, through `App`: the Retro press in the More menu of the Session's own header on the Cockpit's stage writes it, and the retro is a card in the Session's thread that Fleet replaces as the
// writing goes. **The press moves nobody**: the card's `Open retro` link opens the kept retro on the
// Retros page, headed by the Session's title and address. A terminal Session has no card, and its
// press opens the retro when Fleet answers. The walk (`walks/session-retro.ts`) shows how it reads.

import { beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { s205SessionRetro, s205SessionRetroNothingNew, s205SessionRetroRefused } from "./scenarios/session-retro";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

// The Retros page remembers which source it listed, and one test narrows it.
beforeEach(() => window.localStorage.clear());

const more = () => page.getByRole("region", { name: /^Job: / }).getByRole("button", { name: "More" });
/** The head's More menu, open on its Retro item. */
const retro = async (name: "Retro" | "Writing the retro" = "Retro") => {
  await userEvent.click(more());
  return page.getByRole("menuitem", { name });
};
const sheet = () => page.getByRole("dialog", { name: "Retro" });
const thread = () => page.getByRole("region", { name: "Thread" });
const card = () => thread().getByRole("region", { name: "Retro" });
const ID = "01IDLECCCCCCCCCCCCCCCCCCCC";

/** Opens the Session and presses Retro in its header's More menu. */
async function pressInTheHeader(title = "Fix the flaky store test") {
  await toSession(title);
  await userEvent.click(await retro());
}

/** Writes a retro from the Session's header and opens it from the card, which is how a retro is reached now. */
async function writeAndOpen() {
  await pressInTheHeader();
  await userEvent.click(card().getByRole("button", { name: "Open retro" }));
}

test("The Retro press puts a retro card in the Session's thread, and the person stays in the thread", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await pressInTheHeader();

  // The press is out, so the mark holds and the card says what Fleet is doing.
  await expect.element(await retro("Writing the retro")).toBeDisabled();
  await userEvent.keyboard("{Escape}");
  await expect.element(card()).toBeVisible();
  // It settles to one line, and nothing took the person away from the thread meanwhile.
  await expect.element(card().getByText("Retro 1 written · 5 items")).toBeVisible();
  await expect.element(card().getByText("A reset waited 40 minutes for an answer")).toBeVisible();
  await expect.element(sheet()).not.toBeInTheDocument();
  await expect.element(thread()).toBeVisible();
  await expect.element(await retro()).toBeEnabled();
});

test("The card's Open retro link opens that retro on the Retros page, headed by the Session", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await writeAndOpen();

  await expect.element(sheet()).toBeVisible();
  await expect.element(sheet().getByText("Fix the flaky store test · s-01IDLECC")).toBeVisible();
  await expect.element(sheet().getByRole("listitem", { name: "A reset waited 40 minutes for an answer" })).toBeVisible();

  const item = sheet().getByRole("listitem", { name: "A reset waited 40 minutes for an answer" });
  await userEvent.click(item.getByRole("button", { name: "Reject change", exact: true }));
  await expect.element(item).not.toBeInTheDocument();
});

test("A second press while one is being written is refused in Fleet's words and the card goes on", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await pressInTheHeader();
  await expect.element(card()).toBeVisible();

  // The press is held while writing, so the second comes from elsewhere: the preload, as another window's would.
  const second = await window.armada.retroSession(ID);
  expect(second.ok).toBe(false);
  expect(!second.ok && second.why === "refused" && (second as { error?: { code?: string } }).error?.code).toBe("fleet.session_retro_being_written");
  await expect.element(card().getByText("Retro 1 written · 5 items")).toBeVisible();
});

test("A press answered with the last retro adds no card, so the answer opens that retro", async () => {
  mount(s205SessionRetroNothingNew);
  await onScreen();
  await pressInTheHeader();

  await expect.element(sheet()).toBeVisible();
  await expect.element(sheet().getByText("Fix the flaky store test · s-01IDLECC")).toBeVisible();
  expect(card().elements()).toHaveLength(0);
});

test("A terminal Session has no card, so its press keeps the mark and opens the retro when Fleet answers", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await pressInTheHeader("Tidy the release notes");

  await expect.element(sheet()).toBeVisible();
  await userEvent.click(sheet().getByRole("button", { name: "Close" }));
  expect(card().elements()).toHaveLength(0);
});

test("A refused retro says why and opens nothing", async () => {
  mount(s205SessionRetroRefused);
  await onScreen();
  await pressInTheHeader();

  await expect.element(page.getByText("A retro is already being written for this Session")).toBeVisible();
  await expect.element(sheet()).not.toBeInTheDocument();
});

test("Closed, the retro's items stay on the Retros page beside the other Sessions', each from its Session and as the agent's way", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await writeAndOpen();
  await expect.element(sheet()).toBeVisible();
  await userEvent.click(sheet().getByRole("button", { name: "Close" }));

  const list = page.getByRole("list", { name: "Retros" });
  await expect.element(list.getByRole("button", { name: "Fix the flaky store test · s-01IDLECC" }).first()).toBeVisible();
  await expect.element(list.getByRole("button", { name: /^Retire guides 8 and 20/ }).first()).toBeVisible();
  await expect.element(list.getByRole("listitem", { name: "A restart of Fleet killed the subagents" }).getByText("Agent", { exact: true }).last()).toBeVisible();
});

test("A Session's read honours n: the numbered retro, the newest without it, and an unknown n refused", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await writeAndOpen();
  await expect.element(sheet()).toBeVisible();
  const id = ID;
  const first = await window.armada.readRetro({ kind: "session", id, n: 1 });
  expect(first.ok && first.retro.items?.[0]?.id).toBe(`${id}-r1-1`);
  const newest = await window.armada.readRetro({ kind: "session", id });
  expect(newest.ok && newest.retro.items?.[0]?.id).toBe(`${id}-r1-1`);
  expect((await window.armada.readRetro({ kind: "session", id, n: 9 })).ok).toBe(false);
});
