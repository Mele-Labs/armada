// What Needs you holds, through `App` on `needs-me-replied-and-delete`: a reply that only tells wears an
// unread mark and raises no call, a done stays under Needs you, and an item the agent stated is removed
// with a reason or without, and the agent is told in its thread. The walk of the same name draws it;
// this holds that the presses reach the same code.

import { beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetDismissals } from "../cockpit/dismissed";
import { DONE_ITEM } from "./needs-me-ids";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();
beforeEach(forgetDismissals);

const FIRST = "Confirm the old fixture can go";
const SECOND = "Pick a name for the cache table";

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
  const calling = (title: string) => filed(title).element().matches("[data-calling]");
  await expect.poll(() => calling("Cache key")).toBe(true);
  expect(calling("Changelog script")).toBe(true);
  expect(calling("Pin the store clock")).toBe(false);
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

test("a done dismissed posts nothing to its agent", async () => {
  const app = mount("needs-me-replied-and-delete");
  const dismiss = vi.spyOn(app.api, "dismissWaiting");
  await onScreen();
  await expect.element(filed("Pin the store clock").getByRole("img", { name: "Unread reply" })).toBeVisible();
  await expect.element(filed("Changelog script").getByRole("img", { name: "Done" })).toBeVisible();

  // Nothing in the window dismisses it any more, so Fleet is asked as another window or the phone would.
  expect((await app.api.dismissWaiting({ session_id: "d1", item_id: DONE_ITEM })).ok).toBe(true);
  expect(dismiss).toHaveBeenCalledTimes(1);
  // It is settled when its Session reads as idle, which is also when anything it was going to say would be in the thread.
  await expect.element(filed("Changelog script").getByRole("img", { name: "Idle" })).toBeVisible();
  await toSession(/Changelog script/);
  await expect.element(thread().getByText(/The script is merged/).first()).toBeVisible();
  expect(thread().getByText(/Owner removed/).query()).toBeNull();
});
