// Settings, through `App` on core and its own slice. The mock Fleet answers `get_settings` with
// `MOCK_SETTINGS` (`./fake-settings.ts`), so every category here is drawn from settings.json's schema.

import type { Outcome } from "@armada/protocol";
import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onBoard, openHelm, unmountAfterEach } from "@armada/desktop/mock";

import { MOCK_SETTINGS } from "./fake-settings";
import { openSettingsAt } from "./sections";

unmountAfterEach();

const openSettings = async () => {
  await page.getByRole("button", { name: "Settings", exact: true }).click();
};

test("the index lists groups in order, and a group draws each of its sections under its own heading", async () => {
  mount(onBoard([]), { slices: ["core", "settings"] });
  await openSettings();
  const tabs = page.getByRole("tablist", { name: "Settings" }).getByRole("tab");
  await expect.poll(() => tabs.elements().map((one) => one.textContent)).toEqual([
    "Agents", "Prompts", "Fleet", "Features", "Tools", "Appearance", "Phone", "Guides",
  ]);
  await page.getByRole("tab", { name: "Fleet" }).click();
  for (const section of ["Limits", "Timeouts", "Retention"]) {
    await expect.element(page.getByRole("heading", { name: section, exact: true })).toBeVisible();
  }
  await expect.element(page.getByRole("textbox", { name: "Drones at once", exact: true })).toHaveValue("3");
  // Effort is under Agents, as Fleet groups it.
  await page.getByRole("tab", { name: "Agents" }).click();
  for (const section of ["Harness", "Model", "Effort"]) {
    await expect.element(page.getByRole("heading", { name: section, exact: true })).toBeVisible();
  }
  // The dock is on every surface, Settings included — and it reaches this one over the content.
  await openHelm();
});

test("a saved value is marked Modified, and Reset removes the key so the shipped value is back", async () => {
  const app = mount(onBoard([]), { slices: ["core", "settings"] });
  const save = vi.spyOn(app.api, "saveSettings");
  await openSettings();
  await page.getByRole("tab", { name: "Fleet" }).click();
  await expect.element(page.getByText("Modified").first()).toBeVisible();
  await page.getByRole("button", { name: "Reset: Drones at once" }).click();
  expect(save).toHaveBeenCalledWith({ "limits.dronesAtOnce": null });
  await expect.element(page.getByRole("textbox", { name: "Drones at once", exact: true })).toHaveValue("2");
  await expect.element(page.getByRole("button", { name: "Reset: Drones at once" })).not.toBeInTheDocument();
});

test("a figure is typed, saved by key, and a switch saves as it is pressed", async () => {
  const app = mount(onBoard([]), { slices: ["core", "settings"] });
  const save = vi.spyOn(app.api, "saveSettings");
  await openSettings();
  await page.getByRole("tab", { name: "Fleet" }).click();
  const memory = page.getByRole("textbox", { name: "Memory to keep free", exact: true });
  await memory.clear();
  await memory.fill("20");
  await page.getByRole("button", { name: "Save Memory to keep free" }).click();
  expect(save).toHaveBeenCalledWith({ "limits.memorySparePercent": 20 });
  await page.getByRole("tab", { name: "Features" }).click();
  const draft = page.getByRole("switch", { name: /^Draft pull requests/ });
  await expect.element(draft).not.toBeChecked();
  await page.getByText("Draft pull requests").click();
  await expect.element(draft).toBeChecked();
  expect(save).toHaveBeenCalledWith({ "features.draftPullRequests": true });
});

test("a setting waiting on a restart says so, and one an environment variable wins over says which", async () => {
  mount(onBoard([]), { slices: ["core", "settings"] });
  await openSettings();
  await page.getByRole("tab", { name: "Fleet" }).click();
  await expect.element(page.getByText("Restart Fleet to apply.")).toBeVisible();
  await page.getByRole("tab", { name: "Agents" }).click();
  await expect.element(page.getByText("$ARMADA_AGENT_BINARY")).toBeVisible();
});

test("a prompt is edited in several lines, and Reset to shipped takes the edit back", async () => {
  const app = mount(onBoard([]), { slices: ["core", "settings"] });
  const save = vi.spyOn(app.api, "saveSettings");
  await openSettings();
  await page.getByRole("tab", { name: "Prompts" }).click();
  const baseline = page.getByRole("textbox", { name: "Drone baseline", exact: true });
  await baseline.fill("Stay inside the worktree.\nRun the checks.");
  await page.getByRole("button", { name: "Save Drone baseline" }).click();
  expect(save).toHaveBeenCalledWith({ "prompts.droneBaseline": "Stay inside the worktree.\nRun the checks." });
  await page.getByRole("button", { name: "Reset to shipped: Drone baseline" }).click();
  expect(save).toHaveBeenLastCalledWith({ "prompts.droneBaseline": null });
  await expect.element(page.getByRole("button", { name: "Reset to shipped: Drone baseline" })).not.toBeInTheDocument();
});

test("Settings finds a setting by search, across every category, by its key as well as its title", async () => {
  mount(onBoard([]), { slices: ["core", "settings"] });
  await openSettings();
  await page.getByRole("searchbox", { name: "Search settings" }).fill("proposer");
  await expect.element(page.getByRole("textbox", { name: "Job proposer model", exact: true })).toBeVisible();
  await expect.element(page.getByRole("textbox", { name: "Job proposer", exact: true })).toBeVisible();
  // Matches from several groups share the page, so each section says which group it is in.
  await expect.element(page.getByRole("heading", { name: "Agents › Model" })).toBeVisible();
  await expect.element(page.getByRole("heading", { name: "Theme" })).not.toBeInTheDocument();
  // A category holding no match is not offered while the search is on.
  await expect.element(page.getByRole("tab", { name: /^Layout/ })).not.toBeInTheDocument();
  await page.getByRole("searchbox", { name: "Search settings" }).fill("timeouts.checkSeconds");
  await expect.element(page.getByRole("textbox", { name: "Check time limit", exact: true })).toBeVisible();
});

test("a refused settings.json says which key and why, and that the last good settings stay in force", async () => {
  const scenario = onBoard([]);
  const refused = { ...MOCK_SETTINGS, refused: { key: "limits.dronesAtOnce", reason: "12 is above the most this takes, 8." } };
  mount({ ...scenario, state: { ...scenario.state, settings: refused } }, { slices: ["core", "settings"] });
  await openSettings();
  await expect.element(page.getByText("Fleet refused settings.json")).toBeVisible();
  await expect.element(page.getByText(/12 is above the most this takes, 8\. The last settings that read cleanly stay in force/)).toBeVisible();
});

/** `fleet.unacceptable_settings`, the 422 a save Fleet will not take answers with. */
const unacceptable = (key: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code: "fleet.unacceptable_settings", message, run_id: "run_settings", fields: { key }, chain: [message] },
});

test("a save Fleet refuses is drawn against the setting its key field names, in Fleet's words", async () => {
  const app = mount(onBoard([]), { slices: ["core", "settings"] });
  vi.spyOn(app.api, "saveSettings").mockResolvedValueOnce(unacceptable("limits.diskFloorGib", "Disk to keep free is below the 20 GiB the slot builds need."));
  await openSettings();
  await page.getByRole("tab", { name: "Fleet" }).click();
  const memory = page.getByRole("textbox", { name: "Memory to keep free", exact: true });
  await memory.clear();
  await memory.fill("20");
  await page.getByRole("button", { name: "Save Memory to keep free" }).click();
  await expect.element(page.getByText("Disk to keep free is below the 20 GiB the slot builds need.")).toBeVisible();
  await expect.element(page.getByRole("textbox", { name: "Disk to keep free", exact: true })).toBeInvalid();
  await expect.element(memory).not.toBeInvalid();
});

test("Open settings.json asks main to open the file, and names no path", async () => {
  const app = mount(onBoard([]), { slices: ["core", "settings"] });
  const open = vi.spyOn(app.api, "openSettingsFile");
  await openSettings();
  await expect.element(page.getByText(MOCK_SETTINGS.path)).toBeVisible();
  await page.getByRole("button", { name: "Open settings.json" }).click();
  expect(open).toHaveBeenCalledWith();
});

test("before Fleet has answered, each section of a group says its settings are not here", async () => {
  const scenario = onBoard([]);
  mount({ ...scenario, state: { ...scenario.state, connection: { state: "not_running", absence: { why: "no_runtime_file", path: "/tmp/armada/fleet.json" } } } }, { slices: ["core", "settings"] });
  await openSettings();
  await page.getByRole("tab", { name: "Fleet" }).click();
  await expect.element(page.getByText("Fleet has not answered yet, so these settings are not here.")).toHaveLength(3);
});

test("a palette row opens its section's group", async () => {
  mount(onBoard([]), { slices: ["core", "settings"] });
  openSettingsAt("settings:terminal", () => {});
  await openSettings();
  await expect.element(page.getByRole("tab", { name: "Tools" })).toHaveAttribute("aria-selected", "true");
  await expect.element(page.getByRole("heading", { name: "Terminal", exact: true })).toBeVisible();
  openSettingsAt("fleet_settings", () => {});
  await expect.element(page.getByRole("tab", { name: "Fleet" })).toHaveAttribute("aria-selected", "true");
  await expect.element(page.getByRole("heading", { name: "Limits", exact: true })).toBeVisible();
});

test("Settings → Keyboard rebinds a key, and the new key is the one that acts", async () => {
  mount(onBoard([]), { slices: ["core", "settings"] });
  await openSettings();
  await page.getByRole("tab", { name: "Appearance" }).click();
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
