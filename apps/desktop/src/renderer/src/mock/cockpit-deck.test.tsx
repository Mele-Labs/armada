// The flight deck in the real Cockpit, through `App` on the ship scenario: a call opens as a pane and leaves Incoming, a
// Session that replied raises a call, the Cockpit tab comes back to the deck as it was left, Checks narrows what the deck
// shows. The deck is the Cockpit, and no rail is drawn. #2301.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { viewedAt } from "../cockpit/viewed";
import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Work", exact: true });
const incoming = () => page.getByRole("region", { name: "Incoming calls" });
const controls = () => page.getByRole("region", { name: "Flight controls" });
const places = () => page.getByRole("navigation", { name: "Places" });
const stage = () => document.querySelector(".deck-stage > .deck-pane");
const hails = () => [...document.querySelectorAll<HTMLElement>(".deck-incoming .deck-hail")].map((one) => one.getAttribute("aria-label") ?? "");

async function deck() {
  const app = mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  return app;
}

test("calls open as panes, and an open Session's question is a card in its thread, not an Incoming call, until it is deselected", async () => {
  await deck();
  await expect.poll(() => hails().some((one) => one.startsWith("Session question: "))).toBe(true);
  const asked = hails().find((one) => one.startsWith("Session question: "))!;
  const before = hails().length;
  await incoming().getByRole("button", { name: asked }).click();

  // What it is about opens on the stage. While a Session is open its own calls are cards in its thread and leave Incoming calls, so nothing
  // shows twice (`withoutOpenSession`, the owner's ruling of 16 Oct 2026). Seen is not answered, so the question stays asked.
  await expect.poll(() => stage()?.getAttribute("aria-label") ?? "").toContain(asked.replace(/^[^:]+: /, ""));
  await expect.element(page.getByRole("article", { name: "Waiting on you" })).toBeVisible();
  expect(hails()).not.toContain(asked);
  expect(hails()).toHaveLength(before - 1);

  // Let go of the Session and its call is back, unanswered.
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => stage()?.getAttribute("aria-label") ?? "").toBe("Flight map");
  // It comes back as the same call, under the same label.
  await expect.poll(() => hails()).toContain(asked);
  expect(hails()).toHaveLength(before);
});

// Seen in this window was not enough: the deck's memory of it went with a restart, and a reply's call came back because the
// Session still read as unread. Open on the stage, the Session is read, as on the Sessions page (the owner, 15 Oct 2026).
test("a Session opened from its call is read, so its reply does not call again after a restart", async () => {
  await deck();
  const title = "Pin the store clock";
  timePasses();
  const call = `Session question: ${title}`;
  await expect.poll(() => hails()).toContain(call);
  const before = Date.now();
  await incoming().getByRole("button", { name: call }).click();

  await expect.poll(() => stage()?.getAttribute("aria-label") ?? "").toContain(title);
  await expect.poll(() => viewedAt("r1")).toBeGreaterThanOrEqual(before);
});

// A reply alone raises none (main's "Move replies out of Needs you", 9701ecf0c), so the Session asks a question. The deck is the only
// Cockpit, so the call is raised there and nowhere else.
test("a Session that waits on him raises an Incoming call", async () => {
  await deck();
  const title = "Pin the store clock";
  await expect.poll(() => hails().length).toBeGreaterThan(0);
  expect(hails().some((one) => one.includes(title))).toBe(false);

  timePasses();
  await expect.element(incoming().getByRole("button", { name: new RegExp(title) })).toBeVisible();
});

test("the Cockpit tab returns from a page opened from the deck exactly", async () => {
  await deck();
  const label = () => stage()?.getAttribute("aria-label") ?? "";
  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.poll(label).toContain("Pin the store clock");
  const was = label();

  // The reports open over the Cockpit, which stays mounted under them.
  await userEvent.keyboard("{Meta>}k{/Meta}");
  await page.getByRole("option", { name: "Reported" }).click();
  await expect.element(page.getByRole("button", { name: "Back to the list" })).toBeVisible();
  await expect.poll(() => document.querySelector(".deck-list")).toBeNull();

  await places().getByRole("tab", { name: "Cockpit" }).click();
  await expect.poll(label).toBe(was);
  expect(document.querySelector('.deck-row[data-selected]')?.getAttribute("aria-label")).toBe("Pin the store clock");

  // Exactly, to history too: the tab went back to the deck and did not open it a second time, so Back does not find the page again.
  await userEvent.keyboard("{Meta>}[[{/Meta}");
  await new Promise((done) => window.setTimeout(done, 200));
  expect(document.querySelector(".deck-list")).not.toBeNull();
});

test("Checks filters list, Activity and Stage to subjects running a Check", async () => {
  await deck();
  const kept = "Migration notes";
  const dropped = "Pin the store clock";
  const rows = () => [...document.querySelectorAll(".deck-list .deck-row")].map((one) => one.getAttribute("aria-label"));
  await expect.poll(() => rows().includes(dropped)).toBe(true);
  const whole = rows().length;
  // The stage rests on the map or the timeline, whichever a test before left remembered: this asks for the timeline.
  await controls().getByRole("button", { name: "Timeline" }).click();

  // The Activity dots and the stage's timeline keep the same subjects and no others: both name it before the filter, neither after.
  // Activity names each event on its dot.
  const feed = () => [...document.querySelectorAll(".deck-activity .armada-tl__dot")].map((one) => one.getAttribute("aria-label")).join("\n");
  const timeline = () => document.querySelector('.deck-pane[aria-label="Fleet timeline"]')?.textContent ?? "";
  await expect.poll(() => feed().includes(dropped) && timeline().includes(dropped)).toBe(true);

  await controls().getByRole("button", { name: "Checks" }).click();
  await expect.poll(() => rows().includes(dropped)).toBe(false);
  expect(rows()).toContain(kept);
  expect(rows().length).toBeLessThan(whole);
  expect(feed()).not.toContain(dropped);
  expect(timeline()).not.toContain(dropped);
  await controls().getByRole("button", { name: "Checks" }).click();
  await expect.poll(() => rows().length).toBe(whole);
});

// **The owner, 15 Oct 2026**: when navigating the lists, `o` opens the item along with Enter. `open` is the registry's own row
// for it, in the `list` scope the deck's lists are in, so this is the deck reading a key the registry already ships.
test("in the deck's lists, o opens the item under the cursor as Enter does", async () => {
  await deck();
  // What the stage is labelled while nothing is picked.
  const AT_REST = "Flight map";
  const opened = async (selector: string, key: string) => {
    const item = document.querySelector<HTMLElement>(selector);
    expect(item, selector).not.toBeNull();
    item!.focus();
    await userEvent.keyboard(key);
    await expect.poll(() => stage()?.getAttribute("aria-label") ?? "", { message: `${key} on ${selector}` }).toMatch(new RegExp(`^(?!${AT_REST}$).+`));
    const label = stage()!.getAttribute("aria-label");
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => stage()?.getAttribute("aria-label") ?? "").not.toBe(label);
    return label;
  };
  for (const selector of [".deck-list .deck-row", ".deck-bays .deck-bay", ".deck-queue button.deck-chip"]) {
    expect(await opened(selector, "o"), selector).toBe(await opened(selector, "{Enter}"));
  }
  // A control is not an item: `o` on the Autopilot switch leaves it as it was.
  const autopilot = document.querySelector<HTMLElement>('.deck-autopilot [role="switch"]')!;
  const was = autopilot.getAttribute("aria-checked");
  autopilot.focus();
  await userEvent.keyboard("o");
  expect(autopilot.getAttribute("aria-checked")).toBe(was);
  expect(stage()?.getAttribute("aria-label")).toBe(AT_REST);
  // A call opens the Session or Job it is about, as it does when pressed.
  document.querySelector<HTMLElement>(".deck-incoming .deck-hail")!.focus();
  await userEvent.keyboard("o");
  await expect.poll(() => stage()?.getAttribute("aria-label")).not.toBe(AT_REST);
});

// The owner, 15 Oct 2026: nothing could be let go, so the map always held one drone's card.
test("Esc, and a press on the list or the map away from a row or a drone, let the cursor go", async () => {
  await deck();
  await controls().getByRole("button", { name: "Map" }).click();
  const cursored = () => document.querySelectorAll(".deck-list [data-cursor]").length;
  // Read again each time: the list draws its rows afresh.
  const row = () => document.querySelector<HTMLElement>(".deck-list .deck-row")!;
  // A scripted click moves no focus, so the row is let go of first, as a real press elsewhere would.
  const onRow = () => ((document.activeElement as HTMLElement | null)?.blur(), row().focus());

  onRow();
  await expect.poll(cursored).toBe(1);
  await userEvent.keyboard("{Escape}");
  await expect.poll(cursored).toBe(0);
  expect(stage()?.getAttribute("aria-label")).toBe("Flight map");
  expect(document.activeElement?.closest(".deck-row"), "focus leaves the row, or it would be the cursor again").toBeNull();

  onRow();
  await expect.poll(cursored).toBe(1);
  document.querySelector<HTMLElement>(".deck-list__rows")!.click();
  await expect.poll(cursored).toBe(0);

  onRow();
  await expect.poll(cursored).toBe(1);
  document.querySelector<HTMLElement>(".bf-pixi")!.click();
  await expect.poll(cursored).toBe(0);
});
