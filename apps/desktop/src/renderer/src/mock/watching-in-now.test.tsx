// The Watching block stays on the Job's spine: a Job with a live watch draws it at the foot of Now, with the watch's row, through `App` on
// `watching`. The walks `watching` and `watchingTidy` play the block's own acts; this holds where it stands.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("a Job with a live watch draws the Watching block inside Now, with the watch's row", async () => {
  mount("watching");
  await onScreen();
  await page.getByRole("region", { name: "Jobs", exact: true }).getByRole("button", { name: "Ship the rate limit to QA", exact: true }).click();
  const now = page.getByRole("region", { name: "Now", exact: true });
  const watching = now.getByRole("region", { name: "Watching", exact: true });
  await expect.element(watching).toBeVisible();
  await expect.element(watching.getByRole("button", { name: "Watch QA health", exact: true })).toBeVisible();
});
