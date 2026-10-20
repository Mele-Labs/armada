// A file opens in the side panel's editor from each press that names one, through `App` on the `file-panel` scenario: Settings'
// Open settings.json, ⌘O, and a Session's show_file row. A Session's file artifact is `sessions-artifacts.test.tsx`. An edit
// is kept by path across closing the panel, a save clears it, and a save the disk has moved past asks which stands.

import { typeIntoEditor } from "@armada/components";
import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, toSystemPage, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const dialog = (name: string) => page.getByRole("dialog", { name, exact: true });
const editor = (name: string) => dialog(name).getByRole("group", { name: `Editor ${name}`, exact: true });
const unsaved = () => dialog("settings.json").getByRole("img", { name: "Unsaved" });

async function onSettingsFile(): Promise<void> {
  mount("file-panel");
  await onScreen();
  await toSystemPage("Settings");
  await userEvent.click(page.getByRole("button", { name: "Open settings.json" }));
  await expect.element(editor("settings.json")).toBeVisible();
}

/** The walk's own way of typing: through the editor's view, which a test cannot reach into a shadow root for. */
function typed(words: string): void {
  const host = document.querySelector<HTMLElement>('[aria-label="Editor settings.json"]');
  expect(host).not.toBeNull();
  expect(typeIntoEditor(host!, words)).toBe(true);
}

test("Open settings.json opens the panel on the settings path", async () => {
  await onSettingsFile();
  await expect.element(dialog("settings.json").getByText("~/Library/Application Support/Armada/settings.json")).toBeInTheDocument();
});

test("⌘O opens the file the dialog picked", async () => {
  mount("file-panel");
  await onScreen();
  await userEvent.keyboard("{Meta>}o{/Meta}");
  await expect.element(editor("clock.rs")).toBeVisible();
});

test("a show_file row in the thread opens the file in the panel", async () => {
  mount("file-panel");
  await onScreen();
  await toSession("Store clock notes");
  await userEvent.click(page.getByRole("region", { name: "Thread" }).getByRole("button", { name: "Opened store-clock.md" }));
  await expect.element(editor("store-clock.md")).toBeVisible();
});

test("the ledger says a file the Session only showed was shown, and one it wrote was written, shown or not", async () => {
  mount("file-panel");
  await onScreen();
  await toSession("Store clock notes");
  const ledger = page.getByRole("region", { name: "Attachments" });
  await expect.element(ledger.getByRole("button", { name: "File shown store-clock.md" })).toBeVisible();
  await expect.element(ledger.getByRole("button", { name: "File written clock.rs" })).toBeVisible();
});

test("an edit survives closing the panel on it, and a save clears it", async () => {
  await onSettingsFile();
  typed('@9 ,\n  "sounds": false');
  await expect.element(unsaved()).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(dialog("settings.json")).not.toBeInTheDocument();
  await userEvent.click(page.getByRole("button", { name: "Open settings.json" }));
  await expect.element(unsaved()).toBeVisible();
  await userEvent.keyboard("{Meta>}s{/Meta}");
  await expect.element(unsaved()).not.toBeInTheDocument();
  await userEvent.keyboard("{Escape}");
  await userEvent.click(page.getByRole("button", { name: "Open settings.json" }));
  await expect.element(editor("settings.json")).toBeVisible();
  await expect.element(unsaved()).not.toBeInTheDocument();
});

test("a save the disk has moved past shows both, and Keep mine saves over what is there", async () => {
  await onSettingsFile();
  typed('@9 ,\n  "sounds": false');
  timePasses();
  await userEvent.keyboard("{Meta>}s{/Meta}");
  await expect.element(dialog("settings.json").getByText("Changed on disk")).toBeVisible();
  await expect.element(dialog("settings.json").getByRole("group", { name: "Editor settings.json on disk" })).toBeVisible();
  await userEvent.click(dialog("settings.json").getByRole("button", { name: "Keep mine" }));
  await expect.element(dialog("settings.json").getByText("Changed on disk")).not.toBeInTheDocument();
  await expect.element(unsaved()).not.toBeInTheDocument();
});
