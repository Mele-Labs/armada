// What Needs you holds, through `App` on `needs-me-replied-and-delete`: a reply is filed under Idle with an
// unread mark and raises no call, a done stays under Needs you, and an item the agent stated is removed
// with a reason or without, and the agent is told in its thread. The walk of the same name draws it;
// this holds that the presses reach the same code.

import { beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetDismissals } from "../cockpit/dismissed";
import { DONE_ITEM, REPLIED_ITEM } from "./needs-me-ids";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();
beforeEach(forgetDismissals);

const FIRST = "Confirm the old fixture can go";
const SECOND = "Pick a name for the cache table";

const rail = () => page.getByRole("complementary");
const work = () => page.getByRole("region", { name: "Jobs", exact: true });
const filed = (title: string) => work().getByRole("button", { name: title, exact: true });
const thread = () => page.getByRole("region", { name: "Thread" });
const reason = (item: string) => page.getByRole("textbox", { name: `Reason for removing ${item}` });
const remove = (item: string) => page.getByRole("button", { name: `Remove ${item}`, exact: true });

/** The Cockpit, with the dispatch bar it opens in left so the keys reach the card. */
async function cockpit() {
  const app = mount("needs-me-replied-and-delete");
  await onScreen();
  await userEvent.keyboard("{Escape}");
  return app;
}

/** Cache key opened, with the Needs you panel on its composer open on the list of the two items its agent stated. */
async function band() {
  const app = mount("needs-me-replied-and-delete");
  await onScreen();
  await toSession(/Cache key/);
  const held = page.getByRole("region", { name: "Needs you" });
  await userEvent.click(held.getByRole("button", { name: /^Needs you/ }));
  await expect.element(held.getByRole("button", { name: `Open ${FIRST}`, exact: true })).toBeVisible();
  return { app, held };
}

/** The item pressed open in the panel's place, with its acts. */
async function item(held: ReturnType<typeof page.getByRole>, name: string) {
  await userEvent.click(held.getByRole("button", { name: `Open ${name}`, exact: true }));
  await expect.element(remove(name)).toBeVisible();
}

test("a reply has the unread mark and a done is marked Done, and the mark goes once the Session is opened", async () => {
  mount("needs-me-replied-and-delete");
  await onScreen();
  await expect.element(work()).toBeVisible();

  await expect.element(filed("Pin the store clock").getByRole("img", { name: "Unread reply" })).toBeVisible();
  await expect.element(filed("Changelog script").getByRole("img", { name: "Done" })).toBeVisible();
  await expect.element(filed("Cache key")).toBeVisible();

  await toSession(/Pin the store clock/);
  await expect.element(filed("Pin the store clock").getByRole("img", { name: "Idle" })).toBeVisible();
  await expect.element(filed("Pin the store clock").getByRole("img", { name: "Unread reply" })).not.toBeInTheDocument();
  await expect.element(filed("Changelog script").getByRole("img", { name: "Done" })).toBeVisible();
});

test("a reply raises no call, and the done and the agent's items do", async () => {
  await cockpit();
  await expect.element(page.getByRole("region", { name: /^Job: Cache key/ })).toBeVisible();
  // Every call is in front or a strip behind it; none is the reply's.
  expect(page.getByRole("region", { name: /Pin the store clock/ }).query()).toBeNull();
  expect(rail().getByRole("button", { name: /Pin the store clock/ }).query()).toBeNull();
  await expect.element(rail().getByRole("button", { name: /Changelog script/ })).toBeVisible();
});

test("Remove with a reason tells the agent in its thread, and the item leaves the list; Esc removes nothing", async () => {
  const { app, held } = await band();
  const dismiss = vi.spyOn(app.api, "dismissWaiting");

  await item(held, FIRST);
  await userEvent.click(remove(FIRST));
  await expect.element(reason(FIRST)).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  await expect.element(reason(FIRST)).not.toBeInTheDocument();
  expect(dismiss).not.toHaveBeenCalled();
  await expect.element(remove(FIRST)).toBeVisible();

  await userEvent.click(remove(FIRST));
  await userEvent.type(reason(FIRST), "The cache tests still read it.{Enter}");
  await expect.element(thread().getByText(`Owner removed "${FIRST}": The cache tests still read it.`, { exact: true })).toBeVisible();
  await expect.element(remove(FIRST)).not.toBeInTheDocument();
  // The item pressed open is gone, so the panel is back on its list, which holds the other.
  await expect.element(held.getByRole("button", { name: `Open ${FIRST}`, exact: true })).not.toBeInTheDocument();
  await expect.element(held.getByRole("button", { name: `Open ${SECOND}`, exact: true })).toBeVisible();
  expect(dismiss).toHaveBeenCalledTimes(1);
  expect(dismiss).toHaveBeenLastCalledWith({ session_id: "o1", item_id: "o1-a1", reason: "The cache tests still read it." });
});

test("Remove with no reason posts the message without one", async () => {
  const { app, held } = await band();
  const dismiss = vi.spyOn(app.api, "dismissWaiting");

  await item(held, SECOND);
  await userEvent.click(remove(SECOND));
  await userEvent.keyboard("{Enter}");
  await expect.element(thread().getByText(`Owner removed "${SECOND}"`, { exact: true })).toBeVisible();
  await expect.element(remove(SECOND)).not.toBeInTheDocument();
  await item(held, FIRST);
  expect(dismiss).toHaveBeenLastCalledWith({ session_id: "o1", item_id: "o1-a2" });
});

test("a reply or a done dismissed posts nothing to its agent", async () => {
  const app = mount("needs-me-replied-and-delete");
  const dismiss = vi.spyOn(app.api, "dismissWaiting");
  await onScreen();
  await expect.element(filed("Pin the store clock").getByRole("img", { name: "Unread reply" })).toBeVisible();
  await expect.element(filed("Changelog script").getByRole("img", { name: "Done" })).toBeVisible();

  // Nothing in the window dismisses either any more, so Fleet is asked as another window or the phone would.
  expect((await app.api.dismissWaiting({ session_id: "r1", item_id: REPLIED_ITEM })).ok).toBe(true);
  expect((await app.api.dismissWaiting({ session_id: "d1", item_id: DONE_ITEM })).ok).toBe(true);
  expect(dismiss).toHaveBeenCalledTimes(2);
  // Each is settled when its Session reads as idle, which is also when anything it was going to say would be in the thread.
  await expect.element(filed("Pin the store clock").getByRole("img", { name: "Idle" })).toBeVisible();
  await expect.element(filed("Changelog script").getByRole("img", { name: "Idle" })).toBeVisible();
  for (const [title, said] of [["Pin the store clock", /The clock is pinned in all four store tests/], ["Changelog script", /The script is merged/]] as const) {
    await toSession(new RegExp(title));
    await expect.element(thread().getByText(said).first()).toBeVisible();
    expect(thread().getByText(/Owner removed/).query()).toBeNull();
  }
});

test("the call card removes an item the agent stated, with a reason or without, and tells the agent", async () => {
  const app = await cockpit();
  const dismiss = vi.spyOn(app.api, "dismissWaiting");
  const front = page.getByRole("region", { name: /^Job: Cache key/ });
  // Which of the two is in front is the deck's to say; the one behind steps up when it goes.
  const inFront = (item: string) => front.getByText(item, { exact: true });
  await expect.poll(() => inFront(FIRST).query() !== null || inFront(SECOND).query() !== null).toBe(true);
  const [one, other] = inFront(FIRST).query() !== null ? [FIRST, SECOND] : [SECOND, FIRST];
  const said = new Map([[FIRST, "o1-a1"], [SECOND, "o1-a2"]]);

  // An item the agent stated is removed, not dismissed: its card has no Dismiss.
  expect(rail().getByRole("button", { name: /Dismiss/ }).query()).toBeNull();
  await userEvent.click(remove(one));
  await userEvent.type(reason(one), "The cache tests still read it.{Enter}");
  await expect.element(inFront(one)).not.toBeInTheDocument();
  expect(dismiss).toHaveBeenLastCalledWith({ session_id: "o1", item_id: said.get(one), reason: "The cache tests still read it." });

  // The other item is still a call, behind: its strip brings it forward.
  await userEvent.click(rail().getByRole("button", { name: /Cache key/ }));
  // d opens the reason box on the card in front, and Enter on it removes with none.
  await expect.element(inFront(other)).toBeVisible();
  await userEvent.keyboard("d");
  await expect.element(reason(other)).toHaveFocus();
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("region", { name: /^Job: Changelog script/ })).toBeVisible();
  expect(dismiss).toHaveBeenLastCalledWith({ session_id: "o1", item_id: said.get(other) });

  // A done is not the agent's own item: it is dismissed, and nothing offers to remove it.
  expect(page.getByRole("button", { name: /^Remove/ }).query()).toBeNull();
  await expect.element(rail().getByRole("button", { name: /Dismiss/ })).toBeVisible();

  await toSession(/Cache key/);
  await expect.element(thread().getByText(`Owner removed "${one}": The cache tests still read it.`, { exact: true })).toBeVisible();
  await expect.element(thread().getByText(`Owner removed "${other}"`, { exact: true })).toBeVisible();
});
