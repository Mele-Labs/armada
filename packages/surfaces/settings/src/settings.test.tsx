// Settings, through `App` on core and its own slice. Moved from `overview.test.tsx`, which drew it
// beside the Overview.

import { expect, test } from "vitest";
import { page } from "vitest/browser";
import type { FleetLimits } from "@armada/protocol";

import { mount, onBoard, openHelm, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

/** Fleet's four limits, as `get_limits` answers them. */
const LIMITS: FleetLimits = {
  concurrency: 2,
  memory_spare_percent: 15,
  disk_floor_gib: 10,
  checks_at_once: 4,
  shipped: { concurrency: 2, memory_spare_percent: 15, disk_floor_gib: 10, checks_at_once: 4 },
};

test("Settings draws Fleet's limits and this machine's settings under Helm's dock", async () => {
  const scenario = onBoard([]);
  mount({ ...scenario, state: { ...scenario.state, limits: LIMITS } }, { slices: ["core", "settings"] });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect.element(page.getByRole("heading", { name: "Fleet" })).toBeVisible();
  await expect.element(page.getByRole("heading", { name: "This machine" })).toBeVisible();
  // The dock is on every surface, Settings included — and it reaches this one
  // over the content rather than out of it.
  await openHelm();
});
