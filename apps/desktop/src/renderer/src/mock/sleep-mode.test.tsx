// Sleep mode, through `App`: the moon beside Helm, a question answered for the owner overnight, and
// the Morning review that waking opens, with one answer overridden. The walk is `walks/sleep-mode.ts`.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const review = () => page.getByRole("dialog", { name: "Morning review" });

test("sleep mode", async () => {
  mount("session-question");
  await onScreen();

  const moon = page.getByRole("button", { name: "Sleep mode" });
  await expect.element(moon).toHaveAttribute("aria-pressed", "false");
  await userEvent.click(moon);
  await expect.element(moon).toHaveAttribute("aria-pressed", "true");
  await expect.element(page.getByRole("button", { name: "Morning review" })).toBeVisible();

  await userEvent.click(moon);
  await expect.element(review()).toBeVisible();
  const decided = review().getByRole("region", { name: "Decided for you" });
  await expect.element(decided.getByText("Large", { exact: true })).toBeVisible();
  await expect.element(review().getByRole("region", { name: "Still needs you" })).toBeVisible();
  await expect.element(review().getByRole("region", { name: "Landed overnight" })).toBeVisible();
  await expect.element(review().getByRole("region", { name: "Walks waiting" })).toBeVisible();

  await userEvent.click(decided.getByRole("button", { name: "Override" }).first());
  await userEvent.fill(decided.getByRole("textbox", { name: "Correction" }), "Medium");
  await userEvent.click(decided.getByRole("button", { name: "Send" }));
  await expect.element(decided.getByText("Medium")).toBeVisible();
});
