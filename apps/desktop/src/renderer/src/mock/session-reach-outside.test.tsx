// A Session's read outside its slots, through `App`: its card says it is a read, draws the whole
// command and offers the five answers over a folder picker, Always allow on Documents settles the
// card, and Settings, Features lists the folder beside the blocked one, with Remove taking it off
// again. Another Session's write says it is a write, and Always allow for this job settles it with
// nothing listed. A third, a node script the guard cannot class, says run. #2441.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, toSystemPage, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Reach outside: the card's folder picker, Always allow, and the list in Settings", async () => {
  mount("session-reach-outside");
  await onScreen();
  await toSession(/Retry budget/);

  // The question is the only thing waiting, so the panel opens on its card by itself.
  const card = page.getByRole("article", { name: "Waiting on you" });
  const answers = card.getByRole("group", { name: "Answers" });
  await expect.element(card.getByText("Permission to read", { exact: true })).toBeVisible();
  await expect.element(card.getByText("Always applies to reads of", { exact: true })).toBeVisible();
  // The command is cut at 200 characters on the wire and drawn whole, to its last word.
  await expect.element(card.getByText(/echo gutterline$/)).toBeVisible();
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
  await expect.element(card.getByText("Permission to write", { exact: true })).toBeVisible();
  await expect.element(card.getByText("Always applies to writes to", { exact: true })).toBeVisible();
  await userEvent.click(answers.getByRole("button", { name: "Always allow for this job notes.md", exact: true }));
  await expect.element(card).not.toBeInTheDocument();

  // A command the guard cannot class says run, over the command, and never write.
  await toSession(/Tidy other repo/);
  await expect.element(card.getByText("Permission to run", { exact: true })).toBeVisible();
  await expect.element(card.getByText(/tidy\.mjs ~\/code\/other-repo/)).toBeVisible();
  await expect.element(card.getByText("Always applies to commands on", { exact: true })).toBeVisible();

  await toSystemPage("Features");
  const list = page.getByRole("list", { name: "Folders jobs reach" });
  await expect.element(list.getByText("/Users/user/Documents", { exact: true })).toBeVisible();
  await expect.element(list.getByText("/Users/user/.ssh", { exact: true })).toBeVisible();
  await expect.element(list.getByText("/Users/user/Desktop/drafts/notes.md", { exact: true })).not.toBeInTheDocument();
  await userEvent.click(list.getByRole("button", { name: "Remove /Users/user/Documents" }));
  await expect.element(list.getByText("/Users/user/Documents", { exact: true })).not.toBeInTheDocument();
});
