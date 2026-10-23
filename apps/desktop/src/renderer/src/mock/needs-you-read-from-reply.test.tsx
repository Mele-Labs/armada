// A waiting item Fleet read out of an agent's reply, through `App` on `not-a-question`: it draws its own glyph, ends in two
// thumbs that say whether Fleet read it right, and a right press on the row offers the same two. An item the agent stated
// has neither. The walk of the same name draws it; this holds that the presses reach the same code.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const SUMMARY = /^What you get/;
const NEXT = /^Next:/;
const ASKED = /^Should the header keep the approved mark/;

/** The Session opened, with the Needs you panel on its list of the three items. */
async function band() {
  const app = mount("not-a-question");
  await onScreen();
  await toSession(/Trim the Progress header/);
  const held = page.getByRole("region", { name: "Needs you" });
  await userEvent.click(held.getByRole("button", { name: /^Needs you/ }));
  const row = (text: RegExp) => held.getByRole("listitem", { name: text });
  await expect.element(row(SUMMARY)).toBeVisible();
  return { app, held, row };
}

test("a row read from a reply draws the Read from the reply glyph, and one the agent stated draws the question glyph", async () => {
  const { row } = await band();
  await expect.element(row(SUMMARY).getByRole("img", { name: "Read from the reply" })).toBeVisible();
  await expect.element(row(NEXT).getByRole("img", { name: "Read from the reply" })).toBeVisible();

  await expect.element(row(ASKED).getByRole("img", { name: "Question" })).toBeVisible();
  await expect.element(row(ASKED).getByRole("img", { name: "Read from the reply" })).not.toBeInTheDocument();
});

test("a row read from a reply ends in A question and Not a question thumbs", async () => {
  const { row } = await band();
  await expect.element(row(SUMMARY).getByRole("button", { name: "A question", exact: true })).toBeVisible();
  await expect.element(row(SUMMARY).getByRole("button", { name: "Not a question", exact: true })).toBeVisible();
});

test("Not a question takes the row off and tells Fleet it was misread", async () => {
  const { app, held, row } = await band();
  const dismiss = vi.spyOn(app.api, "dismissWaiting");

  await userEvent.click(row(SUMMARY).getByRole("button", { name: "Not a question", exact: true }));
  await expect.element(row(SUMMARY)).not.toBeInTheDocument();
  await expect.element(row(NEXT)).toBeVisible();
  await expect.element(held).toBeVisible();
  expect(dismiss).toHaveBeenCalledTimes(1);
  expect(dismiss).toHaveBeenLastCalledWith({ session_id: "naq-1", item_id: expect.stringMatching(/^replied:.*:0$/), misread: true });
});

test("A question keeps the row, tells Fleet to confirm it, and the row loses its thumbs", async () => {
  const { app, row } = await band();
  const confirm = vi.spyOn(app.api, "confirmWaiting");
  const dismiss = vi.spyOn(app.api, "dismissWaiting");

  await userEvent.click(row(NEXT).getByRole("button", { name: "A question", exact: true }));
  await expect.poll(() => confirm.mock.calls.length).toBe(1);
  expect(confirm).toHaveBeenLastCalledWith({ session_id: "naq-1", item_id: expect.stringMatching(/^replied:.*:1$/) });
  expect(dismiss).not.toHaveBeenCalled();

  await expect.element(row(NEXT)).toBeVisible();
  await expect.element(row(NEXT).getByRole("button", { name: "A question", exact: true })).not.toBeInTheDocument();
  await expect.element(row(NEXT).getByRole("button", { name: "Not a question", exact: true })).not.toBeInTheDocument();
  // The other row Fleet read is still unconfirmed.
  await expect.element(row(SUMMARY).getByRole("button", { name: "A question", exact: true })).toBeVisible();
});

test("a right press on the row opens a menu of both acts, and picking one does what its thumb does", async () => {
  const { app, row } = await band();
  const confirm = vi.spyOn(app.api, "confirmWaiting");
  const dismiss = vi.spyOn(app.api, "dismissWaiting");

  await userEvent.click(row(NEXT), { button: "right" });
  await expect.element(page.getByRole("menuitem", { name: "A question", exact: true })).toBeVisible();
  await userEvent.click(page.getByRole("menuitem", { name: "Not a question", exact: true }));
  await expect.element(row(NEXT)).not.toBeInTheDocument();
  expect(dismiss).toHaveBeenCalledTimes(1);
  expect(dismiss).toHaveBeenLastCalledWith({ session_id: "naq-1", item_id: expect.stringMatching(/^replied:.*:1$/), misread: true });

  await userEvent.click(row(SUMMARY), { button: "right" });
  await userEvent.click(page.getByRole("menuitem", { name: "A question", exact: true }));
  await expect.poll(() => confirm.mock.calls.length).toBe(1);
  expect(confirm).toHaveBeenLastCalledWith({ session_id: "naq-1", item_id: expect.stringMatching(/^replied:.*:0$/) });
  await expect.element(row(SUMMARY)).toBeVisible();
});

test("on a confirmed row the menu offers only Not a question", async () => {
  const { row } = await band();
  await userEvent.click(row(NEXT).getByRole("button", { name: "A question", exact: true }));
  await expect.element(row(NEXT).getByRole("button", { name: "A question", exact: true })).not.toBeInTheDocument();

  await userEvent.click(row(NEXT), { button: "right" });
  await expect.element(page.getByRole("menuitem", { name: "Not a question", exact: true })).toBeVisible();
  expect(page.getByRole("menuitem", { name: "A question", exact: true }).query()).toBeNull();
});

test("an item the agent stated has no thumbs and no menu", async () => {
  const { row } = await band();
  await expect.element(row(ASKED)).toBeVisible();
  expect(row(ASKED).getByRole("button", { name: "A question", exact: true }).query()).toBeNull();
  expect(row(ASKED).getByRole("button", { name: "Not a question", exact: true }).query()).toBeNull();

  await userEvent.click(row(ASKED), { button: "right" });
  expect(page.getByRole("menuitem").query()).toBeNull();
});
