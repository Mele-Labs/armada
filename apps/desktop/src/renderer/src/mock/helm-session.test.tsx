// The dock on a Helm Session, through `App` on the `helm-is-a-session` moment: what the dock does while the Session
// has a turn running. The look is the walk's; this holds the two things a walk only passes by, which are that
// Start fresh waits on a turn and that Stop is the Session's own.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, openHelm, unmountAfterEach } from "./testing";

unmountAfterEach();

const helm = () => page.getByRole("complementary", { name: "Helm" });
const box = () => helm().getByRole("textbox", { name: "Ask Helm" });

test("while Helm works, Stop is in its thread and Start fresh waits", async () => {
  mount("helm-is-a-session");
  await openHelm();
  await expect.element(helm().getByRole("button", { name: "Start fresh" })).toBeEnabled();

  await userEvent.click(box());
  await userEvent.keyboard("can you pin the clock in the ledger test?{Enter}");

  await expect.element(helm().getByRole("button", { name: "Stop" })).toBeVisible();
  await expect.element(helm().getByRole("button", { name: "Start fresh" })).toBeDisabled();
});
