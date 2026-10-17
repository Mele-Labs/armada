// A command a Session gave the person runs in a docked terminal in the Session's view, from the block in
// its thread or from the item in Needs you: the same pane, one run at a time, and a pane put away comes
// back from the block's mark. Through `App` on `session-run-in-terminal`, whose program is a fake
// wrangler that reads the keys it is typed. The whole story is the walk `run-in-terminal-docked`.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { s256SessionRunInTerminal } from "./scenarios/session-run-in-terminal";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const COMMAND = /wrangler deploy/;
const pane = () => page.getByRole("group", { name: "Terminal", exact: true });
const needsYou = () => page.getByRole("region", { name: "Needs you" });
const thread = () => page.getByRole("region", { name: "Thread" });

async function open() {
  mount(s256SessionRunInTerminal);
  await onScreen();
  await toSession(/Deploy the store/);
  await expect.element(thread()).toBeVisible();
}

const focused = () => document.activeElement?.getAttribute("aria-label");

test("Run on the item in Needs you opens the pane above the message box, with the keys in it", async () => {
  await open();
  await needsYou().getByRole("button", { name: /^Needs you/ }).click();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  await needsYou().getByRole("group", { name: COMMAND }).getByRole("button", { name: "Run", exact: true }).click();

  await expect.element(pane()).toBeVisible();
  expect(document.querySelectorAll(".armada-terminal")).toHaveLength(1);
  await expect.element(pane().getByText("Attempting to login via OAuth...")).toBeVisible();
  await expect.poll(focused).toBe("Terminal input");
  const box = page.getByRole("textbox", { name: "Message" }).element().getBoundingClientRect();
  const docked = pane().element().getBoundingClientRect();
  expect(docked.bottom).toBeLessThanOrEqual(box.top);

  // What is typed reaches the program: it asks at its first prompt, and echoes the key.
  await expect.element(pane().getByText("Would you like to register a workers.dev subdomain now?")).toBeVisible();
  await userEvent.keyboard("n");
  await expect.element(pane().getByText(/Would you like to register.*›.*\(Y\/n\).*n/)).toBeVisible();
});

test("Run on the block in the thread opens the same pane, Stop ends it, and Run again starts one afresh", async () => {
  await open();
  await thread().getByRole("group", { name: COMMAND }).getByRole("button", { name: "Run", exact: true }).click();
  await expect.element(pane()).toBeVisible();
  await expect.element(pane().getByText("Attempting to login via OAuth...")).toBeVisible();

  await pane().getByRole("button", { name: "Stop" }).click();
  await expect.element(pane().getByRole("img", { name: "Stopped" })).toBeVisible();
  await pane().getByRole("button", { name: "Run again" }).click();
  await expect.element(pane().getByRole("img", { name: "Running" })).toBeVisible();
  expect(document.querySelectorAll(".armada-terminal")).toHaveLength(1);
  // The screen began again: the first run's prompt is not on it twice.
  await expect.element(pane().getByText("Attempting to login via OAuth...")).toBeVisible();
  expect(pane().element().textContent?.split("Attempting to login via OAuth...")).toHaveLength(2);

  await pane().getByRole("button", { name: "Stop" }).click();
  await pane().getByRole("button", { name: "Close" }).click();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  await thread().getByRole("button", { name: "Show terminal" }).click();
  await expect.element(pane().getByRole("img", { name: "Stopped" })).toBeVisible();
});
