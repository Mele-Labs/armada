// Settings, through `App` on core and its own slice. Moved from `overview.test.tsx`, which drew it
// beside the Overview.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
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
  await page.getByRole("tab", { name: "Fleet" }).click();
  await expect.element(page.getByRole("heading", { name: "Fleet" })).toBeVisible();
  await page.getByRole("tab", { name: "This machine" }).click();
  await expect.element(page.getByRole("heading", { name: "This machine" })).toBeVisible();
  // The dock is on every surface, Settings included — and it reaches this one
  // over the content rather than out of it.
  await openHelm();
});

test("Settings offers a draft pull request as this machine's default, and keeps it once Fleet has it", async () => {
  const scenario = onBoard([]);
  mount({ ...scenario, state: { ...scenario.state, limits: LIMITS } }, { slices: ["core", "settings"] });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("tab", { name: "This machine" }).click();
  const draft = page.getByRole("switch", { name: "Draft pull requests" });
  await expect.element(draft).not.toBeChecked();
  await page.getByText("Draft pull requests").click();
  await expect.element(draft).toBeChecked();
});

test("Settings finds a setting by search, across every category", async () => {
  const scenario = onBoard([]);
  mount({ ...scenario, state: { ...scenario.state, limits: LIMITS } }, { slices: ["core", "settings"] });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("searchbox", { name: "Search settings" }).fill("draft");
  await expect.element(page.getByRole("switch", { name: "Draft pull requests" })).toBeVisible();
  await expect.element(page.getByRole("heading", { name: "Theme" })).not.toBeInTheDocument();
  // A category holding no match is not offered while the search is on.
  await expect.element(page.getByRole("tab", { name: /^Layout/ })).not.toBeInTheDocument();
});

test("Settings → Keyboard rebinds a key, and the new key is the one that acts", async () => {
  const scenario = onBoard([]);
  mount({ ...scenario, state: { ...scenario.state, limits: LIMITS } }, { slices: ["core", "settings"] });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("tab", { name: "Keyboard shortcuts" }).click();
  const helm = page.getByRole("group", { name: "Helm", exact: true });
  await helm.getByRole("button", { name: /^Helm: ⌘J$/ }).click();
  await userEvent.keyboard("{Meta>}{Shift>}h{/Shift}{/Meta}");
  await expect.element(helm.getByRole("button", { name: /^Helm: ⇧⌘H$/ })).toBeVisible();
  // ⌘J no longer opens Helm, and ⇧⌘H does.
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.element(page.getByRole("complementary", { name: "Helm" })).not.toBeInTheDocument();
  await userEvent.keyboard("{Meta>}{Shift>}h{/Shift}{/Meta}");
  await expect.element(page.getByRole("complementary", { name: "Helm" })).toBeVisible();
  await userEvent.keyboard("{Meta>}{Shift>}h{/Shift}{/Meta}");
  // Reset puts the registry's key back.
  await helm.getByRole("button", { name: "Reset Helm" }).click();
  await expect.element(helm.getByRole("button", { name: /^Helm: ⌘J$/ })).toBeVisible();
});
