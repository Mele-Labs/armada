// A Kit item's file is its detail: the skill's SKILL.md is in the editor, edited and saved where it is read, through the
// same container the side panel uses. A plugin is a folder and offers Show in Finder; what it brings opens in the panel and
// leaves the plugin page where it is. Through `App` on the `kit-edits-in-place` scenario.

import { typeIntoEditor } from "@armada/components";
import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSystemPage, unmountAfterEach } from "./testing";

unmountAfterEach();

const SKILL = "/Users/user/.agent/skills/humanizer/SKILL.md";
const PLUGIN = "/Users/user/.agent/plugins/cache/armada/1.0.0";

const article = (name: string) => page.getByRole("article", { name, exact: true });
const dialog = (name: string) => page.getByRole("dialog", { name, exact: true });

async function pick(entry: string, row: RegExp): Promise<void> {
  mount("kit-edits-in-place");
  await onScreen();
  await toSystemPage(entry);
  await userEvent.click(page.getByRole("region", { name: entry, exact: true }).getByRole("button", { name: row }));
}

/** The walk's own way of typing: through the editor's view, which a test cannot reach into a shadow root for. */
function typed(file: string, words: string): void {
  const host = document.querySelector<HTMLElement>(`[aria-label="Editor ${file}"]`);
  expect(host).not.toBeNull();
  expect(typeIntoEditor(host!, words)).toBe(true);
}

test("a skill's detail is its SKILL.md in the editor, with no Open button", async () => {
  await pick("Skills", /^humanizer/);
  await expect.element(article("humanizer").getByRole("group", { name: "Editor SKILL.md", exact: true })).toBeVisible();
  expect(article("humanizer").getByRole("button", { name: "Open", exact: true }).elements()).toHaveLength(0);
  await expect.element(article("humanizer").getByRole("button", { name: "Open in your editor" })).toBeVisible();
  await expect.element(article("humanizer").getByRole("button", { name: "Show in Finder" })).toBeVisible();
});

test("typing offers Save, and Save writes the file as a Kit file and goes away", async () => {
  await pick("Skills", /^humanizer/);
  const save = article("humanizer").getByRole("button", { name: /^Save/ });
  expect(save.elements()).toHaveLength(0);
  const saving = vi.spyOn(window.armada, "savePanelFile");
  typed("SKILL.md", "@8 \n\nRead it aloud.");
  await expect.element(save).toBeVisible();
  await userEvent.click(save);
  await expect.element(save).not.toBeInTheDocument();
  expect(saving).toHaveBeenCalledTimes(1);
  expect(saving.mock.calls[0]?.[0]).toEqual({ kind: "file", path: SKILL, from: "kit" });
  expect(saving.mock.calls[0]?.[1]).toContain("Read it aloud.");
  saving.mockRestore();
});

test("a plugin is a folder: Show in Finder alone, naming the folder", async () => {
  await pick("Plugins", /^armada/);
  const opened = vi.spyOn(window.armada, "openKitFile");
  const here = article("armada");
  await expect.element(here.getByRole("button", { name: "Show in Finder" })).toBeVisible();
  expect(here.getByRole("group", { name: /^Editor / }).elements()).toHaveLength(0);
  expect(here.getByRole("button", { name: "Open", exact: true }).elements()).toHaveLength(0);
  await userEvent.click(here.getByRole("button", { name: "Show in Finder" }));
  expect(opened.mock.calls).toEqual([[PLUGIN, "finder"]]);
  opened.mockRestore();
});

test("a press on what a plugin brings opens its file in the side panel and stays on the plugin", async () => {
  await pick("Plugins", /^armada/);
  await userEvent.click(article("armada").getByRole("button", { name: /^implementer/ }));
  await expect.element(dialog("implementer.md").getByRole("group", { name: "Editor implementer.md", exact: true })).toBeVisible();
  await expect.element(article("armada")).toBeVisible();
  await expect.element(article("armada").getByRole("button", { name: /^implementer/ })).toBeVisible();
});

test("Enter on a brought row opens its file", async () => {
  await pick("Plugins", /^armada/);
  article("armada").getByRole("button", { name: /^explorer/ }).element().focus();
  await userEvent.keyboard("{Enter}");
  await expect.element(dialog("explorer.md").getByRole("group", { name: "Editor explorer.md", exact: true })).toBeVisible();
});
