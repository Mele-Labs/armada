// A Session id in a thread links to that Session, written in backticks or plain, and an ended Session's
// too. Code that is more than the id stays code. Through `App`; the walk `session-id-links` draws it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { s208SessionIdLinks } from "./scenarios/session-id-links";

unmountAfterEach();

const thread = () => page.getByRole("region", { name: "Thread" });

test("a Session address in backticks, and one in plain text, each open that Session", async () => {
  mount(s208SessionIdLinks);
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: /Hand-off thread/ }));

  // Code that is more than an address stays code.
  await expect.element(thread().getByText("s-e165b4df --fix")).toBeVisible();
  await expect.element(thread().getByRole("link", { name: "s-e165b4df --fix" })).not.toBeInTheDocument();

  // The ended Session still has its page, so its address is a link.
  await userEvent.click(thread().getByRole("link", { name: "s-b7d20c9e" }));
  await expect.element(thread().getByText("Nothing more to add.")).toBeVisible();
});

test("an address in backticks links and opens its Session", async () => {
  mount(s208SessionIdLinks);
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: /Hand-off thread/ }));
  await userEvent.click(thread().getByRole("link", { name: "s-e165b4df", exact: true }));
  await expect.element(thread().getByText("Picking this up from")).toBeVisible();
});
