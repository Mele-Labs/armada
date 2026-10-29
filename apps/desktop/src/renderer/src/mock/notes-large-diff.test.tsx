// A note written on a large pull request's Files tab keeps up with the keys. Each diff line carries a tooltip, so a diff of a few
// thousand lines is a few thousand elements, and every key in the note box once restyled all of them: a quarter of a second a
// character. A line off screen is not styled, so a key restyles only what the owner can see.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("the lines of a large diff are not styled while they are off screen, and typing a note is not a restyle of all of them", async () => {
  await page.viewport(1440, 900);
  mount("shared-side-panel");
  await onScreen();
  await page.getByRole("region", { name: "Merge queue" }).getByRole("button", { name: /^#1890/ }).click();
  const pull = page.getByRole("dialog", { name: "Pull request #1890" });
  await pull.getByRole("tab", { name: "Files" }).click();
  await expect.element(pull.getByRole("button", { name: /^Note for the job on \+\s+marker: Option<String>,/ })).toBeInTheDocument();

  const lines = [...document.querySelectorAll<HTMLElement>(".armada-diff__line")];
  // The panel cuts a diff at about two thousand lines, which is still more than a restyle of all of them can be quick over.
  expect(lines.length).toBeGreaterThan(2000);
  expect(getComputedStyle(lines[lines.length - 1]!).contentVisibility).toBe("auto");

  await pull.getByRole("button", { name: /^Note for the job on \+\s+marker: Option<String>,/ }).click();
  const note = pull.getByRole("textbox", { name: "Note for the job" });
  await note.click();
  const words = "Why an Option here, when every pause has a marker?";
  const from = performance.now();
  await userEvent.keyboard(words);
  const each = (performance.now() - from) / words.length;
  // 15ms a key where the lines are skipped, over 300ms where they are not: the bound sits well clear of both.
  expect(each).toBeLessThan(120);
});
