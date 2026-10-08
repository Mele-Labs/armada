// Fork from a dead Session on a real Fleet's path, through `App`: a Session that ended, or whose
// terminal stopped asking, offers Fork in place of its message box; a live one offers the box and no
// Fork; and pressing it asks Fleet to fork that Session and opens the new one. What stands where
// main stands is `sessions-fleet.ts`. The walk (`walks/session-fork.ts`) shows how it reads.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "./scenario";
import type { Scenario } from "./scenario";
import { FakeSessionsFleet, hosted, terminal } from "./sessions-fleet";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const ENDED = "01ENDEDAAAAAAAAAAAAAAAAAAA";

function served(fleet: FakeSessionsFleet): Scenario {
  const board = onBoard([job("escalated", { id: "01JOB", handle: "52-the-retry-loop", title: "The retry loop", owner_manifest_id: "armada" })], {
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
const VIEW: Record<string, string> = { "One that ended": "Ended", "A terminal gone quiet": "Quiet" };

async function open(title: string) {
  const list = page.getByRole("region", { name: "Sessions" });
  await userEvent.click(list.getByRole("tab", { name: VIEW[title] ?? "Active" }));
  await userEvent.click(list.getByRole("button", { name: title }));
}
const fork = () => page.getByRole("button", { name: "Fork", exact: true });
const message = () => page.getByRole("textbox", { name: "Message" });

test("Fork is offered on a Session that ended or went quiet, and a message box on none of them", async () => {
  mount(served(new FakeSessionsFleet(records())));
  await onScreen();

  for (const title of ["One that ended", "A terminal gone quiet"]) {
    await open(title);
    await expect.element(fork()).toBeVisible();
    await expect.element(message()).not.toBeInTheDocument();
    await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  }
});

test("A live Session, hosted or in a terminal that is listening, offers a message box and no Fork", async () => {
  mount(served(new FakeSessionsFleet(records())));
  await onScreen();

  for (const title of ["A live hosted one", "A terminal being listened to"]) {
    await open(title);
    await expect.element(message()).toBeVisible();
    await expect.element(fork()).not.toBeInTheDocument();
    await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  }
});

test("Pressing Fork asks Fleet to fork that Session and opens the new one, linked back to it", async () => {
  const fleet = new FakeSessionsFleet(records());
  mount(served(fleet));
  await onScreen();

  await open("One that ended");
  await userEvent.click(fork());

  await expect.poll(() => fleet.calls.forked).toEqual([ENDED]);
  const ledger = page.getByRole("region", { name: "Attachments" });
  await expect.element(ledger.getByText("Forked from One that ended")).toBeVisible();
  // The new Session is live: it takes a message, and offers no Fork.
  await expect.element(message()).toBeVisible();
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
  await userEvent.click(fork());

  const folded = page.getByRole("group", { name: "Forked from A terminal gone quiet" });
  await expect.element(folded).toBeVisible();
  await expect.element(page.getByText("It splits on newlines.")).not.toBeVisible();
  await userEvent.click(page.getByRole("button", { name: "Forked from A terminal gone quiet", exact: true }));
  await expect.element(page.getByText("It splits on newlines.")).toBeVisible();
});
