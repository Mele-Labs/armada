// A Session's read outside its slots, through `App`: its card says it is a read and offers the five
// answers over a folder picker, Always allow on Documents settles the card, and Settings, Features
// lists the folder beside the blocked one, with Remove taking it off again. Another Session's write
// says it is a write, and Always allow for this job settles it with nothing listed. #2441.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, toSystemPage, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The question arrives in the Needs you list, open; its row opens the card that answers it. */
async function openCard() {
  const needs = page.getByRole("region", { name: "Needs you" });
  await userEvent.click(needs.getByRole("button", { name: /^Open / }));
}

test("Reach outside: the card's folder picker, Always allow, and the list in Settings", async () => {
  mount("session-reach-outside");
  await onScreen();
  await toSession(/Custom margins/);

  await openCard();
  const card = page.getByRole("article", { name: "Waiting on you" });
  const answers = card.getByRole("group", { name: "Answers" });
  await expect.element(card.getByText("Permission to read", { exact: true })).toBeVisible();
  await expect.element(card.getByText("Always applies to reads of", { exact: true })).toBeVisible();
  for (const name of ["Reject once", "Always allow HANDOFF.md", "Always allow for this job HANDOFF.md", "Always block HANDOFF.md"]) {
    await expect.element(answers.getByRole("button", { name, exact: true })).toBeVisible();
  }
  const picker = card.getByRole("group", { name: "Folder for always" });
  await expect.element(picker.getByRole("button", { name: "HANDOFF.md" })).toHaveAttribute("aria-pressed", "true");
  await userEvent.click(picker.getByRole("button", { name: "Documents" }));
  await expect.element(picker.getByRole("button", { name: "Documents" })).toHaveAttribute("aria-pressed", "true");
  await userEvent.click(answers.getByRole("button", { name: "Always allow Documents", exact: true }));
  await expect.element(card).not.toBeInTheDocument();

  await toSession(/Release notes/);
  await openCard();
  await expect.element(card.getByText("Permission to write", { exact: true })).toBeVisible();
  await expect.element(card.getByText("Always applies to writes to", { exact: true })).toBeVisible();
  await userEvent.click(answers.getByRole("button", { name: "Always allow for this job notes.md", exact: true }));
  await expect.element(card).not.toBeInTheDocument();

  await toSystemPage("Settings");
  await page.getByRole("tab", { name: "Features" }).click();
  const list = page.getByRole("list", { name: "Folders jobs reach" });
  await expect.element(list.getByText("/Users/user/Documents", { exact: true })).toBeVisible();
  await expect.element(list.getByText("/Users/user/.ssh", { exact: true })).toBeVisible();
  await expect.element(list.getByText("/Users/user/Desktop/drafts/notes.md", { exact: true })).not.toBeInTheDocument();
  await userEvent.click(list.getByRole("button", { name: "Remove /Users/user/Documents" }));
  await expect.element(list.getByText("/Users/user/Documents", { exact: true })).not.toBeInTheDocument();
});
