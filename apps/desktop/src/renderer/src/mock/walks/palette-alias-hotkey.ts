// A word and a key for a palette row, given in two places: ⌘K on the row inside the palette, and a table in Settings.
// Mock only. Over the scenario of the palette like Raycast's.

import { dialog, inside, role, text, toSystemPage, walk } from "../walk";
import type { Step } from "../walk";

const places = role("navigation", "Places");
const palette = dialog("Command palette");
const bar = inside(palette, role("combobox", "Search actions, jobs and settings"));
const alias = inside(palette, role("combobox", "Alias"));
const record = inside(palette, role("combobox", "Press a key"));

function steps(): Step[] {
  return [
    { key: "Meta+k", on: places, say: "⌘K" },
    { type: "plan windows", into: bar, say: "A pull request" },
    { key: "Meta+k", on: bar, say: "⌘K on it. Apart, below its own acts, are Set alias and Record hotkey, on every row that opens" },
    { press: role("option", "Set alias", { exact: true }), say: "Set alias. The field at the foot takes the word. Enter keeps it and Esc backs out" },
    { type: "cache\n", into: alias, say: "A word" },
    { type: "cache", into: bar, say: "The word typed whole puts its row first, ahead of the Job that matches it by title" },
    { key: "Meta+k", on: bar, say: "⌘K on that row again. It holds the word now, and Remove alias lets it go" },
    { press: role("option", "Record hotkey", { exact: true }), say: "Record hotkey. The field waits for the next key" },
    { key: "Meta+k", on: record, say: "⌘K is refused, and says which act has it. So is any key without ⌘ or ⌃, ⌘1 to ⌘9, or another row's" },
    { key: "Meta+Shift+j", on: record, say: "⇧⌘J is free, so it is kept" },
    { key: "Escape", on: bar, say: "Esc closes the palette" },
    { key: "Meta+Shift+j", on: places, say: "The key runs the row from anywhere in Bridge, as Enter would: Reviews, on #1899" },
    { look: role("region", "Pull request #1899"), say: "Reviews" },
    ...toSystemPage("Command palette", "System, Command palette: what the empty search shows, then a table of the rows with a word, a key or a pin"),
    { look: role("table", "Command palette rows"), say: "A row per alias, hotkey or pin. Each cell is edited where it stands, the clear sits in the same place on every row, and ↑ ↓ move down a column" },
    { look: text("history"), say: "The ones given before the window opened are here too" },
  ];
}

const paletteAliasHotkey = walk("palette-like-raycast", steps());

export { paletteAliasHotkey as "palette-alias-hotkey" };
