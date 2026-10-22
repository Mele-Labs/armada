// Fork from a dead Session on a real Fleet's path, through `App`: a Session that ended, or whose
// terminal stopped asking, offers Fork in its head's More menu, with no message box; a live one offers the box and no
// Fork; and pressing it asks Fleet to fork that Session and opens the new one. What stands where
// main stands is `sessions-fleet.ts`. The walk (`walks/session-fork.ts`) shows how it reads.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "./scenario";
import type { Scenario } from "./scenario";
import { FakeSessionsFleet, hosted, terminal } from "./sessions-fleet";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const ENDED = "01ENDEDAAAAAAAAAAAAAAAAAAA";

function served(fleet: FakeSessionsFleet): Scenario {
  const board = onBoard({
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  return fleet.scenario(board);
}

const records = () => [
  hosted("01LIVEAAAAAAAAAAAAAAAAAAAA", { title: "A live hosted one" }),
  terminal("01LISTENINGAAAAAAAAAAAAAA", { title: "A terminal being listened to" }),
  terminal("01QUIETAAAAAAAAAAAAAAAAAA", { title: "A terminal gone quiet", terminal: {} }),
  hosted(ENDED, { title: "One that ended", state: "ended", last_seen_at: new Date().toISOString() }),
];

/** The dead ones are in the list's Quiet and Ended views, off the Active one it opens on. */
const VIEW: Record<string, "Ended" | "Quiet"> = { "One that ended": "Ended", "A terminal gone quiet": "Quiet" };

async function open(title: string) {
  await toSession(title, VIEW[title] ?? "Active");
}
/** Fork is an item of the More menu in the Session's head, and the menu has to be opened to press it. */
const fork = () => page.getByRole("menuitem", { name: "Fork", exact: true });
const menu = () => userEvent.click(page.getByRole("region", { name: /^Job: / }).getByRole("button", { name: "More" }));
const message = () => page.getByRole("textbox", { name: "Message" });

test("Fork is offered on a Session that ended or went quiet, and a message box on none of them", async () => {
  mount(served(new FakeSessionsFleet(records())));
  await onScreen();

  for (const title of ["One that ended", "A terminal gone quiet"]) {
    await open(title);
    await menu();
    await expect.element(fork()).toBeVisible();
    await expect.element(message()).not.toBeInTheDocument();
  }
});

test("A live Session, hosted or in a terminal that is listening, offers a message box and no Fork", async () => {
  mount(served(new FakeSessionsFleet(records())));
  await onScreen();

  for (const title of ["A live hosted one", "A terminal being listened to"]) {
    await open(title);
    await expect.element(message()).toBeVisible();
    await menu();
    await expect.element(fork()).not.toBeInTheDocument();
  }
});

test("Pressing Fork asks Fleet to fork that Session and opens the new one, linked back to it", async () => {
  const fleet = new FakeSessionsFleet(records());
  mount(served(fleet));
  await onScreen();

  await open("One that ended");
  await menu();
  await userEvent.click(fork());

  await expect.poll(() => fleet.calls.forked).toEqual([ENDED]);
  await expect.element(page.getByRole("group", { name: "Forked from One that ended" })).toBeVisible();
  // The new Session is live: it takes a message, and offers no Fork.
  await expect.element(message()).toBeVisible();
  await menu();
  await expect.element(fork()).not.toBeInTheDocument();
});

test("Fork: the thread opens on one closed row holding the old conversation, read when it is pressed", async () => {
  const old = terminal("01QUIETAAAAAAAAAAAAAAAAAA", { title: "A terminal gone quiet", terminal: {} });
  const fleet = new FakeSessionsFleet([old], {
    [old.id]: [{ kind: "message", id: "m1", at: "2026-10-07T13:48:02.000Z", from: { kind: "agent" }, text: "It splits on newlines." }],
  });
  mount(served(fleet));
  await onScreen();

  await open("A terminal gone quiet");
  await menu();
  await userEvent.click(fork());

  const folded = page.getByRole("group", { name: "Forked from A terminal gone quiet" });
  await expect.element(folded).toBeVisible();
  // The Cockpit's row for the Session says its last words too, so the old conversation is read in the fold.
  const said = folded.getByText("It splits on newlines.");
  await expect.element(said).not.toBeVisible();
  await userEvent.click(page.getByRole("button", { name: "Forked from A terminal gone quiet", exact: true }));
  await expect.element(said).toBeVisible();
});
