// A Session that handed groups of its plan to Sessions it follows, through `App` on `linked-sessions`: each group handed carries its Session's
// row in the Blueprint and the same Session is in the Artifacts layer under Sessions; the children stand under the parent in Work; what a child does
// is a row in the parent's thread where it happened; and Enter opens a child while Esc returns to the parent as it was. The walk
// `linkedSessions` plays it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const PARENT = "Move the store to named migrations";
const work = () => page.getByRole("region", { name: "Jobs", exact: true });
const blueprint = () => page.getByRole("region", { name: "Blueprint", exact: true });
const artifacts = () => page.getByRole("region", { name: "Artifacts", exact: true });
const openArtifacts = () => page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Artifacts" }).click();

async function parent() {
  mount("linked-sessions");
  await onScreen();
  await expect.element(work()).toBeVisible();
  await work().getByRole("button", { name: PARENT, exact: true }).click();
  await expect.element(page.getByRole("region", { name: `Job: ${PARENT}` })).toBeVisible();
}

test("in Work each linked Session stands right under its parent, led by link and naming its group", async () => {
  await parent();
  const rows = [...document.querySelectorAll<HTMLElement>("[data-row]")];
  const at = rows.findIndex((one) => one.getAttribute("aria-label") === PARENT);
  expect(at).toBeGreaterThanOrEqual(0);
  const under = rows.slice(at + 1, at + 4);
  expect(under.map((one) => one.getAttribute("aria-label"))).toEqual(["Migrations get names", "Migrations apply in order", "One row for each migration"]);
  expect(under.every((one) => one.hasAttribute("data-linked"))).toBe(true);
  await expect.element(work().getByLabelText("Linked Job, handed from Name the migrations")).toBeVisible();
});

test("in the Blueprint a group handed to a Session carries its row in two lines, and one not handed offers the hand off", async () => {
  await parent();
  await blueprint().getByRole("button", { name: "Whole blueprint", exact: true }).click();
  const group = blueprint().getByRole("region", { name: "Name the migrations", exact: true });
  const row = group.getByRole("button", { name: "Open Migrations get names" });
  await expect.element(row).toBeVisible();
  await expect.element(row.getByLabelText("Linked Job")).toBeVisible();
  // Its second line holds the state mark and the pull request, and the group is the row it stands under, so it is not said again.
  await expect.element(row.getByText("#2412")).toBeVisible();
  expect(row.element().querySelector(".armada-linked__group")).toBeNull();
  const open = blueprint().getByRole("button", { name: "List them in System", exact: true });
  await open.click();
  await expect.element(blueprint().getByRole("button", { name: "Hand to a Job" })).toBeVisible();
});

test("in the Artifacts layer the same Jobs are under Jobs, each with the group it came from", async () => {
  await parent();
  await openArtifacts();
  const row = artifacts().getByRole("button", { name: "Migrations apply in order" });
  await expect.element(row).toBeVisible();
  await expect.element(row.getByText("Apply them in order")).toBeVisible();
  await expect.element(artifacts().getByRole("region", { name: "Jobs", exact: true })).toBeVisible();
});

test("a child that merges is a row in the parent's thread where it happened, a child that asks is not, and a press opens it", async () => {
  await parent();
  timePasses();
  const thread = page.getByRole("region", { name: "Thread", exact: true });
  await expect.element(thread.getByRole("button", { name: "Migrations apply in order, Needs you" })).not.toBeInTheDocument();
  timePasses();
  const row = thread.getByRole("button", { name: "One row for each migration, Merged" });
  await expect.element(row).toBeVisible();
  await row.click();
  await expect.element(page.getByRole("region", { name: "Job: One row for each migration" })).toBeVisible();
});

test("Enter on a Session's row in Artifacts opens it and Esc is Back to the parent", async () => {
  await parent();
  await openArtifacts();
  const row = artifacts().getByRole("button", { name: "Migrations apply in order" });
  row.element().focus();
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("region", { name: "Job: Migrations apply in order" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("region", { name: `Job: ${PARENT}` })).toBeVisible();
});
