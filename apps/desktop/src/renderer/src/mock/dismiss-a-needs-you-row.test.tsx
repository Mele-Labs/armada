// A walk, or a pull request Fleet read from the reply, is put away from the Needs you panel in one press, through
// `App` on `dismiss-a-needs-you-row`: the press goes to Fleet's `dismissWaiting` for that item and the row goes.
// The walk of the same name draws it; this holds that the presses reach the same code.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const WALK = /ready to look at/;
const PR = /won't merge/;

async function band() {
  const app = mount("dismiss-a-needs-you-row");
  await onScreen();
  await toSession(/Docs for the pairing screen/);
  const held = page.getByRole("region", { name: "Needs you" });
  await userEvent.click(held.getByRole("button", { name: /^Needs you/ }));
  const row = (text: RegExp) => held.getByRole("listitem", { name: text });
  await expect.element(row(WALK)).toBeVisible();
  return { app, row };
}

test("a walk row is dismissed in one press, on Fleet, with no reason box", async () => {
  const { app, row } = await band();
  const dismiss = vi.spyOn(app.api, "dismissWaiting");

  await userEvent.click(row(WALK).getByRole("button", { name: /^Dismiss/ }));
  await expect.element(row(WALK)).not.toBeInTheDocument();
  await expect.element(row(PR)).toBeVisible();
  expect(dismiss).toHaveBeenCalledWith({ session_id: "s22", item_id: "walk:https://git.example/pairing" });
});

test("a pull request row read from the reply is dismissed the same way", async () => {
  const { app, row } = await band();
  const dismiss = vi.spyOn(app.api, "dismissWaiting");

  await userEvent.click(row(PR).getByRole("button", { name: /^Dismiss/ }));
  await expect.element(row(PR)).not.toBeInTheDocument();
  expect(dismiss).toHaveBeenCalledWith({ session_id: "s22", item_id: "replied:pairing-pr" });
});
