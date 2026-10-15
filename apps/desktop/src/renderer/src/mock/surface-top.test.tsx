// Where a surface's content starts under the title bar, through `App`.
//
// The left column and Helm's dock each keep `--space-4` between the title bar
// and their first panel. A surface mounted between them keeps the same gap, so
// the three read as one row of panels. The Manifest surface's own ask on All
// repositories sat flush against the title bar instead — the owner's
// annotation 20260917-165852-t9da — because the mount kept no gap and only
// Overview padded its own.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const top = (element: Element): number => element.getBoundingClientRect().top;

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("Manifest's ask on All repositories starts level with the left column, not against the title bar", async () => {
  mount("job/running");
  // The control beside the picker, in the rail's repository panel, is how the
  // surface is reached (#1595).
  await page.getByRole("button", { name: "Open the Manifest", exact: true }).click();

  const ask = page.getByRole("status").filter({ hasText: "Pick a repository to open its Manifest" });
  await expect.element(ask).toBeVisible();
  const first = page.getByRole("region", { name: "Repository" });
  await expect.element(first).toBeVisible();

  // Level with the rail's first panel, which the title bar does not touch.
  expect(top(ask.element())).toBe(top(first.element()));
});
