// A Session moved to another repository, on a real Fleet's path, through `App`: its repository in the head is a menu of the
// others, a pick asks with what stays behind, Move asks Fleet and opens the new Session, and both threads name the other
// from the ledger Fleet wrote. A terminal still listening is drawn off, and Fleet's refusal is a toast. The walks
// `session-move-repository` and `cockpit-session-move-repository` show how it reads.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { SessionRow } from "@armada/protocol";

import { onBoard } from "./scenario";
import type { Scenario } from "./scenario";
import { FakeSessionsFleet, held, hosted, terminal } from "./sessions-fleet";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada", repository: "armada" } };
const STOREFRONT: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: { ...repository().manifest!, id: "storefront", repository: "storefront", path: "storefront/armada.yml" },
};

const WRONG = "01WRONGAAAAAAAAAAAAAAAAAAA";
const AT = "2026-10-15T13:31:00.000Z";

function served(fleet: FakeSessionsFleet): Scenario {
  const board = onBoard({ repositories: [ARMADA, STOREFRONT] });
  return fleet.scenario(board);
}

const records = () => [
  hosted(WRONG, { title: "Pin the store clock", attachments: [held("slot", "4"), held("branch", "fix/pin-store-clock")] }),
  hosted("01OTHERBBBBBBBBBBBBBBBBBBB", { title: "Round the totals" }),
  terminal("01LISTENINGCCCCCCCCCCCCCC", { title: "Why the reader drops lines" }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [WRONG]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "Pin the clock in the store tests." },
    { kind: "message", id: "m2", at: AT, from: { kind: "agent" }, text: "There are no store tests in this repository." },
  ],
});

async function open(title: string) {
  await toSession(title);
}
const toast = (headline: string) => page.getByText(headline, { exact: true }).first();

test("Move: the repository lists the others, the pick asks with what stays, and Move opens the new Session in the other repository", async () => {
  const fleet = new FakeSessionsFleet(records(), threads());
  mount(served(fleet));
  await onScreen();
  await open("Pin the store clock");

  await userEvent.click(page.getByRole("button", { name: "Repository armada" }));
  await expect.element(page.getByRole("menuitem", { name: "storefront" })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: "armada" })).not.toBeInTheDocument();
  await userEvent.click(page.getByRole("menuitem", { name: "storefront" }));

  const ask = page.getByRole("dialog", { name: "Move to another repository" });
  await expect.element(ask.getByText("fix/pin-store-clock")).toBeVisible();
  await expect.element(ask.getByText(/and its files stay in armada/)).toBeVisible();
  await expect.element(ask.getByText("Ends this Job")).toBeVisible();
  expect(fleet.calls.moved).toEqual([]);

  await userEvent.click(ask.getByRole("button", { name: "Move to storefront" }));
  expect(fleet.calls.moved).toEqual([{ sessionId: WRONG, manifestId: "storefront" }]);

  // The new Session is open, in the other repository, under the same title, with the conversation and a row naming where it came from.
  await expect.element(page.getByRole("button", { name: "Repository storefront" })).toBeVisible();
  await expect.element(page.getByText("Pin the store clock").first()).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Moved from armada" })).toBeVisible();
  await expect.element(page.getByText("There are no store tests in this repository.")).toBeVisible();

  // The row opens the old one: ended, ending with where it went, and its repository is text, so it cannot move twice.
  await userEvent.click(page.getByRole("button", { name: "Moved from armada" }));
  await expect.element(page.getByRole("button", { name: "Moved to storefront" })).toBeVisible();
  await expect.element(page.getByRole("img", { name: "Repository armada" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Repository armada" })).not.toBeInTheDocument();
});

test("Move: a Session whose terminal is still listening has every repository drawn off, and a press moves nothing", async () => {
  const fleet = new FakeSessionsFleet(records(), threads());
  mount(served(fleet));
  await onScreen();
  await open("Why the reader drops lines");

  await userEvent.click(page.getByRole("button", { name: "Repository armada" }));
  const item = page.getByRole("menuitem", { name: "storefront" });
  await expect.element(item).toHaveAttribute("aria-disabled", "true");
  await expect.element(page.getByText("Runs in a terminal that is still listening").first()).toBeInTheDocument();
  await userEvent.click(item, { force: true });
  await expect.element(page.getByRole("button", { name: /^Move to/ })).not.toBeInTheDocument();
  expect(fleet.calls.moved).toEqual([]);
});

test("Move: a move Fleet refuses is a toast in the usual place, and the Session stays where it was", async () => {
  const fleet = new FakeSessionsFleet(records(), threads());
  mount(served(fleet));
  await onScreen();
  await open("Round the totals");

  fleet.refusesMove = { code: "fleet.session_move_same", message: "session is already in that repository" };
  await userEvent.click(page.getByRole("button", { name: "Repository armada" }));
  await userEvent.click(page.getByRole("menuitem", { name: "storefront" }));
  await userEvent.click(page.getByRole("button", { name: "Move to storefront" }));
  await expect.element(toast("Already in that repository")).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Repository armada" })).toBeVisible();

  fleet.refusesMove = { code: "fleet.something_else", message: "Fleet cannot do that now." };
  await userEvent.click(page.getByRole("button", { name: "Move to storefront" }));
  await expect.element(toast("Fleet cannot do that now.")).toBeVisible();
});
