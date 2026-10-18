// The deck's keys are the registry's: the key that goes to a panel is a slot of `deck_panel`, the sheet is drawn from the acts'
// own verbs, and an act that is not built is not on it.

import { ACTIONS, formatSlot, slotsOf } from "@armada/components";
import { expect, test } from "vitest";

import { PANELS, PANEL_SLOTS, keySheetOf, panelKey } from "./keys-model";

test("each panel's key is its slot of deck_panel, and a panel with no slot has no key", () => {
  const slots = slotsOf("deck_panel").map(formatSlot);
  expect(slots).toEqual(["l", "b", "c", "q", "f", "a", "d"]);
  expect(PANEL_SLOTS.map(panelKey)).toEqual(slots);
  expect(panelKey("activity")).toBe("");
  expect(panelKey("stage")).toBe("");
});

test("every panel the registry's keys go to is a panel the deck draws", () => {
  // `c` is the one key that is no panel: it goes to the first row of the list that needs him.
  expect(PANEL_SLOTS.every((id) => id === "needs" || PANELS.some((one) => one.id === id))).toBe(true);
  expect(panelKey("needs")).toBe("c");
  expect(keySheetOf().flatMap((group) => group.rows).find((row) => row.keys.includes("c"))?.does).toBe("Needs you");
  expect(PANEL_SLOTS).toHaveLength(slotsOf("deck_panel").length);
});

test("the key sheet lists every deck act with the keymap's own keys, and the map's and the replay's", () => {
  const rows = keySheetOf().flatMap((group) => group.rows);
  const keys = rows.flatMap((row) => row.keys);
  for (const act of ACTIONS.filter((one) => one.scope === "deck" && one.unbuilt === null)) {
    for (const slot of slotsOf(act.id).map(formatSlot)) expect(keys, `${act.id} on the sheet`).toContain(slot);
  }
  // Nothing the deck takes is left not built: the battlefield was the last of it.
  expect(ACTIONS.filter((one) => one.scope === "deck" && one.unbuilt !== null).map((one) => one.id)).toEqual([]);
  // The map is the Dashboard's `m`, read by the deck.
  expect(keys).toContain(formatSlot(slotsOf("dashboard_view")[0]!));
  expect(keySheetOf().map((group) => group.head)).toEqual(expect.arrayContaining(["Map", "Replay"]));
});

test("the page chord lists its three second keys after the registry's first", () => {
  const first = formatSlot(slotsOf("deck_page")[0]!);
  const pages = keySheetOf()
    .find((group) => group.head === "Pages")!
    .rows.filter((row) => row.keys[0] === first);
  expect(pages.map((row) => row.keys[1])).toEqual(["c", "s", "y"]);
});

test("the sheet lists Open with Enter and with o, and Settings with its own key", () => {
  const rows = keySheetOf().flatMap((group) => group.rows);
  expect(rows.find((row) => row.does === "Open")?.keys).toEqual([...slotsOf("open_focused").map(formatSlot), ...slotsOf("open").map(formatSlot)]);
  expect(slotsOf("open").map(formatSlot)).toEqual(["o"]);
  expect(rows.find((row) => row.does === "Open Settings")?.keys).toEqual(["⌘,"]);
});
