// Settings → Command palette over the palette's own memory: the table lists the rows that hold a pin, a word or a key, named
// from the rows the palette last listed, and editing or clearing a cell changes what the palette remembers. A word or key the
// palette's panel would refuse is refused here in the same words.

import { PaletteRows } from "@armada/components";
import { mount, unmount } from "@armada/screens/src/mounted";
import { afterEach, beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetPaletteMemory, paletteMemory, PALETTE_KEY, seedPaletteMemory } from "./palette-memory";
import { setKnownRows } from "./palette-known";
import { usePaletteTable } from "./palette-table";

// The palette lists a row with the key it holds as its shortcut, which is what another row's key is refused against.
const KNOWN = [
  { id: "nav:reviews", label: "Reviews", kind: "Page", keyable: true, dormant: false },
  { id: "session:q1", label: "Cache key for the plan windows", kind: "Job", keyable: true, dormant: false },
  { id: "session:q2", label: "Migration notes for Pocket", kind: "Job", keyable: true, dormant: false, shortcut: "⇧⌘J" },
  { id: "nav:untouched", label: "Untouched", kind: "Page", keyable: true, dormant: false },
];

/** The table as Settings draws it: the rows the hook names, under the controls the hook answers. */
function Table() {
  const { rows, onAlias, onHotkey, onPin } = usePaletteTable();
  return <PaletteRows rows={rows} onAlias={onAlias} onHotkey={onHotkey} onPin={onPin} />;
}

beforeEach(() => {
  setKnownRows(KNOWN);
  seedPaletteMemory({ pinned: ["session:q1"], aliases: { "nav:reviews": "rev" }, hotkeys: { "session:q2": "⇧⌘J" } });
});
afterEach(() => {
  unmount();
  setKnownRows([]);
  window.localStorage.removeItem(PALETTE_KEY);
  forgetPaletteMemory();
});

const alias = (name: string) => page.getByRole("textbox", { name: `Alias: ${name}` });

test("it lists the rows that hold a pin, a word or a key, by name, and no others", async () => {
  mount(<Table />);
  const names = ["Cache key for the plan windows", "Migration notes for Pocket", "Reviews"];
  for (const name of names) await expect.element(alias(name)).toBeVisible();
  expect(page.getByRole("textbox", { name: /^Alias: / }).elements()).toHaveLength(3);
  await expect.element(alias("Reviews")).toHaveValue("rev");
  await expect.element(page.getByRole("checkbox", { name: "Pinned: Cache key for the plan windows" })).toBeChecked();
  await expect.element(page.getByRole("button", { name: "Migration notes for Pocket, hotkey: ⇧⌘J" })).toBeVisible();
});

test("a pin, a word or a key kept for a row the palette does not list is left out of the table", async () => {
  seedPaletteMemory({ pinned: ["session:gone"], aliases: { "nav:reviews": "rev" }, hotkeys: {} });
  mount(<Table />);
  await expect.element(alias("Reviews")).toBeVisible();
  expect(page.getByRole("textbox", { name: /^Alias: / }).elements()).toHaveLength(1);
});

test("typing a word and pressing ↵ keeps it in the palette's memory, and its clear lets it go", async () => {
  mount(<Table />);
  await userEvent.fill(alias("Cache key for the plan windows"), "ck");
  await userEvent.keyboard("{Enter}");
  expect(paletteMemory().aliases).toEqual({ "nav:reviews": "rev", "session:q1": "ck" });
  await userEvent.click(page.getByRole("button", { name: "Clear alias: Reviews" }));
  expect(paletteMemory().aliases).toEqual({ "session:q1": "ck" });
});

test("a word that is two, or another row's, is refused beside the field and kept out of memory", async () => {
  mount(<Table />);
  await userEvent.fill(alias("Cache key for the plan windows"), "two words");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByText("One word")).toBeVisible();
  await userEvent.fill(alias("Cache key for the plan windows"), "REV");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByText("Used by Reviews")).toBeVisible();
  expect(paletteMemory().aliases).toEqual({ "nav:reviews": "rev" });
});

test("pressing a key on a row's control keeps it, and its clear lets it go", async () => {
  mount(<Table />);
  await expect.element(page.getByRole("button", { name: "Reviews, hotkey" })).toBeInTheDocument();
  page.getByRole("button", { name: "Reviews, hotkey" }).element().focus();
  await userEvent.keyboard("{Enter}");
  await userEvent.keyboard("{Meta>}{Shift>}l{/Shift}{/Meta}");
  expect(paletteMemory().hotkeys).toEqual({ "session:q2": "⇧⌘J", "nav:reviews": "⇧⌘L" });
  await userEvent.click(page.getByRole("button", { name: "Clear hotkey: Reviews" }));
  expect(paletteMemory().hotkeys).toEqual({ "session:q2": "⇧⌘J" });
});

test("a key the panel would refuse is refused here in its words, and is not kept", async () => {
  mount(<Table />);
  await expect.element(page.getByRole("button", { name: "Reviews, hotkey" })).toBeInTheDocument();
  page.getByRole("button", { name: "Reviews, hotkey" }).element().focus();
  await userEvent.keyboard("{Enter}");
  await userEvent.keyboard("{Meta>}c{/Meta}");
  await expect.element(page.getByRole("alert")).toHaveTextContent("Kept by the system");
  // Another row's key names that row.
  await userEvent.keyboard("{Meta>}{Shift>}j{/Shift}{/Meta}");
  await expect.element(page.getByRole("alert")).toHaveTextContent("Bound to Migration notes for Pocket");
  expect(paletteMemory().hotkeys).toEqual({ "session:q2": "⇧⌘J" });
});

test("the pin checkbox pins and unpins the row in memory", async () => {
  mount(<Table />);
  await userEvent.click(page.getByRole("checkbox", { name: "Pinned: Reviews" }));
  expect(paletteMemory().pinned).toEqual(["session:q1", "nav:reviews"]);
  await userEvent.click(page.getByRole("checkbox", { name: "Pinned: Cache key for the plan windows" }));
  expect(paletteMemory().pinned).toEqual(["nav:reviews"]);
});
