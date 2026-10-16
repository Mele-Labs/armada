// A Session's read outside its slots, through `App`: its card offers the four answers over a folder
// picker, Always allow on Documents settles the card, and Settings, Features lists the folder beside
// the blocked one, with Remove taking it off again. #2441.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toPlace, toSystemPage, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Reach outside: the card's folder picker, Always allow, and the list in Settings", async () => {
  mount("session-reach-outside");
  await onScreen();
  await toPlace("Sessions");
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: /Custom margins/ }));

  const card = page.getByRole("article", { name: "Waiting on you" });
  const answers = card.getByRole("group", { name: "Answers" });
  for (const name of ["Reject once", "Always allow", "Always block"]) await expect.element(answers.getByRole("button", { name })).toBeVisible();
  const picker = card.getByRole("group", { name: "Folder for always" });
  await expect.element(picker.getByRole("button", { name: "HANDOFF.md" })).toHaveAttribute("aria-pressed", "true");
  await userEvent.click(picker.getByRole("button", { name: "Documents" }));
  await expect.element(picker.getByRole("button", { name: "Documents" })).toHaveAttribute("aria-pressed", "true");
  await userEvent.click(answers.getByRole("button", { name: "Always allow" }));
  await expect.element(card).not.toBeInTheDocument();

  await toSystemPage("Settings");
  await page.getByRole("tab", { name: "Features" }).click();
  const list = page.getByRole("list", { name: "Folders sessions reach" });
  await expect.element(list.getByText("/Users/user/Documents", { exact: true })).toBeVisible();
  await expect.element(list.getByText("/Users/user/.ssh", { exact: true })).toBeVisible();
  await userEvent.click(list.getByRole("button", { name: "Remove /Users/user/Documents" }));
  await expect.element(list.getByText("/Users/user/Documents", { exact: true })).not.toBeInTheDocument();
});
