// An Artifacts row of a Session's ledger opens in the side panel, through `App`: a picture, a file's
// text rendered, and a page in a frame. The walk `session-artifacts-panel` plays the same presses from the Artifacts layer; this
// holds what each shows.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

async function onWriteUp(): Promise<void> {
  mount("session-ledger");
  await onScreen();
  await toSession("Write up the store clock");
}

const ledger = () => page.getByRole("region", { name: "Attachments" });

test("Artifacts: a picture the Session looked at opens in the panel, fit to it", async () => {
  await onWriteUp();
  await userEvent.click(ledger().getByRole("button", { name: "Pictures" }));
  await userEvent.click(ledger().getByRole("button", { name: "Looked at ledger-screenshot.png" }));
  const panel = page.getByRole("dialog", { name: "ledger-screenshot.png" });
  await expect.element(panel.getByRole("img", { name: "ledger-screenshot.png" })).toBeVisible();
});

test("Artifacts: a file the Session wrote opens in the panel's editor, markdown as its text", async () => {
  await onWriteUp();
  await userEvent.click(ledger().getByRole("button", { name: "File written store-clock.md" }));
  const panel = page.getByRole("dialog", { name: "store-clock.md" });
  await expect.element(panel.getByRole("group", { name: "Editor store-clock.md", exact: true })).toBeVisible();
});

test("Artifacts: a page opens in a frame with a press for the browser, and a Doc the same", async () => {
  await onWriteUp();
  await userEvent.click(ledger().getByRole("button", { name: "Published page Store clock findings" }));
  const panel = page.getByRole("dialog", { name: "Store clock findings" });
  await expect.element(panel.getByRole("region", { name: "Page" })).toBeVisible();
  await expect.element(panel.getByTitle("Store clock findings")).toBeVisible();
  await expect.element(panel.getByRole("button", { name: "Open in browser" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(panel).not.toBeInTheDocument();
  await userEvent.click(ledger().getByRole("button", { name: "Doc Store clock write-up" }));
  await expect.element(page.getByRole("dialog", { name: "Store clock write-up" }).getByRole("region", { name: "Page" })).toBeVisible();
});
