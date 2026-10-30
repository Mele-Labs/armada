// Workflows switched off, through `App` on the `workflows-off` scenario: the page's switch for Jobs saves `features.workflows`, a switch on a row saves
// `workflows.off` to the machine on All and to the repository with one picked, and a workflow off on the machine says so in a repository's view.
// The scenario has `prototype` off on the machine and `bug` off in armada, which holds its Manifest by `MANIFEST_ID`.

import { MANIFEST_ID } from "@armada/screens/src/fixtures/repository";
import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { workflowsOff } from "./scenarios/workflows-off";
import { mount, onScreen, toSystemPage, unmountAfterEach } from "./testing";

unmountAfterEach();

async function workflows() {
  const app = mount(workflowsOff);
  await onScreen();
  await toSystemPage("Workflows");
  await expect.element(page.getByRole("button", { name: "design_plan", exact: true })).toBeVisible();
  return { app, saved: vi.spyOn(app.api, "saveSettings") };
}

const rowOf = (name: string) => page.getByRole("listitem").filter({ has: page.getByRole("button", { name, exact: true }) });

async function inArmada(): Promise<void> {
  await userEvent.click(page.getByRole("button", { name: "Repository", exact: true }));
  await userEvent.click(page.getByRole("menuitem", { name: /armada$/ }));
  await expect.element(rowOf("bug").getByRole("switch", { name: "Off in armada" })).toBeVisible();
}

test("the switch for Jobs saves features.workflows, off and then on", async () => {
  const { saved } = await workflows();
  const jobs = page.getByRole("switch", { name: "Workflows for Jobs", exact: true });
  await expect.element(jobs).toBeChecked();
  await jobs.click({ force: true });
  expect(saved).toHaveBeenLastCalledWith({ "features.workflows": false });
  await expect.element(jobs).not.toBeChecked();
  await jobs.click({ force: true });
  expect(saved).toHaveBeenLastCalledWith({ "features.workflows": true });
});

test("on All a row's switch is the machine's: it saves the machine's list and leaves each repository's as it was", async () => {
  const { saved } = await workflows();
  await expect.element(rowOf("prototype").getByRole("switch", { name: "Off on this machine" })).toBeVisible();
  await rowOf("design_plan").getByRole("switch", { name: "On on this machine" }).click({ force: true });
  expect(saved).toHaveBeenLastCalledWith({ "workflows.off": { machine: ["prototype", "design_plan"], repositories: { [MANIFEST_ID]: ["bug"] } } });
  await rowOf("prototype").getByRole("switch", { name: "Off on this machine" }).click({ force: true });
  expect(saved).toHaveBeenLastCalledWith({ "workflows.off": { machine: ["design_plan"], repositories: { [MANIFEST_ID]: ["bug"] } } });
});

test("with a repository picked a row's switch saves that repository's list and leaves the machine's as it was", async () => {
  const { saved } = await workflows();
  await inArmada();
  await rowOf("design_plan").getByRole("switch", { name: "On in armada" }).click({ force: true });
  expect(saved).toHaveBeenLastCalledWith({ "workflows.off": { machine: ["prototype"], repositories: { [MANIFEST_ID]: ["bug", "design_plan"] } } });
  await rowOf("bug").getByRole("switch", { name: "Off in armada" }).click({ force: true });
  expect(saved).toHaveBeenLastCalledWith({ "workflows.off": { machine: ["prototype"], repositories: { [MANIFEST_ID]: ["design_plan"] } } });
});

test("a workflow off on the machine reads Off on this machine in a repository's view, in place of a switch", async () => {
  await workflows();
  await inArmada();
  await expect.element(rowOf("prototype").getByText("Off on this machine")).toBeVisible();
  expect(rowOf("prototype").getByRole("switch").elements()).toHaveLength(0);
  await expect.element(rowOf("design_plan").getByRole("switch", { name: "On in armada" })).toBeVisible();
});
