// An added step's two switches stay live until it fires: the panel sends the change to Fleet, and a
// step that has fired reads them. Self repair is not drawn for a Skill or a Drone step.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { AddedStep } from "@armada/protocol";
import { mount, unmount } from "@armada/screens/src/mounted";

import { AddedSheets } from "./added-steps";
import type { AddedBinding, AddedSteps } from "./added-steps";

afterEach(() => unmount());

const step = (state: AddedStep["state"], runs: AddedStep["runs"] = { kind: "script", command: "fmt" }): AddedStep => ({
  id: "a1",
  runs,
  when: "step_passes",
  step: "verify",
  block: false,
  repair: false,
  placed: "running",
  added_at: "2026-10-07T10:00:00.000Z",
  state,
});

function panel(one: AddedStep, onSwitch = vi.fn().mockResolvedValue(undefined)) {
  const added = {
    jobId: "job-1",
    workflow: "feature",
    steps: ["plan", "verify"],
    rows: [one],
    gated: false,
    open: one.id,
    onOpen: vi.fn(),
    composing: null,
    onComposing: vi.fn(),
    keeping: null,
    onKeeping: vi.fn(),
    onRemove: vi.fn(),
    onSwitch,
    binding: { commands: ["fmt"], triggers: [], onAdd: vi.fn(), onRemove: vi.fn(), onEdit: vi.fn(), onKeep: vi.fn() } as AddedBinding,
  } as unknown as AddedSteps;
  mount(<AddedSheets added={added} />);
  return onSwitch;
}

test("a step that has not fired sends a changed switch to Fleet", async () => {
  const onSwitch = panel(step("pending"));
  await page.getByText("Block the Job").click();
  expect(onSwitch).toHaveBeenLastCalledWith(expect.objectContaining({ id: "a1" }), { block: true });
  await page.getByText("Self repair").click();
  expect(onSwitch).toHaveBeenLastCalledWith(expect.objectContaining({ id: "a1" }), { repair: true });
});

test("a step that has fired reads its switches", async () => {
  panel(step("passed"));
  await expect.element(page.getByRole("switch", { name: "Block the Job" })).toBeDisabled();
  await expect.element(page.getByRole("switch", { name: "Self repair" })).toBeDisabled();
});

test("Self repair is not drawn for a Drone step, and Block the Job still is", async () => {
  panel(step("pending", { kind: "drone", brief: "Add a line" }));
  await expect.element(page.getByRole("switch", { name: "Block the Job" })).toBeEnabled();
  expect(page.getByRole("switch", { name: "Self repair" }).elements()).toHaveLength(0);
});
