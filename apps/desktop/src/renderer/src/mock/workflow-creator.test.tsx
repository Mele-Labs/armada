// The Workflow creator mock: the list marks where each definition came from
// and what shadows what, the picked one is a graph with a back edge wherever a
// step sends work back, a definition the loader would refuse is refused with
// its rule on the step it is about, and a saved one lands in the scope that
// was chosen. Nothing here is real: no Fleet reads or writes a file.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const frame = () => page.getByRole("region", { name: "Workflow definition" });
const save = () => page.getByRole("button", { name: "Save", exact: true });

async function open() {
  mount("every-state");
  await onScreen();
  await page.getByRole("navigation", { name: "Machine" }).getByRole("button", { name: "Workflows", exact: true }).click();
}

test("rows are marked by place, a shadowed one reads as overridden, and a left-out one names its reason", async () => {
  await open();

  await expect.element(page.getByRole("button", { name: "bug, repository" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "bug, carried, overridden" })).toBeVisible();
  // A row draws its workflow as dots with an arc where a step sends work back, and a left-out row draws none.
  await expect.element(page.getByRole("button", { name: "bug, repository" }).getByRole("img", { name: /review returns to fix/ })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "hotfix, kit, left out" }).getByRole("img", { name: /,/ })).not.toBeInTheDocument();
  // The rail row is marked while any file is left out.
  await expect.element(page.getByRole("img", { name: "A workflow file cannot run" })).toBeVisible();
  // Nothing is drawn until a workflow is picked.
  await expect.element(frame()).not.toBeInTheDocument();

  await page.getByRole("button", { name: "hotfix, kit, left out" }).click();
  const left = page.getByRole("region", { name: "hotfix, left out" });
  await expect.element(left.getByText(/does not declare/)).toBeVisible();
  await expect.element(frame()).not.toBeInTheDocument();
});

test("a workflow is drawn as steps, a step sending work back is a back edge, and there is no structure to pick", async () => {
  await open();

  await page.getByRole("button", { name: "bug, repository" }).click();
  await expect.element(page.getByRole("button", { name: /^repro, / })).toBeVisible();
  await expect.element(page.getByText("up to 4 passes")).toBeVisible();
  await expect.element(page.getByRole("radio", { name: "Loop" })).not.toBeInTheDocument();
});

test("a step opens in the sheet, a Judge with no question is refused on its node, and fixing it saves", async () => {
  await open();

  await page.getByRole("button", { name: "release_notes, kit" }).click();
  await page.getByRole("button", { name: /^gather, / }).click();
  const panel = page.getByRole("dialog", { name: "gather" });
  await panel.getByRole("checkbox", { name: "Judge" }).click({ force: true });
  await panel.getByRole("button", { name: "Close" }).click();
  await save().click();

  const refusals = page.getByRole("list", { name: "Refusals" });
  await expect.element(refusals.getByText("Judge ticked and the step names no question")).toBeVisible();
  await expect.element(frame().getByText("Refused", { exact: true })).toBeVisible();
  await expect.element(page.getByRole("button", { name: /^gather, / }).getByText(/names no question/)).toBeVisible();

  await page.getByRole("button", { name: /^gather, / }).click();
  await page.getByRole("dialog", { name: "gather" }).getByRole("textbox", { name: "Judge question" }).fill("Does it cover every change?");
  await page.getByRole("dialog", { name: "gather" }).getByRole("button", { name: "Close" }).click();
  await expect.element(refusals).not.toBeInTheDocument();
  await save().click();
  await expect.element(frame().getByText("Saved", { exact: true })).toBeVisible();
});

test("scope is Kit or a Manifest from a dropdown, the Manifest the window is in selected for a new one", async () => {
  await open();

  await page.getByRole("button", { name: "New workflow" }).click();
  await frame().getByRole("button", { name: "Settings", exact: true }).click();
  const scope = page.getByRole("combobox", { name: "Scope" });
  await expect.element(scope).toHaveValue("armada");
  expect([...scope.element().querySelectorAll("option")].map((one) => one.textContent)).toEqual(["Kit", "armada", "ledger", "site"]);
});

test("saving a Kit file to this Manifest shadows the Kit file", async () => {
  await open();

  await page.getByRole("button", { name: "release_notes, kit" }).click();
  await frame().getByRole("button", { name: "Settings", exact: true }).click();
  const scope = page.getByRole("combobox", { name: "Scope" });
  await expect.element(scope).toHaveValue("kit");
  await scope.selectOptions("armada");
  await page.getByRole("dialog").getByRole("button", { name: "Close" }).click();
  await save().click();

  await expect.element(page.getByRole("button", { name: "release_notes, repository" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "release_notes, kit, overridden" })).toBeVisible();
});

test("an id another repository file holds is refused when written there", async () => {
  await open();

  await page.getByRole("button", { name: "New workflow" }).click();
  await frame().getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("textbox", { name: "Workflow id" }).fill("migration");
  await page.getByRole("dialog").getByRole("button", { name: "Close" }).click();
  await save().click();

  await expect.element(page.getByText("Another file in .armada/workflows has this id")).toBeVisible();
});

test("Discuss with Helm marks the draft as handed over", async () => {
  await open();

  await page.getByRole("button", { name: "New workflow" }).click();
  await page.getByRole("button", { name: "Discuss with Helm" }).click();
  await expect.element(page.getByRole("img", { name: "Handed to Helm" })).toBeVisible();
});
