// The Workflows page on a repository in two Workspaces, through `App`: a group of tabs for the default, one for each Workspace that holds the repository and one for the repositories; the warning
// where Platform and Mobile both change a rule; and the buttons that keep either one's rule as the repository's own. The reads and saves are `slices/lifecycles.ts`; the model is `lifecycle-editing`.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, toSystemPage, unmountAfterEach } from "./testing";
import { workspaceTabs } from "./scenarios/workspace-tabs";

unmountAfterEach();

const API = "01M1CNPKTV0018H2M1CXDNBK0C";
const RUNS_IN = () => page.getByRole("group", { name: "Runs in", exact: true });
const LANES = () => page.getByRole("region", { name: "Workflow", exact: true });
const PANEL = page.getByRole("dialog");
const WARNING = () => LANES().getByRole("img", { name: "Platform and Mobile differ", exact: true });

/** The page on api, whose tab is open with Mark ready picked. */
async function onApi(): Promise<ReturnType<typeof mount>> {
  await page.viewport(2000, 1200);
  const app = mount(workspaceTabs);
  await onScreen();
  await toSystemPage("Workflows");
  await expect.element(LANES()).toBeVisible();
  await RUNS_IN().getByRole("tab", { name: "api", exact: true }).click();
  return app;
}

test("the Runs in row has the default, then a tab for Platform and one for Mobile, then api, web and ios", async () => {
  await onApi();
  await expect.element(RUNS_IN().getByRole("tab", { name: "Mobile", exact: true })).toBeVisible();
  expect(
    RUNS_IN()
      .getByRole("tab")
      .elements()
      // A Workspace that changes something carries a mark that names it ("Workspace Platform, changed"), which is not part of the tab's name.
      .map((one) => one.textContent?.replace(/Workspace .*$/, "").trim()),
  ).toEqual(["Default", "Platform", "Mobile", "api", "web", "ios"]);
});

test("api warns on Mark ready, where Platform and Mobile both change it, and says it runs the Default's", async () => {
  await onApi();
  await expect.element(WARNING()).toBeVisible();
  await LANES().getByRole("button", { name: "Mark ready", exact: true }).click();
  await expect.element(PANEL.getByText("Platform and Mobile both change Mark ready. api runs the Default's.")).toBeVisible();
  await expect.element(PANEL.getByRole("button", { name: "Use Platform's", exact: true })).toBeVisible();
  await expect.element(PANEL.getByRole("button", { name: "Use Mobile's", exact: true })).toBeVisible();
});

test("web, which only Platform holds, and the Workspace tabs themselves do not warn", async () => {
  await onApi();
  await expect.element(WARNING()).toBeVisible();
  for (const name of ["web", "Platform", "Mobile"]) {
    await RUNS_IN().getByRole("tab", { name, exact: true }).click();
    await expect.element(RUNS_IN().getByRole("tab", { name, exact: true })).toHaveAttribute("aria-selected", "true");
    expect(WARNING().elements()).toHaveLength(0);
  }
});

test("Use Mobile's keeps Mobile's rule as api's own, and the warning and the line are gone", async () => {
  const app = await onApi();
  const saved = vi.spyOn(app.api, "saveLifecyclePiece");
  await LANES().getByRole("button", { name: "Mark ready", exact: true }).click();
  await PANEL.getByRole("button", { name: "Use Mobile's", exact: true }).click();
  await expect.poll(() => saved.mock.calls.length).toBe(1);
  expect(saved.mock.calls[0]![0]).toMatchObject({ piece: { kind: "rule", rule: { id: "ready", autonomy: "acts", scope: { kind: "within", id: API } } } });
  // The line is gone and api has a way back to the default's: the piece is its own now.
  await expect.element(PANEL.getByRole("button", { name: "Use the default", exact: true })).toBeVisible();
  expect(PANEL.getByText("both change Mark ready", { exact: false }).elements()).toHaveLength(0);
  expect(WARNING().elements()).toHaveLength(0);
  const read = await app.api.listLifecycleRules(API);
  const kept = read.ok ? read.value.rules.filter((one) => one.rule.id === "ready").map((one) => [one.rule.scope, one.rule.autonomy]) : [];
  expect(kept).toContainEqual([{ kind: "within", id: API }, "acts"]);
});

test("Use Platform's keeps Platform's rule as api's own", async () => {
  const app = await onApi();
  const saved = vi.spyOn(app.api, "saveLifecyclePiece");
  await LANES().getByRole("button", { name: "Mark ready", exact: true }).click();
  await PANEL.getByRole("button", { name: "Use Platform's", exact: true }).click();
  await expect.poll(() => saved.mock.calls.length).toBe(1);
  expect(saved.mock.calls[0]![0]).toMatchObject({ piece: { rule: { id: "ready", autonomy: "asks", scope: { kind: "within", id: API } } } });
  expect(WARNING().elements()).toHaveLength(0);
});
