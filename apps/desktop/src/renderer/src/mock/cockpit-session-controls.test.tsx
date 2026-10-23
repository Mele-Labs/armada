// With a Job open on the flight deck's stage, the flight controls are that Session's: Ask for update sends it a status
// request, Compact runs its compact, Report hands off to a new Session, Auto land is Fleet's flag on the Session and sends the agent its words, Map
// returns to the map, and Release is the sixth. Activity gives its place to the Job's own panels and returns when it closes. The brief is
// the owner's ask of 15 Oct 2026.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const controls = () => page.getByRole("region", { name: "Flight controls" });
const pane = () => page.getByRole("region", { name: "Job: Pin the store clock" });
// Activity names each event on its dot.
const feed = () => [...document.querySelectorAll(".deck-activity .armada-tl__dot")].map((one) => one.getAttribute("aria-label")).join("\n");
const tiles = () => [...document.querySelectorAll(".deck-controls .deck-tile")].map((one) => one.getAttribute("aria-label"));

async function open() {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(pane()).toBeVisible();
}

test("with a Job open the flight controls are that Session's, and the four return when it closes", async () => {
  await open();
  await expect.poll(tiles).toEqual(["Ask for update", "Auto land", "Compact job context", "Report issue with job", "Map", "Judge", "Release everything the Job holds", "Artifacts", "Subagents", "Checks", "Bays", "Requests", "Terminal", "Pause"]);
  await pane().getByRole("button", { name: "Back" }).click();
  await expect.poll(tiles).toEqual(["Checks", "Needs me", "Map", "Timeline"]);
});

test("Ask for update sends the Session a status request, and the key 1 does the same", async () => {
  await open();
  await controls().getByRole("button", { name: "Ask for update" }).click();
  await expect.poll(() => pane().element().textContent?.split("Bring your task list up to date with TaskUpdate, using the plan's task text word for word if you recorded a plan, then say what is done, what is running now, what is next, and anything blocked.").length).toBe(2);
  await userEvent.keyboard("1");
  await expect.poll(() => pane().element().textContent?.split("Bring your task list up to date with TaskUpdate, using the plan's task text word for word if you recorded a plan, then say what is done, what is running now, what is next, and anything blocked.").length).toBe(3);
});

test("Compact runs the Session's compact", async () => {
  await open();
  await controls().getByRole("button", { name: "Compact job context" }).click();
  await expect.poll(() => pane().element().textContent?.includes("/compact")).toBe(true);
});

test("Report is offered where the Session's repository is known", async () => {
  await open();
  await expect.element(controls().getByRole("button", { name: "Report issue with job" })).not.toHaveAttribute("aria-disabled");
});

test("Auto land reads Off, then On once Fleet keeps it, and sends the Session the words for each", async () => {
  await open();
  const auto = () => controls().getByRole("button", { name: "Auto land" });
  await expect.element(auto()).toHaveAttribute("aria-pressed", "false");
  await expect.element(auto().getByRole("img", { name: "Auto land off" })).toBeVisible();

  await auto().click();
  await expect.poll(() => pane().element().textContent?.includes("Fleet turns on auto-merge when your pull request opens")).toBe(true);
  await expect.element(auto()).toHaveAttribute("aria-pressed", "true");
  await expect.element(auto().getByRole("img", { name: "Auto land on" })).toBeVisible();

  await auto().click();
  await expect.poll(() => pane().element().textContent?.includes("Auto land is off. Fleet turns auto-merge off.")).toBe(true);
  await expect.element(auto()).toHaveAttribute("aria-pressed", "false");
  await expect.element(auto().getByRole("img", { name: "Auto land off" })).toBeVisible();
});

test("Map closes the Session and shows the map, from the tile and from the key 5", async () => {
  await open();
  await controls().getByRole("button", { name: "Map" }).click();
  await expect.poll(() => document.querySelector(".deck-stage > .deck-pane")?.getAttribute("aria-label") ?? "").not.toContain("Job:");
  await expect.poll(tiles).toContain("Checks");

  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(pane()).toBeVisible();
  await userEvent.keyboard("5");
  await expect.poll(tiles).toContain("Checks");
  await expect.element(pane()).not.toBeInTheDocument();
});

test("with a Job open Activity gives its place to the Job's own panels, and returns with all its events when it closes", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  const other = "Migration notes";
  await expect.poll(() => feed().includes(other)).toBe(true);

  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(pane()).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Activity", exact: true })).not.toBeInTheDocument();

  await pane().getByRole("button", { name: "Back" }).click();
  await expect.poll(() => feed().includes(other)).toBe(true);
});
