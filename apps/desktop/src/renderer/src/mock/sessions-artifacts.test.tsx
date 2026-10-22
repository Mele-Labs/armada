// An Artifacts row of a Session opens in the side panel, through `App`: a picture, a file's
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

const artifacts = () => page.getByRole("region", { name: "Artifacts", exact: true });
const group = (name: string) => artifacts().getByRole("region", { name, exact: true });

/** The Artifacts key in Flight control opens the layer over the thread; it stays open under a panel a row opens. */
const openArtifacts = () => userEvent.click(page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Artifacts", exact: true }));

test("Artifacts: a picture the Session looked at opens in the panel, fit to it", async () => {
  await onWriteUp();
  await openArtifacts();
  await userEvent.click(group("Files").getByRole("button", { name: "ledger-screenshot.png", exact: true }));
  const panel = page.getByRole("dialog", { name: "ledger-screenshot.png" });
  await expect.element(panel.getByRole("img", { name: "ledger-screenshot.png" })).toBeVisible();
});

test("Artifacts: a file the Session wrote opens in the panel's editor, markdown as its text", async () => {
  await onWriteUp();
  await openArtifacts();
  await userEvent.click(group("Files").getByRole("button", { name: "store-clock.md", exact: true }));
  const panel = page.getByRole("dialog", { name: "store-clock.md" });
  await expect.element(panel.getByRole("group", { name: "Editor store-clock.md", exact: true })).toBeVisible();
});

test("Artifacts: a page opens in a frame with a press for the browser, and a Doc the same", async () => {
  await onWriteUp();
  await openArtifacts();
  await userEvent.click(group("Pages").getByRole("button", { name: "Store clock findings", exact: true }));
  const panel = page.getByRole("dialog", { name: "Store clock findings" });
  await expect.element(panel.getByRole("region", { name: "Page" })).toBeVisible();
  await expect.element(panel.getByTitle("Store clock findings")).toBeVisible();
  await expect.element(panel.getByRole("button", { name: "Open in browser" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(panel).not.toBeInTheDocument();
  await userEvent.click(group("Docs").getByRole("button", { name: "Store clock write-up", exact: true }));
  await expect.element(page.getByRole("dialog", { name: "Store clock write-up" }).getByRole("region", { name: "Page" })).toBeVisible();
});
