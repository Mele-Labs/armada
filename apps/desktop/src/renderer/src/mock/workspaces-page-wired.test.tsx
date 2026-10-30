// System > Workspaces over the Workspaces and autonomy defaults a Fleet keeps (armada-cwj), through `App`: a new Workspace is saved as the piece it is under an id made from its name, a change in the
// autonomy table is saved as that level's layer and read back, and a save Fleet refuses is said in its words on the page.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, toSystemPage, unmountAfterEach } from "./testing";
import { workspacesAndAutonomy } from "./scenarios/workspaces-and-autonomy";

unmountAfterEach();

const SCRATCH = "01M1CNPKTV0018H2M1CXDNBK0A";
const MOBILE = "01M1CNPKTV0018H2M1CXDNBK0B";

async function workspaces(): Promise<ReturnType<typeof mount>> {
  const app = mount(workspacesAndAutonomy);
  await onScreen();
  await toSystemPage("Workspaces");
  await expect.element(page.getByRole("button", { name: "New Workspace", exact: true })).toBeVisible();
  return app;
}

const levels = () => page.getByRole("region", { name: "Levels", exact: true });
const form = () => page.getByRole("form", { name: "New Workspace", exact: true });

async function created(): Promise<void> {
  await page.getByRole("button", { name: "New Workspace", exact: true }).click();
  await form().getByRole("textbox", { name: "Name", exact: true }).fill("Mobile Apps");
  await form().getByRole("checkbox", { name: "scratch", exact: true }).click();
  await form().getByRole("checkbox", { name: "store-mobile", exact: true }).click();
  await form().getByRole("button", { name: "Create", exact: true }).click();
}

test("a new Workspace is saved under an id made from its name and listed with its repositories", async () => {
  const app = await workspaces();
  const saved = vi.spyOn(app.api, "saveLifecyclePiece");
  await created();
  await expect.poll(() => saved.mock.calls.length).toBe(1);
  expect(saved.mock.calls[0]![0]).toEqual({ piece: { kind: "workspace", workspace: { id: "mobile-apps", name: "Mobile Apps", repositories: [SCRATCH, MOBILE] } } });
  const group = levels().getByRole("group", { name: "Mobile Apps", exact: true });
  await expect.element(group).toBeVisible();
  await expect.element(group.getByRole("button", { name: "store-mobile", exact: true })).toBeVisible();
});

test("a row of the autonomy table is saved as its level's layer, and reset takes it away", async () => {
  const app = await workspaces();
  const saved = vi.spyOn(app.api, "saveLifecyclePiece");
  const deleted = vi.spyOn(app.api, "deleteLifecyclePiece");
  await created();
  await expect.poll(() => saved.mock.calls.length).toBe(1);
  await page.getByRole("radio", { name: "Ticket writes: Off", exact: true }).click({ force: true });
  await expect.poll(() => saved.mock.calls.length).toBe(2);
  expect(saved.mock.calls[1]![0]).toEqual({ piece: { kind: "autonomy", autonomy: { id: "defaults", scope: { kind: "within", id: "mobile-apps" }, defaults: { tracker_write: "off" } } } });
  await page.getByRole("button", { name: "Use Everywhere's: Asks", exact: true }).click();
  await expect.poll(() => deleted.mock.calls.length).toBe(1);
  expect(deleted.mock.calls[0]![0]).toEqual({ kind: "autonomy", id: "defaults", scope: { kind: "within", id: "mobile-apps" } });
  await expect.element(page.getByRole("radio", { name: "Ticket writes: Asks", exact: true })).toBeChecked();
});

test("a save Fleet refuses is said in its words, and the table stays as Fleet holds it", async () => {
  const app = await workspaces();
  const error = { code: "fleet.piece_unfit", message: "its id is also the id of a repository it holds", run_id: "r", fields: {}, chain: [] };
  app.api.saveLifecyclePiece = async () => ({ ok: false, outcome: { ok: false, why: "refused", error } });
  await levels().getByRole("group", { name: "my-store", exact: true }).getByRole("button", { name: "my-store", exact: true }).click();
  await page.getByRole("radio", { name: "Ticket writes: Off", exact: true }).click({ force: true });
  await expect.element(page.getByRole("alert").getByText("its id is also the id of a repository it holds")).toBeVisible();
  await expect.element(page.getByRole("radio", { name: "Ticket writes: Asks", exact: true })).toBeChecked();
});
