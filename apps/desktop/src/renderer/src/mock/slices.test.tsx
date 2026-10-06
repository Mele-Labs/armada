// A window mounted with core and one surface's fake, through `App`: that surface draws, and every
// other surface's calls answer `unanswered` — so a surface's test needs no other surface's fleet.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { SLICES } from "./slices";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Studios draws on core and its own slice, and a call of another slice is unanswered", async () => {
  const { api } = mount("every-state", { slices: ["studios"] });
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await userEvent.selectOptions(page.getByLabelText("Repository", { exact: true }), repository().root);
  await expect.element(page.getByRole("cell", { name: "Every kind of node and edge", exact: true })).toBeVisible();

  const asked = await api.readWorkflows();
  expect(asked).toMatchObject({ ok: false, outcome: { ok: false, why: "transport" } });
  // A slice left out holds its state empty, and one kept keeps the scenario's.
  expect((await api.state()).watched).toEqual({ state: "none" });
  expect((await api.state()).jobs.length).toBeGreaterThan(0);
});

test("every slice but Studios answers a call it owns as unanswered", async () => {
  const { api } = mount("every-state", { slices: ["studios"] });
  expect(await api.killJob("none")).toMatchObject({ ok: false, why: "transport" });
  expect(await api.readHeld(true)).toMatchObject({ ok: false });
  expect(SLICES.map((one) => one.name)).toHaveLength(11);
});

test("no slice list mounts every slice, as before", async () => {
  const { api } = mount("every-state");
  expect(await api.readWorkflows()).toMatchObject({ ok: true });
});
