// Jumping into a Session through `App`: Needs you opens on the agent's question every time, even after it was folded
// and left, and stays folded when only items wait. The question's own row takes focus, the first thing the arrows move from.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Entry: a Session with a question opens Needs you on it each time he comes back", async () => {
  mount("dashboard-needs-you");
  await onScreen();
  await toSession(/Flaky store test/);
  const band = page.getByRole("region", { name: "Needs you" });
  const trigger = band.getByRole("button", { name: /^Needs you/ });
  await expect.element(trigger).toHaveAttribute("aria-expanded", "true");
  await userEvent.click(trigger);
  await expect.element(trigger).toHaveAttribute("aria-expanded", "false");

  await toSession(/Migration notes/);
  await toSession(/Flaky store test/);
  await expect.element(trigger).toHaveAttribute("aria-expanded", "true");
  await expect.poll(() => band.element().contains(document.activeElement)).toBe(true);
});

test("Entry: a Session with only items waiting leaves Needs you folded", async () => {
  mount("needs-you-steps");
  await onScreen();
  await toSession(/Add single sign-on/);
  const trigger = page.getByRole("region", { name: "Needs you" }).getByRole("button", { name: /^Needs you/ });
  await expect.element(trigger).toHaveAttribute("aria-expanded", "false");
  await toSession(/Add single sign-on/);
  await expect.element(trigger).toHaveAttribute("aria-expanded", "false");
});
