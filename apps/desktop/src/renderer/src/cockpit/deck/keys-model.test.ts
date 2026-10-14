// The deck's keys are the registry's: the key that goes to a panel is a slot of `deck_panel`, the sheet is drawn from the acts'
// own verbs, and an act that is not built is not on it.

import { ACTIONS, formatSlot, slotsOf } from "@armada/components";
import { expect, test } from "vitest";

import { PANELS, PANEL_SLOTS, keySheetOf, panelKey } from "./keys-model";

test("each panel's key is its slot of deck_panel, and a panel with no slot has no key", () => {
  const slots = slotsOf("deck_panel").map(formatSlot);
  expect(slots).toEqual(["l", "b", "c", "q", "f", "a"]);
  expect(PANEL_SLOTS.map(panelKey)).toEqual(slots);
  expect(panelKey("activity")).toBe("");
  expect(panelKey("stage")).toBe("");
});

test("every panel the registry's keys go to is a panel the deck draws", () => {
  expect(PANEL_SLOTS.every((id) => PANELS.some((one) => one.id === id))).toBe(true);
  expect(PANEL_SLOTS).toHaveLength(slotsOf("deck_panel").length);
});

test("the key sheet lists the built deck acts with the keymap's own keys, and no act that is not built", () => {
  const rows = keySheetOf().flatMap((group) => group.rows);
  const keys = rows.flatMap((row) => row.keys);
  for (const act of ACTIONS.filter((one) => one.scope === "deck" && one.unbuilt === null)) {
    for (const slot of slotsOf(act.id).map(formatSlot)) expect(keys, `${act.id} on the sheet`).toContain(slot);
  }
  const unbuilt = ACTIONS.filter((one) => one.scope === "deck" && one.unbuilt !== null);
  expect(unbuilt.map((one) => one.id)).toEqual(["deck_view"]);
  expect(rows.map((row) => row.does)).not.toContain(unbuilt[0]!.verb);
});

test("the page chord lists its four second keys after the registry's first", () => {
  const first = formatSlot(slotsOf("deck_page")[0]!);
  const pages = keySheetOf()
    .find((group) => group.head === "Pages")!
    .rows.filter((row) => row.keys[0] === first);
  expect(pages.map((row) => row.keys[1])).toEqual(["c", "j", "s", "y"]);
});
