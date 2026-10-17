// A check pressed in a Session open on the Cockpit's stage opens its panel there, over the Cockpit,
// and does not carry the person to the Session's page. The walk `cockpit-session-check-panel` draws it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Cockpit: pressing a Session's check opens the panel over the Cockpit", async () => {
  mount("session-checks");
  await onScreen();
  const list = page.getByRole("region", { name: "Work", exact: true });
  await userEvent.click(list.getByRole("button", { name: "Session check rows" }));
  const pane = page.getByRole("region", { name: "Session: Session check rows" });
  await userEvent.click(pane.getByRole("button", { name: "Check desktop:typecheck, passed" }));

  await expect.element(page.getByRole("dialog", { name: "Check log" })).toBeVisible();
  // Still on the Cockpit: the list and the pane are where they were.
  await expect.element(list).toBeVisible();
  await expect.element(pane).toBeVisible();
  await userEvent.click(page.getByRole("dialog", { name: "Check log" }).getByRole("button", { name: /^Close/ }));
  await expect.element(page.getByRole("dialog", { name: "Check log" })).not.toBeInTheDocument();
  await expect.element(pane).toBeVisible();
});
