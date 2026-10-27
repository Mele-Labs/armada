// A word and a key given to a palette row from its ⌘K panel: Set alias takes a word and ↵ keeps it, Esc backs out, the word
// typed whole puts its row first; Record hotkey takes the next key pressed, and a key refused says why beside the field and
// is not kept. The rules are `palette-refusals.test.ts`'s; this is what a person sees of them.

import { Palette, type PaletteChoice, type PaletteProps } from "@armada/shell";
import { createRoot, type Root } from "react-dom/client";
import { Check } from "lucide-react";
import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const JOBS = [
  { id: "q1", label: "Cache key for the plan windows", aliases: ["q1"], repo: "armada", state: { icon: Check, said: "Working" } },
  { id: "pk1", label: "Migration notes for Pocket", aliases: ["pk1"], repo: "Pocket", state: { icon: Check, said: "Working" } },
];

function palette(over: Partial<PaletteProps> = {}) {
  const chosen: PaletteChoice[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(<Palette open onClose={() => {}} context="board" on={null} surfaces={[]} sessions={JOBS} settings={[]} onChoose={(choice) => chosen.push(choice)} {...over} />);
  return { chosen };
}

/** Typing waits for the search field to hold focus: a key pressed before it does is lost, and the query is cut short. */
async function type(typed: string) {
  await expect.element(page.getByRole("combobox", { name: "Search actions, jobs and settings" })).toHaveFocus();
  await userEvent.keyboard(typed);
}

/** ⌘K on the first Job found by `typed`, and the act pressed in its panel. */
async function act(typed: string, name: string) {
  await type(typed);
  await userEvent.keyboard("{Meta>}k{/Meta}");
  await userEvent.click(page.getByRole("option", { name, exact: true }));
}

const given = (chosen: PaletteChoice[], action: string) => chosen.filter((one) => one.of === "on" && one.action === action);

test("Set alias takes a word, and ↵ hands it on for that row and goes back to the results", async () => {
  const { chosen } = palette();
  await act("cache key", "Set alias");
  await expect.element(page.getByRole("combobox", { name: "Alias" })).toHaveFocus();
  await userEvent.keyboard("ck{Enter}");
  expect(given(chosen, "alias")).toEqual([{ of: "on", target: "session:q1", action: "alias", value: "ck" }]);
  await expect.element(page.getByRole("combobox", { name: "Search actions, jobs and settings" })).toHaveFocus();
  expect(page.getByRole("combobox", { name: "Alias" }).elements()).toHaveLength(0);
});

test("Esc in the alias field backs out to the acts and keeps nothing", async () => {
  const { chosen } = palette();
  await act("cache key", "Set alias");
  await userEvent.keyboard("ck{Escape}");
  await expect.element(page.getByRole("combobox", { name: "Search for actions" })).toBeVisible();
  await expect.element(page.getByRole("option", { name: "Set alias", exact: true })).toBeVisible();
  expect(given(chosen, "alias")).toEqual([]);
});

test("a word of two is refused in the field and not kept, and so is another row's", async () => {
  const { chosen } = palette({ aliases: { "session:pk1": "mig" } });
  await act("cache key", "Set alias");
  await userEvent.keyboard("two words{Enter}");
  await expect.element(page.getByRole("alert")).toHaveTextContent("One word");
  await page.getByRole("combobox", { name: "Alias" }).fill("MIG");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("alert")).toHaveTextContent("Used by Migration notes for Pocket");
  expect(given(chosen, "alias")).toEqual([]);
  // Still taking the word: the person can try another.
  await expect.element(page.getByRole("combobox", { name: "Alias" })).toBeVisible();
});

test("a word typed whole puts its row first, ahead of a Job that matches by title", async () => {
  palette({ aliases: { "session:pk1": "cache" } });
  await type("cache");
  // Retried: read at once, the rows can still be the ones a shorter query drew.
  await expect.element(page.getByRole("option").nth(0)).toHaveTextContent("Migration notes");
  await expect.element(page.getByRole("option").nth(1)).toHaveTextContent("Cache key");
});

test("a row that holds a word offers Remove alias, which hands it on", async () => {
  const { chosen } = palette({ aliases: { "session:q1": "ck" } });
  await type("cache key");
  await userEvent.keyboard("{Meta>}k{/Meta}");
  await userEvent.click(page.getByRole("option", { name: /^Remove alias/ }));
  expect(given(chosen, "remove-alias")).toEqual([{ of: "on", target: "session:q1", action: "remove-alias" }]);
});

test("Record hotkey takes the next chord, and hands it on spelled as the palette draws keys", async () => {
  const { chosen } = palette();
  await act("cache key", "Record hotkey");
  await expect.element(page.getByRole("combobox", { name: "Press a key" })).toHaveFocus();
  await userEvent.keyboard("{Meta>}{Shift>}j{/Shift}{/Meta}");
  expect(given(chosen, "hotkey")).toEqual([{ of: "on", target: "session:q1", action: "hotkey", value: "⇧⌘J" }]);
});

// Each key is pressed in the recording field; the reason is said beside it and nothing is kept.
const REFUSED: [string, string, RegExp][] = [
  ["a key without ⌘ or ⌃", "j", /^Needs ⌘ or ⌃$/],
  ["⌘1 to ⌘9", "{Meta>}3{/Meta}", /^⌘1 to ⌘9 open results$/],
  ["a key an act answers", "{Meta>}k{/Meta}", /^Bound to /],
  ["a key the system keeps", "{Meta>}c{/Meta}", /^Kept by the system$/],
];
for (const [what, press, why] of REFUSED) {
  test(`${what} is refused with its reason beside the field, and the key is not kept`, async () => {
    const { chosen } = palette();
    await act("cache key", "Record hotkey");
    await userEvent.keyboard(press);
    await expect.element(page.getByRole("alert")).toHaveTextContent(why);
    expect(given(chosen, "hotkey")).toEqual([]);
    // The field goes on waiting for a key that will do.
    await expect.element(page.getByRole("combobox", { name: "Press a key" })).toBeVisible();
  });
}

test("another row's key is refused naming that row, and a free one pressed after it is kept", async () => {
  const { chosen } = palette({ hotkeys: { "session:pk1": "⇧⌘J" } });
  await act("cache key", "Record hotkey");
  await userEvent.keyboard("{Meta>}{Shift>}j{/Shift}{/Meta}");
  await expect.element(page.getByRole("alert")).toHaveTextContent("Bound to Migration notes for Pocket");
  expect(given(chosen, "hotkey")).toEqual([]);
  await userEvent.keyboard("{Meta>}{Shift>}l{/Shift}{/Meta}");
  expect(given(chosen, "hotkey")).toEqual([{ of: "on", target: "session:q1", action: "hotkey", value: "⇧⌘L" }]);
});

test("a row that holds a key offers Remove hotkey, which hands it on", async () => {
  const { chosen } = palette({ hotkeys: { "session:q1": "⇧⌘J" } });
  await type("cache key");
  await userEvent.keyboard("{Meta>}k{/Meta}");
  await userEvent.click(page.getByRole("option", { name: /^Remove hotkey/ }));
  expect(given(chosen, "remove-hotkey")).toEqual([{ of: "on", target: "session:q1", action: "remove-hotkey" }]);
});
