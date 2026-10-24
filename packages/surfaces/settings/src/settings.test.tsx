// Settings as the System sidebar draws them, through `App` on core and its own slice. The mock Fleet answers
// `get_settings` with `MOCK_SETTINGS` (`./fake-settings.ts`), so every entry here is drawn from settings.json's schema.
// An entry is one press in the sidebar; `toSystemPage` takes its name.

import type { Outcome } from "@armada/protocol";
import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onBoard, onScreen, openHelm, timePasses, toPageByName, toPlace, toSystemPage, unmountAfterEach } from "@armada/desktop/mock";

import { MOCK_SETTINGS } from "./fake-settings";

unmountAfterEach();

const sidebar = () => page.getByRole("navigation", { name: "System" });
// An entry is a press in the group's list; a press beside its caption, Kit's Add, is an act and not an entry.
const entriesOf = (group: string) => sidebar().getByRole("group", { name: group }).getByRole("listitem").getByRole("button");
const heading = (name: string) => page.getByRole("heading", { name, exact: true });

test("the sidebar lists Kit, Fleet, Mods and Bridge entries in order, and an entry draws its sections under their own headings", async () => {
  mount(onBoard(), { slices: ["core", "settings"] });
  await toPlace("System");
  const labels = (group: string) => () => entriesOf(group).elements().map((one) => one.textContent);
  await expect.poll(labels("Kit")).toEqual([
    "Skills", "Plugins", "Sub agents", "Commands", "Servers", "Instructions", "Output styles", "Allowlist", "Mods", "Workflows", "Models", "Prompts", "Harness",
  ]);
  await expect.poll(labels("Fleet")).toEqual(["Limits", "Timeouts", "Retention", "Plan", "Machine", "Account", "Store", "Features"]);
  // Mods lists Words and Fleet only once a teaching has made a mod of that kind.
  await expect.poll(labels("Mods")).toEqual(["All", "Theme", "Layout"]);
  await expect.poll(labels("Bridge")).toEqual(["Keyboard shortcuts", "Time", "Editor", "Terminal", "Links", "Phone", "Guides"]);
  // Each Fleet entry draws its one section under its own heading.
  for (const name of ["Limits", "Timeouts", "Retention", "Plan", "Machine", "Account", "Store", "Features"]) {
    await toSystemPage(name);
    await expect.element(heading(name)).toBeVisible();
  }
  await toSystemPage("Limits");
  await expect.element(page.getByRole("textbox", { name: "Drones at once", exact: true })).toHaveValue("3");
  // Kit holds the settings that choose what a Drone runs: Harness, and Prompts with its baseline.
  await toSystemPage("Harness");
  await expect.element(heading("Harness")).toBeVisible();
  await toSystemPage("Prompts");
  await expect.element(page.getByRole("textbox", { name: "Drone baseline", exact: true })).toBeVisible();
  // The dock is on every surface, System included, and it reaches this one over the content.
  await openHelm();
});

test("a saved value is marked Modified, and Reset removes the key so the shipped value is back", async () => {
  const app = mount(onBoard(), { slices: ["core", "settings"] });
  const save = vi.spyOn(app.api, "saveSettings");
  await toSystemPage("Limits");
  await expect.element(page.getByText("Modified").first()).toBeVisible();
  await page.getByRole("button", { name: "Reset: Drones at once" }).click();
  expect(save).toHaveBeenCalledWith({ "limits.dronesAtOnce": null });
  await expect.element(page.getByRole("textbox", { name: "Drones at once", exact: true })).toHaveValue("2");
  await expect.element(page.getByRole("button", { name: "Reset: Drones at once" })).not.toBeInTheDocument();
});

test("a figure is typed, saved by key, and a switch saves as it is pressed", async () => {
  const app = mount(onBoard(), { slices: ["core", "settings"] });
  const save = vi.spyOn(app.api, "saveSettings");
  await toSystemPage("Limits");
  const memory = page.getByRole("textbox", { name: "Memory to keep free", exact: true });
  await memory.clear();
  await memory.fill("20");
  await page.getByRole("button", { name: "Save Memory to keep free" }).click();
  expect(save).toHaveBeenCalledWith({ "limits.memorySparePercent": 20 });
  await toSystemPage("Features");
  const draft = page.getByRole("switch", { name: /^Draft pull requests/ });
  await expect.element(draft).not.toBeChecked();
  await page.getByText("Draft pull requests").click();
  await expect.element(draft).toBeChecked();
  expect(save).toHaveBeenCalledWith({ "features.draftPullRequests": true });
});

test("a setting waiting on a restart says so, and one an environment variable wins over says which", async () => {
  mount(onBoard(), { slices: ["core", "settings"] });
  await toSystemPage("Timeouts");
  await expect.element(page.getByText("Restart Fleet to apply.")).toBeVisible();
  await toSystemPage("Harness");
  await expect.element(page.getByText("$ARMADA_AGENT_BINARY")).toBeVisible();
});

test("a prompt is edited in several lines, and Reset to shipped takes the edit back", async () => {
  const app = mount(onBoard(), { slices: ["core", "settings"] });
  const save = vi.spyOn(app.api, "saveSettings");
  await toSystemPage("Prompts");
  const baseline = page.getByRole("textbox", { name: "Drone baseline", exact: true });
  await baseline.fill("Stay inside the worktree.\nRun the checks.");
  await page.getByRole("button", { name: "Save Drone baseline" }).click();
  expect(save).toHaveBeenCalledWith({ "prompts.droneBaseline": "Stay inside the worktree.\nRun the checks." });
  await page.getByRole("button", { name: "Reset to shipped: Drone baseline" }).click();
  expect(save).toHaveBeenLastCalledWith({ "prompts.droneBaseline": null });
  await expect.element(page.getByRole("button", { name: "Reset to shipped: Drone baseline" })).not.toBeInTheDocument();
});

test("the sidebar search finds a setting across every entry, by its key as well as its title, and offers no entry without a match", async () => {
  mount(onBoard(), { slices: ["core", "settings"] });
  await toPlace("System");
  const search = page.getByRole("searchbox", { name: "Search System" });
  await search.fill("proposer");
  const offered = () => sidebar().getByRole("button").elements().map((one) => one.textContent);
  await expect.poll(offered).toEqual(["Prompts"]);
  await expect.element(page.getByRole("textbox", { name: "Job proposer", exact: true })).toBeVisible();
  await expect.element(heading("Theme")).not.toBeInTheDocument();
  await search.fill("timeouts.checkSeconds");
  await expect.poll(offered).toEqual(["Timeouts"]);
  await expect.element(page.getByRole("textbox", { name: "Check time limit", exact: true })).toBeVisible();
  // A key in another group is found too, and the entry before it is no longer drawn.
  await search.fill("features.draftPullRequests");
  await expect.poll(offered).toEqual(["Features"]);
  await expect.element(page.getByRole("switch", { name: /^Draft pull requests/ })).toBeVisible();
  await expect.element(page.getByRole("textbox", { name: "Check time limit", exact: true })).not.toBeInTheDocument();
});

test("a refused settings.json says which key and why, and that the last good settings stay in force", async () => {
  const scenario = onBoard();
  const refused = { ...MOCK_SETTINGS, refused: { key: "limits.dronesAtOnce", reason: "12 is above the most this takes, 8." } };
  mount({ ...scenario, state: { ...scenario.state, settings: refused } }, { slices: ["core", "settings"] });
  await toSystemPage("Limits");
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
  const app = mount(onBoard(), { slices: ["core", "settings"] });
  vi.spyOn(app.api, "saveSettings").mockResolvedValueOnce(unacceptable("limits.diskFloorGib", "Disk to keep free is below the 20 GiB the slot builds need."));
  await toSystemPage("Limits");
  const memory = page.getByRole("textbox", { name: "Memory to keep free", exact: true });
  await memory.clear();
  await memory.fill("20");
  await page.getByRole("button", { name: "Save Memory to keep free" }).click();
  await expect.element(page.getByText("Disk to keep free is below the 20 GiB the slot builds need.")).toBeVisible();
  await expect.element(page.getByRole("textbox", { name: "Disk to keep free", exact: true })).toBeInvalid();
  await expect.element(memory).not.toBeInvalid();
});

test("Open settings.json opens the file in the side panel, at the path Fleet named", async () => {
  const app = mount(onBoard(), { slices: ["core", "settings", "files"] });
  const read = vi.spyOn(app.api, "readPanelFile");
  const open = vi.spyOn(app.api, "openSettingsFile");
  await toSystemPage("Limits");
  await expect.element(page.getByText(MOCK_SETTINGS.path)).toBeVisible();
  await page.getByRole("button", { name: "Open settings.json" }).click();
  await expect.element(page.getByRole("dialog", { name: "settings.json" })).toBeVisible();
  expect(read).toHaveBeenCalledWith({ kind: "file", path: MOCK_SETTINGS.path, from: "settings" });
  expect(open).not.toHaveBeenCalled();
});

test("before Fleet has answered, each Fleet entry says its settings are not here", async () => {
  const scenario = onBoard();
  mount({ ...scenario, state: { ...scenario.state, connection: { state: "not_running", absence: { why: "no_runtime_file", path: "/tmp/armada/fleet.json" } } } }, { slices: ["core", "settings"] });
  for (const name of ["Limits", "Timeouts", "Retention", "Plan", "Machine", "Account", "Store", "Features"]) {
    await toSystemPage(name);
    await expect.element(page.getByText("Fleet has not answered yet, so these settings are not here.")).toBeVisible();
  }
});

test("a palette row opens its section's entry", async () => {
  mount(onBoard(), { slices: ["core", "settings"] });
  await onScreen();
  await toPageByName("Terminal");
  await expect.element(sidebar().getByRole("button", { name: "Terminal", exact: true })).toHaveAttribute("aria-current", "page");
  await expect.element(heading("Terminal")).toBeVisible();
  await toPageByName("Fleet limits");
  await expect.element(sidebar().getByRole("button", { name: "Limits", exact: true })).toHaveAttribute("aria-current", "page");
  await expect.element(heading("Limits")).toBeVisible();
});

test("the Choose act of the plan's offer lands on Kit > Models, on the select that sets the Job model", async () => {
  mount("plan-downshift");
  await onScreen();
  timePasses();
  const weekly = page.getByRole("button", { name: /Weekly limit \d+%$/ });
  await weekly.click();
  await page.getByRole("dialog", { name: "Plan limits" }).getByRole("button", { name: "Choose…" }).click();
  await expect.element(sidebar().getByRole("button", { name: "Models", exact: true })).toHaveAttribute("aria-current", "page");
  await expect.element(page.getByRole("combobox", { name: "Jobs", exact: true })).toHaveValue("opus");
  // The two settings the offer is made from are the Plan entry's.
  await toSystemPage("Plan");
  await expect.element(page.getByRole("textbox", { name: "Offer a lower default at", exact: true })).toHaveValue("80");
  await expect.element(page.getByRole("textbox", { name: "Lower default model", exact: true })).toHaveValue("sonnet");
});

test("Settings > Keyboard rebinds a key, and the new key is the one that acts", async () => {
  mount(onBoard(), { slices: ["core", "settings"] });
  await toSystemPage("Keyboard shortcuts");
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

test("Terminal lists where a Job's terminal opens, docked by default, the side panel or a window, and saves the choice by key", async () => {
  const app = mount(onBoard(), { slices: ["core", "settings"] });
  const save = vi.spyOn(app.api, "saveSettings");
  await toSystemPage("Terminal");
  const place = page.getByRole("combobox", { name: "Open a Job's terminal in", exact: true });
  await expect.element(place).toHaveValue("docked");
  expect([...(place.element() as HTMLSelectElement).options].map((one) => one.text)).toEqual(["docked", "side panel", "window"]);
  await place.selectOptions("window");
  expect(save).toHaveBeenCalledWith({ "terminal.openJobTerminalIn": "window" });
});

test("Large context sits under Plan with no sentence, names Context limit in its tooltip, and saves its choice by key", async () => {
  const app = mount(onBoard(), { slices: ["core", "settings"] });
  const save = vi.spyOn(app.api, "saveSettings");
  await toSystemPage("Plan");
  const large = page.getByRole("combobox", { name: "Large context", exact: true });
  await expect.element(large).toHaveValue("ask");
  // The wire's `tip` is all the row says of itself: no sentence under the title, and the title is not said twice.
  await expect.element(large).toHaveAccessibleDescription("Context limit");
  await large.hover();
  await expect.element(page.getByText("Context limit", { exact: true })).toBeVisible();
  await large.selectOptions("carry on automatically");
  expect(save).toHaveBeenCalledWith({ "plan.contextHandoff": "carry on automatically" });
});
