// Settings → Command palette: the table of rows that hold an alias, a hotkey or a pin, each cell edited where it stands.
// An alias is a field (↵ keeps, Esc puts it back), a hotkey takes the next key pressed, and a word or key refused is said
// beside its cell and kept for another try. What `onAlias` and `onHotkey` refuse is the app's; here they refuse by name.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, unmount } from "@armada/screens/src/mounted";

import { PaletteSettings } from "./PaletteSettings";

afterEach(unmount);

const ROWS = [
  { id: "nav:reviews", name: "Reviews", kind: "Page", alias: "rev", hotkey: "⇧⌘J", pinned: true, keyable: true },
  { id: "session:q1", name: "Cache key for the plan windows", kind: "Job", alias: "", hotkey: "", pinned: false, keyable: true },
  { id: "session:q2", name: "Stop everything", kind: "Job", alias: "", hotkey: "", pinned: false, keyable: false },
];

function table(refuse: { alias?: string; hotkey?: string } = {}) {
  const onAlias = vi.fn((_id: string, _word: string) => refuse.alias);
  const onHotkey = vi.fn((_id: string, _spelled: string) => refuse.hotkey);
  const onPin = vi.fn();
  mount(<PaletteSettings opens="pinned" onOpens={() => {}} rows={ROWS} onAlias={onAlias} onHotkey={onHotkey} onPin={onPin} />);
  return { onAlias, onHotkey, onPin };
}

const alias = (name: string) => page.getByRole("textbox", { name: `Alias: ${name}` });
const key = (name: string) => page.getByRole("button", { name: new RegExp(`^${name}, hotkey`) });

/** The control for a row's key, pressed from the keyboard: a row that holds no key draws it empty. */
async function record(name: string): Promise<void> {
  await expect.element(key(name)).toBeInTheDocument();
  key(name).element().focus();
  await userEvent.keyboard("{Enter}");
}

test("every row is listed with what it holds", async () => {
  table();
  await expect.element(page.getByRole("table", { name: "Command palette rows" })).toBeVisible();
  await expect.element(alias("Reviews")).toHaveValue("rev");
  await expect.element(page.getByRole("checkbox", { name: "Pinned: Reviews" })).toBeChecked();
  await expect.element(page.getByRole("checkbox", { name: "Pinned: Cache key for the plan windows" })).not.toBeChecked();
});

test("↵ in the alias field keeps the word, trimmed", async () => {
  const { onAlias } = table();
  await userEvent.fill(alias("Cache key for the plan windows"), "  ck ");
  await userEvent.keyboard("{Enter}");
  expect(onAlias).toHaveBeenCalledWith("session:q1", "ck");
});

test("Esc puts the field back to what is kept, and keeps nothing", async () => {
  const { onAlias } = table();
  await userEvent.fill(alias("Reviews"), "other");
  await userEvent.keyboard("{Escape}");
  await expect.element(alias("Reviews")).toHaveValue("rev");
  expect(onAlias).not.toHaveBeenCalled();
});

test("a word refused is said under the field and stays typed for another try", async () => {
  const { onAlias } = table({ alias: "Used by Reviews" });
  await userEvent.fill(alias("Cache key for the plan windows"), "rev");
  await userEvent.keyboard("{Enter}");
  expect(onAlias).toHaveBeenCalledWith("session:q1", "rev");
  await expect.element(page.getByText("Used by Reviews")).toBeVisible();
  await expect.element(alias("Cache key for the plan windows")).toHaveValue("rev");
});

test("the clear beside a word lets it go", async () => {
  const { onAlias } = table();
  await userEvent.click(page.getByRole("button", { name: "Clear alias: Reviews" }));
  expect(onAlias).toHaveBeenCalledWith("nav:reviews", "");
});

test("a hotkey control takes the next key pressed, as the palette spells it, and then stops listening", async () => {
  const { onHotkey } = table();
  await record("Cache key for the plan windows");
  await expect.element(page.getByRole("button", { name: /press a key/ })).toBeVisible();
  await userEvent.keyboard("{Meta>}{Shift>}l{/Shift}{/Meta}");
  expect(onHotkey).toHaveBeenCalledWith("session:q1", "⇧⌘L");
  await expect.element(page.getByRole("button", { name: /press a key/ })).not.toBeInTheDocument();
});

test("a key refused is said beside the control, which goes on waiting, and the key is not kept", async () => {
  const { onHotkey } = table({ hotkey: "Needs ⌘ or ⌃" });
  await record("Cache key for the plan windows");
  await userEvent.keyboard("j");
  await expect.element(page.getByRole("alert")).toHaveTextContent("Needs ⌘ or ⌃");
  await expect.element(page.getByRole("button", { name: /press a key/ })).toBeVisible();
  expect(onHotkey).toHaveBeenCalledTimes(1);
  expect(onHotkey.mock.calls[0]?.[0]).toBe("session:q1");
});

test("Esc lets go of the control without a key, and Delete unbinds", async () => {
  const { onHotkey } = table();
  await record("Reviews");
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("button", { name: /press a key/ })).not.toBeInTheDocument();
  expect(onHotkey).not.toHaveBeenCalled();
  await record("Reviews");
  await userEvent.keyboard("{Delete}");
  expect(onHotkey).toHaveBeenCalledWith("nav:reviews", "");
});

test("a row that ends work takes no key, and a pin is pressed in its checkbox", async () => {
  const { onPin } = table();
  await expect.element(alias("Stop everything")).toBeVisible();
  expect(page.getByRole("button", { name: /^Stop everything, hotkey/ }).elements()).toHaveLength(0);
  await userEvent.click(page.getByRole("checkbox", { name: "Pinned: Reviews" }));
  expect(onPin).toHaveBeenCalledWith("nav:reviews");
});
