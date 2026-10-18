// A reply that asks and replies that only tell, through `App` on `reply-asks-or-tells`: the asking one is a call, so its
// row in Work pulses and wears the needs tint; the two that told wear the faint unread mark and raise none; the deck has
// no Incoming calls panel. The walk of the same name draws it; this holds that the presses reach the same code.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { ASKED_ITEM } from "./needs-me-ids";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const work = () => page.getByRole("region", { name: "Work", exact: true });
const row = (title: string) => work().getByRole("button", { name: title, exact: true });
const flagged = (title: string, flag: string) => row(title).element().matches(`[${flag}]`);

test("the asking reply is a calling row under Needs you, and the replies that only tell are not", async () => {
  mount("reply-asks-or-tells");
  await onScreen();
  await expect.element(row("Retry cap")).toBeVisible();

  expect(flagged("Retry cap", "data-needs")).toBe(true);
  expect(flagged("Retry cap", "data-calling")).toBe(true);
  expect(flagged("Retry cap", "data-reply")).toBe(false);
  for (const title of ["Pin the store clock", "Trim the log lines"]) {
    await expect.element(row(title).getByRole("img", { name: "Unread reply" })).toBeVisible();
    expect(flagged(title, "data-reply")).toBe(true);
    expect(flagged(title, "data-needs")).toBe(false);
    expect(flagged(title, "data-calling")).toBe(false);
  }
  expect(flagged("Migration notes", "data-calling")).toBe(false);
  expect(row("Migration notes").getByRole("img", { name: "Unread reply" }).query()).toBeNull();

  expect(page.getByRole("region", { name: "Incoming calls" }).query()).toBeNull();
});

test("answering the asking line sends the reply and the row stops calling", async () => {
  const app = mount("reply-asks-or-tells");
  const answer = vi.spyOn(app.api, "answerWaiting");
  await onScreen();
  await expect.element(row("Retry cap")).toBeVisible();
  expect(flagged("Retry cap", "data-calling")).toBe(true);

  expect((await app.api.answerWaiting({ session_id: "q1", item_id: ASKED_ITEM, text: "Ten." })).ok).toBe(true);
  expect(answer).toHaveBeenLastCalledWith({ session_id: "q1", item_id: ASKED_ITEM, text: "Ten." });
  await expect.poll(() => flagged("Retry cap", "data-calling")).toBe(false);
  await toSession(/Retry cap/);
  await expect.element(page.getByRole("region", { name: "Thread" }).getByText(/Ten\./).first()).toBeVisible();
});
