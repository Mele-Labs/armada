// Autopilot's switch and setup, held as the owner approved them in the walks `autopilot-arm` and `autopilot-why`: the switch reads Fleet's
// record, starting is two acts, Start sends what was checked and stops a paused one first, a refusal leaves the switch off, and a cap or a
// fault is said where there is something to say.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { Outcome } from "@armada/protocol";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const hardware = () => page.getByRole("region", { name: "Autopilot switch" });
const setup = () => page.getByRole("region", { name: "Autopilot setup" });
const lever = () => hardware().getByRole("switch", { name: "Autopilot" });
const isOn = () => lever().element().getAttribute("aria-checked") === "true";

async function ready(scenario: string) {
  const app = mount(scenario);
  await onScreen();
  await expect.element(lever()).toBeInTheDocument();
  return app;
}

const refusal = (message: string): { ok: false; outcome: Outcome } => ({
  ok: false,
  outcome: { ok: false, why: "refused", error: { code: "fleet.autopilot_running", message, run_id: "", fields: {}, chain: [] } },
});

test("the switch is on where an Autopilot is running, after a reload as well, and closing the guard stops it", async () => {
  const first = await ready("autopilot-back");
  await expect.poll(isOn).toBe(true);
  first.unmount();
  document.getElementById("root")?.remove();

  // A reload asks Fleet again and finds it still running.
  const app = await ready("autopilot-back");
  await expect.poll(isOn).toBe(true);
  const stop = vi.spyOn(app.api, "stopAutopilot");
  await hardware().getByRole("button", { name: "Close guard" }).click();
  await expect.poll(() => stop.mock.calls.length).toBe(1);
  await expect.poll(isOn).toBe(false);
});

test("starting is two acts: the guard lifts, then the switch flips", async () => {
  const app = await ready("cockpit-ship-flightdeck");
  const start = vi.spyOn(app.api, "startAutopilot");
  expect(isOn()).toBe(false);

  // With the guard down the switch does nothing.
  lever().element().click();
  expect(start).not.toHaveBeenCalled();

  await hardware().getByRole("button", { name: "Lift guard" }).click();
  await setup().getByRole("checkbox", { name: "armada" }).click();
  await lever().click();
  await expect.poll(() => start.mock.calls.length).toBe(1);
  await expect.poll(isOn).toBe(true);
});

test("Start sends the repository, the limits and the boxes that were checked", async () => {
  const app = await ready("cockpit-ship-flightdeck");
  const start = vi.spyOn(app.api, "startAutopilot");
  await hardware().getByRole("button", { name: "Lift guard" }).click();
  await setup().getByRole("checkbox", { name: "armada" }).click();
  await setup().getByRole("checkbox", { name: "Jobs already running" }).click();
  await setup().getByRole("checkbox", { name: "Fix a red main" }).click();
  await setup().getByRole("checkbox", { name: "Writes" }).click();
  await setup().getByRole("textbox", { name: "Instruction" }).fill("Fix the flaky test");
  await setup().getByRole("textbox", { name: "Jobs at a time" }).fill("3");
  await setup().getByRole("button", { name: "Start Autopilot" }).click();

  await expect.poll(() => start.mock.calls.length).toBe(1);
  const [manifest, settings] = start.mock.calls[0]!;
  expect(manifest).toEqual(expect.any(String));
  expect(settings).toMatchObject({ instruction: "Fix the flaky test", at_once: 3, on_autopilot: { backlog: true, running: false } });
  // The defaults checked were recommended, plan, draft, land, red and reads: red is off, and writes is on.
  expect([...settings.decisions!].sort()).toEqual(["draft", "land", "plan", "recommended", "reads", "writes"].sort());
});

test("a paused Autopilot is stopped first, in the same press, and the line says which cap stopped it", async () => {
  const app = await ready("autopilot-why");
  const stop = vi.spyOn(app.api, "stopAutopilot");
  const start = vi.spyOn(app.api, "startAutopilot");
  expect(isOn()).toBe(false);

  await hardware().getByRole("button", { name: "Lift guard" }).click();
  await expect.element(setup().getByRole("list", { name: "Why" }).getByText("Job cap reached")).toBeVisible();
  await setup().getByRole("button", { name: "Start Autopilot" }).click();

  await expect.poll(() => start.mock.calls.length).toBe(1);
  expect(stop).toHaveBeenCalledTimes(1);
  expect(stop.mock.invocationCallOrder[0]).toBeLessThan(start.mock.invocationCallOrder[0]!);
  await expect.poll(isOn).toBe(true);
  // Working, it has nothing to say.
  await expect.element(setup().getByRole("button", { name: "Stop Autopilot" })).toBeVisible();
  expect(document.querySelector(".ap-hud__why")).toBeNull();
});

test("a refused start says what the refusal said, and the switch stays off", async () => {
  const app = await ready("cockpit-ship-flightdeck");
  vi.spyOn(app.api, "startAutopilot").mockResolvedValue(refusal("A merge is held up on a red main"));
  await hardware().getByRole("button", { name: "Lift guard" }).click();
  await setup().getByRole("checkbox", { name: "armada" }).click();
  await setup().getByRole("button", { name: "Start Autopilot" }).click();

  await expect.element(setup().getByRole("alert")).toHaveTextContent("A merge is held up on a red main");
  expect(isOn()).toBe(false);
});
