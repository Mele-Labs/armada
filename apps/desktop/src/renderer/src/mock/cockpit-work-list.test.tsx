// The flight deck's list through `App` on the `cockpit-sessions-list` scenario: headed Work, with a Filter menu in its band that holds a
// search and the scopes Active, Quiet, Ended and All; `/` opens it with the search focused; choosing Ended narrows the list and the band says
// so; no Sessions or Jobs tab stands in the title bar; and a Session that ended is a row and never a drone on the Field. The walk
// `cockpit-sessions-list` draws it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const filter = () => list().getByRole("button", { name: /^Filter/ });
const search = () => page.getByRole("searchbox", { name: "Search Jobs" });
const places = () => page.getByRole("navigation", { name: "Places" });
const row = (title: string) => list().getByRole("button", { name: title, exact: true });
const drones = () => [...document.querySelectorAll<HTMLElement>(".bf [data-hit=drone]")].map((one) => one.getAttribute("data-label"));

async function opened() {
  mount("cockpit-sessions-list");
  await onScreen();
  await expect.element(list()).toBeVisible();
  // The live Sessions are in the list before anything is pressed.
  await expect.element(row("Prune the stale branches")).toBeVisible();
}

test("the list is headed Work and its Filter menu holds a search and the four scopes, Active checked", async () => {
  await opened();
  await expect.element(page.getByRole("region", { name: "Jobs and Sessions", exact: true })).not.toBeInTheDocument();
  await expect.element(filter()).toHaveTextContent("Filter");
  await userEvent.click(filter());
  await expect.element(search()).toBeVisible();
  for (const scope of ["Active", "Quiet", "Ended", "All"]) await expect.element(page.getByRole("menuitem", { name: scope, exact: true })).toBeVisible();
  // The scope the list is on is the one marked.
  await expect.element(page.getByRole("menuitem", { name: "Active", exact: true })).toHaveAttribute("aria-current", "true");
  await expect.element(page.getByRole("menuitem", { name: "Ended", exact: true })).not.toHaveAttribute("aria-current");
});

test("/ opens the Filter menu from the list with the search focused", async () => {
  await opened();
  await expect.element(search()).not.toBeInTheDocument();
  await userEvent.keyboard("/");
  await expect.element(search()).toBeVisible();
  await expect.element(search()).toHaveFocus();
});

test("Ended narrows the list to the Sessions that ended and the band names it", async () => {
  await opened();
  await userEvent.click(filter());
  await userEvent.click(page.getByRole("menuitem", { name: "Ended", exact: true }));
  await expect.element(row("Trim the retry loop")).toBeVisible();
  await expect.element(row("Prune the stale branches")).not.toBeInTheDocument();
  await expect.element(list().getByRole("button", { name: "The retry loop", exact: true })).not.toBeInTheDocument();
  await expect.element(filter()).toHaveTextContent("Ended");
  await expect.element(filter()).toHaveAccessibleName("Filter: Ended");
});

test("a search in Ended finds the Session past the week, and the band carries the words", async () => {
  await opened();
  await expect.element(list().getByRole("button", { name: "Backfill the audit log", exact: true })).not.toBeInTheDocument();
  await userEvent.click(filter());
  await userEvent.click(page.getByRole("menuitem", { name: "Ended", exact: true }));
  // Choosing a scope closes the menu; `/` opens it again with the search focused.
  await userEvent.keyboard("/");
  await expect.element(search()).toHaveFocus();
  await userEvent.keyboard("1702");
  await expect.element(row("Backfill the audit log")).toBeVisible();
  await expect.element(row("Trim the retry loop")).not.toBeInTheDocument();
  await expect.element(filter()).toHaveAccessibleName("Filter: Ended · 1702");
});

test("the title bar has no Sessions or Jobs tab", async () => {
  await opened();
  await expect.element(places().getByRole("tab", { name: "Cockpit" })).toBeVisible();
  await expect.element(places().getByRole("tab", { name: "Sessions" })).not.toBeInTheDocument();
  await expect.element(places().getByRole("tab", { name: "Jobs" })).not.toBeInTheDocument();
});

test("a Session that ended is a row of the list and no drone on the Field", async () => {
  await opened();
  await expect.poll(() => drones().length).toBeGreaterThan(0);
  expect(drones()).toContain("Prune the stale branches");
  await userEvent.click(filter());
  await userEvent.click(page.getByRole("menuitem", { name: "Ended", exact: true }));
  await expect.element(row("Trim the retry loop")).toBeVisible();
  expect(drones()).not.toContain("Trim the retry loop");
  expect(drones().some((label) => /^Prune the stale branches \d+$/.test(label ?? ""))).toBe(false);
});
