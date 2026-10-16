// With a Session open on the flight deck's stage, the flight controls are that Session's: Ask for update sends it a status
// request, Compact runs its compact, Report hands off to a new Session, Auto land sends the land instruction and is kept, Map
// returns to the map, and Release is the sixth. Activity narrows to the Session's own events and returns to all of them when it closes. The brief is
// the owner's ask of 15 Oct 2026.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs and Sessions" });
const controls = () => page.getByRole("region", { name: "Flight controls" });
const pane = () => page.getByRole("region", { name: "Session: Pin the store clock" });
const feed = () => document.querySelector(".deck-activity")?.textContent ?? "";
const tiles = () => [...document.querySelectorAll(".deck-controls .deck-tile")].map((one) => one.getAttribute("aria-label"));

async function open() {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(pane()).toBeVisible();
}

test("with a Session open the flight controls are that Session's, and the six return when it closes", async () => {
  await open();
  await expect.poll(tiles).toEqual(["Ask for update", "Auto land", "Compact session context", "Report issue with session", "Map", "Release everything the Session holds"]);
  await pane().getByRole("button", { name: "Back" }).click();
  await expect.poll(tiles).toEqual(["Jobs only", "Sessions only", "Checks", "Needs me", "Map", "Timeline"]);
});

test("Ask for update sends the Session a status request, and the key 1 does the same", async () => {
  await open();
  await controls().getByRole("button", { name: "Ask for update" }).click();
  await expect.poll(() => pane().element().textContent?.split("Progress update, please: what is done, what is running now, what is next, and anything blocked.").length).toBe(2);
  await userEvent.keyboard("1");
  await expect.poll(() => pane().element().textContent?.split("Progress update, please: what is done, what is running now, what is next, and anything blocked.").length).toBe(3);
});

test("Compact runs the Session's compact", async () => {
  await open();
  await controls().getByRole("button", { name: "Compact session context" }).click();
  await expect.poll(() => pane().element().textContent?.includes("/compact")).toBe(true);
});

test("Report is offered where the Session's repository is known", async () => {
  await open();
  await expect.element(controls().getByRole("button", { name: "Report issue with session" })).not.toHaveAttribute("aria-disabled");
});

test("turning Auto land on sends the Session the land instruction, and the tile reads on after a reload", async () => {
  const first = mount("cockpit-ship-flightdeck");
  await onScreen();
  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(pane()).toBeVisible();
  const auto = () => controls().getByRole("button", { name: "Auto land" });
  await expect.element(auto()).toHaveAttribute("aria-pressed", "false");
  await auto().click();
  await expect.poll(() => pane().element().textContent?.includes("Auto land is on. When your work is done, land it")).toBe(true);
  await expect.element(auto()).toHaveAttribute("aria-pressed", "true");

  // A new window, as after a restart.
  first.unmount();
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(auto()).toHaveAttribute("aria-pressed", "true");

  await auto().click();
  await expect.poll(() => pane().element().textContent?.includes("Auto land is off. Do not land on your own.")).toBe(true);
  await expect.element(auto()).toHaveAttribute("aria-pressed", "false");
});

test("Map closes the Session and shows the map, from the tile and from the key 5", async () => {
  await open();
  await controls().getByRole("button", { name: "Map" }).click();
  await expect.poll(() => document.querySelector(".deck-stage > .deck-pane")?.getAttribute("aria-label") ?? "").not.toContain("Session:");
  await expect.poll(tiles).toContain("Jobs only");

  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(pane()).toBeVisible();
  await userEvent.keyboard("5");
  await expect.poll(tiles).toContain("Jobs only");
  await expect.element(pane()).not.toBeInTheDocument();
});

test("with a Session open Activity shows only its events, and all of them again when it closes", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  const other = "Shorten the reconnect wait";
  await expect.poll(() => feed().includes(other)).toBe(true);

  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(pane()).toBeVisible();
  await expect.poll(() => feed().includes(other)).toBe(false);
  expect(feed()).toContain("Pin the store clock");

  await pane().getByRole("button", { name: "Back" }).click();
  await expect.poll(() => feed().includes(other)).toBe(true);
});

test("with a Job open Activity shows only its events", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  await expect.poll(() => feed().includes("Pin the store clock")).toBe(true);
  await list().getByRole("button", { name: "Shorten the reconnect wait", exact: true }).click();
  await expect.poll(() => feed().includes("Pin the store clock")).toBe(false);
  expect(feed()).toContain("Shorten the reconnect wait");
});
