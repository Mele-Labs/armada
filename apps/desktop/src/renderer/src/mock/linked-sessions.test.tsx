// A Session that handed groups of its plan to Sessions it follows, through `App` on `linked-sessions`: each group handed carries its Session's
// row on the Plan and the same Session is in the Ledger under Linked; the children stand under the parent in Work; what a child does is a row
// in the parent's thread where it happened; and Enter opens a child while Esc returns to the parent as it was. The walk `linkedSessions`
// plays it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const PARENT = "Move the store to named migrations";
const work = () => page.getByRole("region", { name: "Work", exact: true });
const plan = () => page.getByRole("region", { name: "Plan", exact: true });
const ledger = () => page.getByRole("region", { name: "Ledger", exact: true });

async function parent() {
  mount("linked-sessions");
  await onScreen();
  await expect.element(work()).toBeVisible();
  await work().getByRole("button", { name: PARENT, exact: true }).click();
  await expect.element(page.getByRole("region", { name: `Session: ${PARENT}` })).toBeVisible();
}

test("in Work each linked Session stands right under its parent, led by link and naming its group", async () => {
  await parent();
  const rows = [...document.querySelectorAll<HTMLElement>("[data-row]")];
  const at = rows.findIndex((one) => one.getAttribute("aria-label") === PARENT);
  expect(at).toBeGreaterThanOrEqual(0);
  const under = rows.slice(at + 1, at + 4);
  expect(under.map((one) => one.getAttribute("aria-label"))).toEqual(["Migrations get names", "Migrations apply in order", "One row for each migration"]);
  expect(under.every((one) => one.hasAttribute("data-linked"))).toBe(true);
  await expect.element(work().getByLabelText("Linked Session, handed from Name the migrations")).toBeVisible();
});

test("on the Plan a group handed to a Session carries its row in two lines, and one not handed offers the hand off", async () => {
  await parent();
  const group = plan().getByRole("region", { name: "Name the migrations", exact: true });
  const row = group.getByRole("button", { name: "Open Migrations get names" });
  await expect.element(row).toBeVisible();
  await expect.element(row.getByLabelText("Linked Session")).toBeVisible();
  // Its second line holds the state mark and the pull request, and the group is the row it stands under, so it is not said again.
  await expect.element(row.getByText("#2412")).toBeVisible();
  expect(row.element().querySelector(".armada-linked__group")).toBeNull();
  const open = plan().getByRole("button", { name: "List them in System", exact: true });
  await open.click();
  await expect.element(plan().getByRole("button", { name: "Hand to a Session" })).toBeVisible();
});

test("in the Ledger the same Sessions are under Linked, each with the group it came from", async () => {
  await parent();
  const row = ledger().getByRole("button", { name: "Open Migrations apply in order" });
  await expect.element(row).toBeVisible();
  await expect.element(row.getByLabelText("Handed from Apply them in order")).toBeVisible();
  expect(row.element().querySelectorAll(".armada-linked__head, .armada-linked__facts")).toHaveLength(2);
});

test("a child that needs the owner is a row in the parent's thread where it happened, and a press opens it", async () => {
  await parent();
  timePasses();
  const thread = page.getByRole("region", { name: "Thread", exact: true });
  const row = thread.getByRole("button", { name: "Migrations apply in order, Needs you" });
  await expect.element(row).toBeVisible();
  timePasses();
  await expect.element(thread.getByRole("button", { name: "One row for each migration, Merged" })).toBeVisible();
  await row.click();
  await expect.element(page.getByRole("region", { name: "Session: Migrations apply in order" })).toBeVisible();
});

test("Enter on a Session's row opens it and Esc is Back to the parent", async () => {
  await parent();
  const row = ledger().getByRole("button", { name: "Open Migrations apply in order" });
  row.element().focus();
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("region", { name: "Session: Migrations apply in order" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("region", { name: `Session: ${PARENT}` })).toBeVisible();
});
