// The Workflow creator mock: the list says where each definition came from and
// what shadows what, a definition the loader would refuse is refused with its
// rule, and a saved one lands in the scope that was chosen. Nothing here is
// real: no Fleet reads or writes a file.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const frame = () => page.getByRole("region", { name: "Workflow definition" });

async function open() {
  mount("every-state");
  await onScreen();
  await page.getByRole("navigation", { name: "Machine" }).getByRole("button", { name: "Workflows", exact: true }).click();
}

test("the list names each file's place, a shadowed one, and a left-out one with its reason", async () => {
  await open();

  await expect.element(page.getByRole("listitem", { name: "bug, repository" })).toBeVisible();
  await expect.element(page.getByRole("listitem", { name: "bug, carried, overridden" })).toBeVisible();
  const leftOut = page.getByRole("listitem", { name: "hotfix, kit, left out" });
  await expect.element(leftOut).toBeVisible();
  await expect.element(leftOut.getByText(/does not declare/)).toBeVisible();
  // A file set aside is never edited.
  await expect.element(leftOut.getByRole("button")).not.toBeInTheDocument();
});

test("a loop that returns to nothing is refused with its rule, and fixing it saves", async () => {
  await open();

  await page.getByRole("listitem", { name: "release_notes, kit" }).getByRole("button", { name: "release_notes" }).click();
  await page.getByRole("radio", { name: "Loop" }).click({ force: true });
  await page.getByRole("button", { name: "Save", exact: true }).click();

  const refusals = page.getByRole("list", { name: "Refusals" });
  await expect.element(refusals.getByText("loop carries no verdict_routing")).toBeVisible();
  await expect.element(frame().getByText("Refused", { exact: true })).toBeVisible();

  await page.getByRole("radio", { name: "Sequence" }).click({ force: true });
  // Editing clears the refusal, and the band is a draft again.
  await expect.element(refusals).not.toBeInTheDocument();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect.element(frame().getByText("Saved", { exact: true })).toBeVisible();
});

test("Kit is the default scope, and saving to this Manifest shadows the Kit file", async () => {
  await open();

  await page.getByRole("listitem", { name: "release_notes, kit" }).getByRole("button", { name: "release_notes" }).click();
  await expect.element(page.getByRole("radio", { name: "Kit" })).toBeChecked();
  await page.getByRole("radio", { name: "This Manifest" }).click({ force: true });
  await page.getByRole("button", { name: "Save", exact: true }).click();

  await expect.element(page.getByRole("listitem", { name: "release_notes, repository" })).toBeVisible();
  await expect.element(page.getByRole("listitem", { name: "release_notes, kit, overridden" })).toBeVisible();
});

test("an id another repository file holds is refused when written there", async () => {
  await open();

  await page.getByRole("button", { name: "New workflow" }).click();
  await expect.element(page.getByRole("radio", { name: "Kit" })).toBeChecked();
  await page.getByRole("radio", { name: "This Manifest" }).click({ force: true });
  await page.getByRole("textbox", { name: "Workflow id" }).fill("migration");
  await page.getByRole("textbox", { name: "Step id" }).fill("run");
  await page.getByRole("button", { name: "Save", exact: true }).click();

  await expect.element(page.getByText("Another file in .armada/workflows has this id")).toBeVisible();
});

test("Discuss with Helm marks the draft as handed over", async () => {
  await open();

  await page.getByRole("button", { name: "New workflow" }).click();
  await page.getByRole("button", { name: "Discuss with Helm" }).click();
  await expect.element(page.getByRole("img", { name: "Handed to Helm" })).toBeVisible();
});
