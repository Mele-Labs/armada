// A link pasted on the Cockpit goes to the page it names: a pull request to Reviews, an issue to the side panel, and any
// other link to the link panel. On the `paste-a-pr-link` scenario, through `App`. Hand-run with the rest of `mock/`.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { s270PasteAPrLink } from "./scenarios/paste-a-pr-link";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

function paste(text: string) {
  const clipboard = new DataTransfer();
  clipboard.setData("text/plain", text);
  document.body.dispatchEvent(new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true }));
}

test("a pull request's link pasted on the Cockpit opens Reviews on it", async () => {
  mount(s270PasteAPrLink);
  await onScreen();
  paste("https://forge.example/mele-labs/armada/pull/1899");
  await expect.element(page.getByRole("region", { name: "Pull request #1899" })).toBeVisible();
});

test("an issue's link pasted on the Cockpit opens the side panel on it", async () => {
  mount(s270PasteAPrLink);
  await onScreen();
  paste("https://forge.example/mele-labs/armada/issues/2214");
  await expect.element(page.getByRole("dialog", { name: "Issue #2214" })).toBeVisible();
});

test("any other link still opens the link panel", async () => {
  mount(s270PasteAPrLink);
  await onScreen();
  paste("https://impeccable.style/");
  await expect.element(page.getByRole("dialog", { name: "Add a link" })).toBeVisible();
});
