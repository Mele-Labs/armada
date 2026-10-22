// A Job's Field while it messages another Job: an arc and its quote only while the message is fresh, the strip's words for the sender and
// the receiver, and a press on the arc or its quote opens the other Job. Over the `job-field-messaging` scenario.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const field = () => page.getByRole("region", { name: "Field", exact: true }).first();
const map = () => field().getByRole("listbox", { name: "Map" });
const main = () => field().getByRole("group", { name: "Drones" }).getByRole("button", { name: /^Rotate the audit log: / });
const message = (name: RegExp) => map().getByRole("button", { name });

async function open() {
  mount("job-field-messaging");
  await onScreen();
  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Rotate the audit log" }).click();
  await expect.element(main()).toBeVisible();
}

test("the arc and its quote show only while the message is fresh, and the strip names the other Job for the sender and the receiver", async () => {
  await open();
  await expect.element(main()).toHaveAccessibleName("Rotate the audit log: Thinking");
  await expect.element(message(/^Message (to|from) /)).not.toBeInTheDocument();

  timePasses();
  await expect.element(main()).toHaveAccessibleName("Rotate the audit log: Messaging Rebuild the store index");
  await expect.element(message(/^Message to Rebuild the store index: Is the store index rebuilt before rotation runs\?$/)).toBeInTheDocument();

  timePasses();
  await expect.element(main()).toHaveAccessibleName("Rotate the audit log: Messaged by Rebuild the store index");
  await expect.element(message(/^Message from Rebuild the store index: Yes, at startup, before any rotation$/)).toBeInTheDocument();
  await expect.element(message(/^Message to /)).not.toBeInTheDocument();

  timePasses();
  await expect.element(main()).toHaveAccessibleName("Rotate the audit log: Thinking");
  await expect.element(message(/^Message (to|from) /)).not.toBeInTheDocument();
});

test("a press on the quote opens the other Job on the stage, and so does a press on the arc", async () => {
  await open();
  timePasses();
  const quote = message(/^Message to Rebuild the store index: /);
  await expect.element(quote).toBeInTheDocument();
  (quote.element() as HTMLElement).click();
  await expect.element(page.getByRole("region", { name: "Job: Rebuild the store index" })).toBeVisible();

  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Rotate the audit log" }).click();
  await expect.element(page.getByRole("region", { name: "Job: Rotate the audit log" })).toBeVisible();
  // The line is no place to stop, so it has no role: it is found by what it is on the map.
  const line = field().element().querySelector<HTMLElement>('[data-hit="message-line"]');
  expect(line?.dataset.key).toBe("session:jm-index");
  line!.click();
  await expect.element(page.getByRole("region", { name: "Job: Rebuild the store index" })).toBeVisible();
});
